use std::error::FromError;
use std::old_io as io;
use std::str;

use rustc_serialize::Encodable;

use {
    BorrowBytes, ByteString, CsvResult, Encoded, Error, RecordTerminator,
    StrAllocating,
};

/// The quoting style to use when writing CSV data.
#[derive(Copy)]
pub enum QuoteStyle {
    /// This puts quotes around every field. Always.
    Always,
    /// This puts quotes around fields only when necessary.
    ///
    /// They are necessary when fields are empty or contain a quote, delimiter
    /// or record terminator.
    ///
    /// This is the default.
    Necessary,
    /// This *never* writes quotes.
    ///
    /// If a field requires quotes, then the writer will report an error.
    Never,
}

/// A CSV writer.
///
/// This writer provides a convenient interface for encoding CSV data. While
/// creating CSV data is much easier than parsing it, having a writer can
/// be convenient because it can handle quoting for you automatically.
/// Moreover, this particular writer supports `Encodable` types, which makes
/// it easy to write your custom types as CSV records.
///
/// All CSV data produced by this writer, with default options, conforms with
/// [RFC 4180](http://tools.ietf.org/html/rfc4180). (If certain options like
/// flexible record lengths are enabled, then compliance with RFC 4180 cannot
/// be guaranteed.)
///
/// One slight deviation is that records with a single empty field are always
/// encoded as `""`. This ensures that the record is not skipped since some
/// CSV parsers will ignore consecutive record terminators (like the one in
/// this crate).
///
/// ### Example
///
/// Here's an example that encodes word pairs and their edit distances:
///
/// ```rust
/// let records = vec![
///     ("sticker", "mortals", 7u),
///     ("bribed", "personae", 7u),
///     ("wobbling", "poncing", 4u),
///     ("interposed", "emmett", 9u),
///     ("chocolate", "refile", 7u),
/// ];
///
/// let mut wtr = csv::Writer::from_memory();
/// for record in records.into_iter() {
///     let result = wtr.encode(record);
///     assert!(result.is_ok());
/// }
/// ```
pub struct Writer<W> {
    buf: io::BufferedWriter<W>,
    delimiter: u8,
    record_terminator: RecordTerminator,
    flexible: bool,
    quote: u8,
    escape: u8,
    double_quote: bool,
    quote_style: QuoteStyle,
    first_len: usize,
}

impl Writer<io::IoResult<io::File>> {
    /// Creates a new `Writer` that writes CSV data to the file path given.
    ///
    /// The file is created if it does not already exist and is truncated
    /// otherwise.
    pub fn from_file(path: &Path) -> Writer<io::IoResult<io::File>> {
        Writer::from_writer(io::File::create(path))
    }
}


impl<W: io::Writer> Writer<W> {
    /// Creates a new CSV writer that writes to the `io::Writer` given.
    ///
    /// Note that the writer is buffered for you automatically.
    pub fn from_writer(w: W) -> Writer<W> {
        Writer::from_buffer(io::BufferedWriter::new(w))
    }

    /// Creates a new CSV writer that writes to the buffer given.
    ///
    /// This lets you specify your own buffered writer (e.g., use a different
    /// capacity). All other constructors wrap the writer given in a buffer
    /// with default capacity.
    pub fn from_buffer(buf: io::BufferedWriter<W>) -> Writer<W> {
        Writer {
            buf: buf,
            delimiter: b',',
            record_terminator: RecordTerminator::Any(b'\n'),
            flexible: false,
            quote: b'"',
            escape: b'\\',
            double_quote: true,
            quote_style: QuoteStyle::Necessary,
            first_len: 0,
        }
    }
}

impl Writer<Vec<u8>> {
    /// Creates a new CSV writer that writes to an in memory buffer. At any
    /// time, `to_string` or `to_bytes` can be called to retrieve the
    /// cumulative CSV data.
    pub fn from_memory() -> Writer<Vec<u8>> {
        Writer::from_writer(Vec::with_capacity(1024 * 64))
    }

    /// Returns the written CSV data as a string.
    pub fn as_string<'r>(&'r mut self) -> &'r str {
        match self.buf.flush() {
            // shouldn't panic with Vec<u8>
            Err(err) => panic!("Error flushing to Vec<u8>: {}", err),
            // This seems suspicious. If the client only writes `String`
            // values, then this can never fail. If the client is writing
            // byte strings, then they should be calling `to_bytes` instead.
            Ok(()) => str::from_utf8(&**self.buf.get_ref()).unwrap(),
        }
    }

    /// Returns the encoded CSV data as raw bytes.
    pub fn as_bytes<'r>(&'r mut self) -> &'r [u8] {
        match self.buf.flush() {
            // shouldn't panic with Vec<u8>
            Err(err) => panic!("Error flushing to Vec<u8>: {}", err),
            Ok(()) => &**self.buf.get_ref(),
        }
    }
}

