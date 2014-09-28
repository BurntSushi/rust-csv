use std::io;
use std::str;

use serialize::Encodable;

use {
    ByteString, CsvResult, Encoded,
    Error, ErrEncode, ErrIo,
};

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
    flexible: bool,
    crlf: bool,
    first_len: uint,
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
    /// extern crate serialize;
    /// # extern crate csv;
    /// # fn main() {
    ///
    /// #[deriving(Encodable)]
    /// struct Distance {
    ///     name1: &'static str,
    ///     name2: &'static str,
    ///     dist: Option<uint>,
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
    /// assert_eq!(wtr.as_string(), "sticker,mortals,\nbribed,personae,7\n");
    /// # }
    /// ```
    pub fn encode<E: Encodable<Encoded, Error>>
                 (&mut self, e: E) -> CsvResult<()> {
        let mut erecord = Encoded::new();
        try!(e.encode(&mut erecord));
        self.write_bytes(erecord.unwrap().into_iter())
    }

    /// Writes a record of Unicode strings.
    ///
    /// This is meant to be the standard method provided by most CSV writers.
    /// That is, it writes a record of strings---no more and no less.
    ///
    /// This method accepts an iterator of *fields* for a single record. Each
    /// field must be a `&str`, which allows the caller to control allocation.
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
    /// Here's a similar example, but with `String` values instead of `&str`.
    ///
    /// ```rust
    /// let records: Vec<Vec<String>> = vec![
    ///     vec!["sticker", "mortals", "7"],
    ///     vec!["bribed", "personae", "7"],
    ///     vec!["wobbling", "poncing", "4"],
    ///     vec!["interposed", "emmett", "9"],
    ///     vec!["chocolate", "refile", "7"],
    /// ].into_iter()
    ///  .map(|r| r.into_iter().map(|f| f.to_string()).collect())
    ///  .collect();
    ///
    /// let mut wtr = csv::Writer::from_memory();
    /// for record in records.into_iter() {
    ///     // It is important to use `iter()` here instead of `into_iter()`.
    ///     // Capturing the strings by reference requires that they live
    ///     // long enough!
    ///     let result = wtr.write(record.iter().map(|f| f.as_slice()));
    ///     assert!(result.is_ok());
    /// }
    /// ```
    ///
    /// If you think this is stupidly inconvenient for such a simple thing, then
    /// you're right. You should just use the `encode` method. Generally, you
    /// should reach for these raw `write` and `write_bytes` methods only when
    /// you need the performance. (i.e., The iterator may let you avoid
    /// allocating intermediate data.)
    pub fn write<'a, I: Iterator<&'a str>>
                (&mut self, r: I) -> CsvResult<()> {
        self.write_bytes(r.map(|r| r.as_bytes()))
    }

    /// Writes a record of *byte strings*.
    ///
    /// This is useful when you need to create CSV data that is not UTF-8
    /// encoded, or more likely, if you are transforming CSV data that you
    /// do not control with an unknown or malformed encoding.
    ///
    /// Note that this writes a *single* record. It accepts an iterator of
    /// *fields* for that record. Each field must satisfy the `Slice` trait.
    /// For example, your iterator can produce `Vec<u8>` or `&[u8]`, which
    /// allows you to avoid allocation if possible.
    ///
    /// ### Example
    ///
    /// This shows how to write records that do not correspond to a valid UTF-8
    /// encoding. (Note the use of Rust's byte string syntax!)
    ///
    /// ```rust
    /// let mut wtr = csv::Writer::from_memory();
    /// let result = wtr.write_bytes(vec![b"\xff", b"\x00"].into_iter());
    /// assert!(result.is_ok());
    ///
    /// assert_eq!(wtr.as_bytes(), b"\xff,\x00\n");
    /// ```
    pub fn write_bytes<S: Slice<u8>, I: Iterator<S>>
                      (&mut self, r: I) -> CsvResult<()> {
        let mut count = 0;
        let delim = self.delimiter;
        for (i, field) in r.enumerate() {
            count += 1;
            if i > 0 {
                try!(self.w_bytes([delim]));
            }
            try!(self.w_user_bytes(field.as_slice()));
        }
        try!(self.w_lineterm());
        self.set_first_len(count)
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
            flexible: false,
            crlf: false,
            first_len: 0,
        }
    }

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

    /// Whether to use CRLF (i.e., `\r\n`) line endings or not.
    ///
    /// By default, this is disabled. LF (`\n`) line endings are used.
    pub fn crlf(mut self, yes: bool) -> Writer<W> {
        self.crlf = yes;
        self
    }

    /// Flushes the underlying buffer.
    pub fn flush(&mut self) -> CsvResult<()> {
        self.buf.flush().map_err(ErrIo)
    }
}

impl Writer<io::MemWriter> {
    /// Creates a new CSV writer that writes to an in memory buffer. At any
    /// time, `to_string` or `to_bytes` can be called to retrieve the
    /// cumulative CSV data.
    pub fn from_memory() -> Writer<io::MemWriter> {
        Writer::from_writer(io::MemWriter::new())
    }

    /// Returns the written CSV data as a string.
    pub fn as_string<'r>(&'r mut self) -> &'r str {
        match self.buf.flush() {
            // shouldn't fail with MemWriter
            Err(err) => fail!("Error flushing to MemWriter: {}", err),
            // This seems suspicious. If the client only writes `String`
            // values, then this can never fail. If the client is writing
            // byte strings, then they should be calling `to_bytes` instead.
            Ok(()) => str::from_utf8(self.buf.get_ref().get_ref()).unwrap(),
        }
    }

    /// Returns the encoded CSV data as raw bytes.
    pub fn as_bytes<'r>(&'r mut self) -> &'r [u8] {
        match self.buf.flush() {
            // shouldn't fail with MemWriter
            Err(err) => fail!("Error flushing to MemWriter: {}", err),
            Ok(()) => self.buf.get_ref().get_ref(),
        }
    }
}

impl<W: io::Writer> Writer<W> {
    fn err<S: StrAllocating>(&self, msg: S) -> CsvResult<()> {
        Err(ErrEncode(msg.into_string()))
    }

    fn w_bytes(&mut self, s: &[u8]) -> CsvResult<()> {
        self.buf.write(s).map_err(ErrIo)
    }

    fn w_user_bytes(&mut self, s: &[u8]) -> CsvResult<()> {
        let delim = self.delimiter;
        let quotable = |&c: &u8| {
            c == delim || c == b'\n' || c == b'\r' || c == b'"'
        };
        if s.iter().any(quotable) {
            self.w_bytes(quote(s)[])
        } else {
            self.w_bytes(s)
        }
    }

    fn w_lineterm(&mut self) -> CsvResult<()> {
        if self.crlf {
            self.w_bytes(b"\r\n")
        } else {
            self.w_bytes(b"\n")
        }
    }

    fn set_first_len(&mut self, cur_len: uint) -> CsvResult<()> {
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
}

fn quote(mut s: &[u8]) -> ByteString {
    let mut buf = Vec::with_capacity(s.len() + 2);

    buf.push(b'"');
    loop {
        match s.position_elem(&b'"') {
            None => {
                buf.push_all(s);
                break
            }
            Some(next_quote) => {
                buf.push_all(s.slice_to(next_quote + 1));
                buf.push(b'"');
                s = s.slice_from(next_quote + 1);
            }
        }
    }
    buf.push(b'"');
    ByteString::from_bytes(buf)
}