impl<W: io::Writer> Writer<W> {
    /// Writes a record by encoding any `Encodable` value.
    ///
    /// This is the most convenient way to write CSV data. Most Rust types
    /// map to CSV data in a straight forward way. A vector is just a sequence
    /// of fields. Similarly for a struct. Enumerations of zero or one
    /// arguments are supported too. (Enums with zero arguments encode to their
    /// name, while enums of one argument encode to their constituent value.)
    /// Option types are also supported (`None` encodes to an empty field).
    ///
    /// ### Example
    ///
    /// This example encodes word pairs that may or may not have their
    /// edit distances computed.
    ///
    /// ```rust
    /// extern crate "rustc-serialize" as rustc_serialize;
    /// # extern crate csv;
    /// # fn main() {
    ///
    /// #[derive(RustcEncodable)]
    /// struct Distance {
    ///     name1: &'static str,
    ///     name2: &'static str,
    ///     dist: Option<usize>,
    /// }
    ///
    /// let records = vec![
    ///     Distance { name1: "sticker", name2: "mortals", dist: None },
    ///     Distance { name1: "bribed", name2: "personae", dist: Some(7) },
    /// ];
    ///
    /// let mut wtr = csv::Writer::from_memory();
    /// for record in records.into_iter() {
    ///     let result = wtr.encode(record);
    ///     assert!(result.is_ok());
    /// }
    /// assert_eq!(wtr.as_string(),
    ///            "sticker,mortals,\nbribed,personae,7\n");
    /// # }
    /// ```
    pub fn encode<E>(&mut self, e: E) -> CsvResult<()> where E: Encodable {
        let mut erecord = Encoded::new();
        try!(e.encode(&mut erecord));
        self.write(erecord.unwrap().into_iter())
    }

    /// Writes a record of strings (Unicode or raw bytes).
    ///
    /// This is meant to be the standard method provided by most CSV writers.
    /// That is, it writes a record of strings---no more and no less.
    ///
    /// This method accepts an iterator of *fields* for a single record. Each
    /// field must satisfy `BorrowBytes`, which allows the caller to control
    /// allocation.
    ///
    /// ### Example
    ///
    /// This shows how to write string records.
    ///
    /// ```rust
    /// let records = vec![
    ///     vec!["sticker", "mortals", "7"],
    ///     vec!["bribed", "personae", "7"],
    ///     vec!["wobbling", "poncing", "4"],
    ///     vec!["interposed", "emmett", "9"],
    ///     vec!["chocolate", "refile", "7"],
    /// ];
    ///
    /// let mut wtr = csv::Writer::from_memory();
    /// for record in records.into_iter() {
    ///     let result = wtr.write(record.into_iter());
    ///     assert!(result.is_ok());
    /// }
    /// ```
    ///
    /// This shows how to write records that do not correspond to a valid UTF-8
    /// encoding. (Note the use of Rust's byte string syntax!)
    ///
    /// ```rust
    /// let mut wtr = csv::Writer::from_memory();
    /// let result = wtr.write(vec![b"\xff", b"\x00"].into_iter());
    /// assert!(result.is_ok());
    ///
    /// assert_eq!(wtr.as_bytes(), b"\xff,\x00\n");
    /// ```
    pub fn write<'a, I>(&mut self, r: I) -> CsvResult<()>
            where I: Iterator, <I as Iterator>::Item: BorrowBytes {
        self.write_iter(r.map(|f| Ok(f)))
    }

    /// Writes a record of results. If any of the results resolve to an error,
    /// then writing stops and that error is returned.
    #[doc(hidden)]
    pub fn write_iter<'a, I, F>(&mut self, r: I) -> CsvResult<()>
            where I: Iterator<Item=CsvResult<F>>, F: BorrowBytes {
        let delim = self.delimiter;
        let mut count = 0;
        let mut last_len = 0;
        for field in r {
            if count > 0 {
                try!(self.w_bytes(&[delim]));
            }
            count += 1;
            let field = try!(field);
            last_len = field.borrow_bytes().len();
            try!(self.w_user_bytes(field.borrow_bytes()));
        }
        // This tomfoolery makes sure that a record with a single empty field
        // is encoded as `""`. Otherwise, you end up with a run of consecutive
        // record terminators, which are ignored by some CSV parsers (such
        // as the one in this library).
        if count == 1 && last_len == 0 {
            let q = self.quote;
            try!(self.w_bytes(&[q, q]));
        }
        try!(self.w_lineterm());
        self.set_first_len(count)
    }

    /// Flushes the underlying buffer.
    pub fn flush(&mut self) -> CsvResult<()> {
        self.buf.flush().map_err(FromError::from_error)
    }
}

impl<W: io::Writer> Writer<W> {
    /// The delimiter to use when writing CSV data.
    ///
    /// Since the CSV writer is meant to be mostly encoding agnostic, you must
    /// specify the delimiter as a single ASCII byte. For example, to write
    /// tab-delimited data, you would use `b'\t'`.
    ///
    /// The default value is `b','`.
    pub fn delimiter(mut self, delimiter: u8) -> Writer<W> {
        self.delimiter = delimiter;
        self
    }

    /// Whether to allow flexible length records when writing CSV data.
    ///
    /// When this is set to `true`, records in the CSV data can have different
    /// lengths. By default, this is disabled, which will cause the CSV writer
    /// to return an error if it tries to write a record that has a different
    /// length than other records it has already written.
    pub fn flexible(mut self, yes: bool) -> Writer<W> {
        self.flexible = yes;
        self
    }

    /// Sets the record terminator to use when writing CSV data.
    ///
    /// By default, this is `RecordTerminator::Any(b'\n')`. If you want to
    /// use CRLF (`\r\n`) line endings, then use `RecordTerminator:CRLF`.
    pub fn record_terminator(mut self, term: RecordTerminator) -> Writer<W> {
        self.record_terminator = term;
        self
    }

    /// Set the quoting style to use when writing CSV data.
    ///
    /// By default, this is set to `QuoteStyle::Necessary`, which will only
    /// use quotes when they are necessary to preserve the integrity of data.
    pub fn quote_style(mut self, style: QuoteStyle) -> Writer<W> {
        self.quote_style = style;
        self
    }

    /// Set the quote character to use when writing CSV data.
    ///
    /// Since the CSV parser is meant to be mostly encoding agnostic, you must
    /// specify the quote as a single ASCII byte. For example, to write
    /// single quoted data, you would use `b'\''`.
    ///
    /// The default value is `b'"'`.
    pub fn quote(mut self, quote: u8) -> Writer<W> {
        self.quote = quote;
        self
    }

    /// Set the escape character to use when writing CSV data.
    ///
    /// This is only used when `double_quote` is set to `false`.
    ///
    /// Since the CSV parser is meant to be mostly encoding agnostic, you must
    /// specify the escape as a single ASCII byte.
    ///
    /// The default value is `b'\\'`.
    pub fn escape(mut self, escape: u8) -> Writer<W> {
        self.escape = escape;
        self
    }

    /// Set the quoting escape mechanism.
    ///
    /// When enabled (which is the default), quotes are escaped by doubling
    /// them. e.g., `"` escapes to `""`.
    ///
    /// When disabled, quotes are escaped with the escape character (which
    /// is `\\` by default).
    pub fn double_quote(mut self, yes: bool) -> Writer<W> {
        self.double_quote = yes;
        self
    }
}

impl<W: io::Writer> Writer<W> {
    fn err<S, T>(&self, msg: S) -> CsvResult<T> where S: StrAllocating {
        Err(Error::Encode(msg.into_str()))
    }

    fn w_bytes(&mut self, s: &[u8]) -> CsvResult<()> {
        self.buf.write_all(s).map_err(Error::Io)
    }

    fn w_user_bytes(&mut self, s: &[u8]) -> CsvResult<()> {
        if try!(self.should_quote(s)) {
            let quoted = self.quote_field(s);
            self.w_bytes(&*quoted)
        } else {
            self.w_bytes(s)
        }
    }

    fn w_lineterm(&mut self) -> CsvResult<()> {
        match self.record_terminator {
            RecordTerminator::CRLF => self.w_bytes(b"\r\n"),
            RecordTerminator::Any(b) => self.w_bytes(&[b]),
        }
    }

    fn set_first_len(&mut self, cur_len: usize) -> CsvResult<()> {
        if cur_len == 0 {
            return self.err("Records must have length greater than 0.")
        }
        if !self.flexible {
            if self.first_len == 0 {
                self.first_len = cur_len;
            } else if self.first_len != cur_len {
                return self.err(format!(
                    "Record has length {} but other records have length {}",
                    cur_len, self.first_len))
            }
        }
        Ok(())
    }

    fn should_quote(&self, field: &[u8]) -> CsvResult<bool> {
        let needs = |&:| field.iter().any(|&b| self.byte_needs_quotes(b));
        match self.quote_style {
            QuoteStyle::Always => Ok(true),
            QuoteStyle::Necessary => Ok(needs()),
            QuoteStyle::Never => {
                if !needs() {
                    Ok(false)
                } else {
                    self.err(format!(
                        "Field requires quotes, but quote style \
                         is 'Never': '{}'",
                        String::from_utf8_lossy(field)))
                }
            }
        }
    }

    fn byte_needs_quotes(&self, b: u8) -> bool {
        b == self.delimiter
        || self.record_terminator == b
        || b == self.quote
        // This is a bit hokey. By default, the record terminator is
        // '\n', but we still need to quote '\r' because the reader
        // interprets '\r' as a record terminator by default.
        || b == b'\r' || b == b'\n'
    }

    fn quote_field(&self, mut s: &[u8]) -> ByteString {
        let mut buf = Vec::with_capacity(s.len() + 2);

        buf.push(self.quote);
        loop {
            match s.position_elem(&self.quote) {
                None => {
                    buf.push_all(s);
                    break
                }
                Some(next_quote) => {
                    buf.push_all(&s[..next_quote]);
                    if self.double_quote {
                        buf.push(self.quote);
                        buf.push(self.quote);
                    } else {
                        buf.push(self.escape);
                        buf.push(self.quote);
                    }
                    s = &s[next_quote + 1..];
                }
            }
        }
        buf.push(self.quote);
        ByteString::from_bytes(buf)
    }
}
