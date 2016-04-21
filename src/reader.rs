use std::fs;
use std::io;
use std::path::Path;
use std::str;

use rustc_serialize::Decodable;

use {
    ByteString, Result, Decoded,
    Error, LocatableError, ParseError,
};

use self::State::*;

const BUF_SIZE: usize = 1024 * 128;

/// A record terminator.
///
/// Ideally, this would just be a `u8` like any other delimiter, but a useful
/// CSV parser must special case CRLF handling. Hence, this enum.
///
/// Generally, you won't need to use this type because `CRLF` is the default,
/// which is by far the most widely used record terminator.
#[derive(Clone, Copy)]
pub enum RecordTerminator {
    /// Parses `\r`, `\n` or `\r\n` as a single record terminator.
    CRLF,
    /// Parses the byte given as a record terminator.
    Any(u8),
}

impl RecordTerminator {
    #[inline]
    fn is_crlf(&self) -> bool {
        match *self {
            RecordTerminator::CRLF => true,
            RecordTerminator::Any(_) => false,
        }
    }
}

impl PartialEq<u8> for RecordTerminator {
    #[inline]
    fn eq(&self, &other: &u8) -> bool {
        match *self {
            RecordTerminator::CRLF => other == b'\r' || other == b'\n',
            RecordTerminator::Any(b) => other == b
        }
    }
}

/// A CSV reader.
///
/// This reader parses CSV data and exposes records via iterators.
///
/// ### Example
///
/// This example shows how to do type-based decoding for each record in the
/// CSV data.
///
/// ```rust
/// let data = "
/// sticker,mortals,7
/// bribed,personae,7
/// wobbling,poncing,4
/// interposed,emmett,9
/// chocolate,refile,7";
///
/// let mut rdr = csv::Reader::from_string(data).has_headers(false);
/// for row in rdr.decode() {
///     let (n1, n2, dist): (String, String, u32) = row.unwrap();
///     println!("{}, {}: {}", n1, n2, dist);
/// }
/// ```
///
/// Here's another example that parses tab-delimited values with records of
/// varying length:
///
/// ```rust
/// let data = "
/// sticker\tmortals\t7
/// bribed\tpersonae\t7
/// wobbling
/// interposed\temmett\t9
/// chocolate\trefile\t7";
///
/// let mut rdr = csv::Reader::from_string(data)
///                           .has_headers(false)
///                           .delimiter(b'\t')
///                           .flexible(true);
/// for row in rdr.records() {
///     let row = row.unwrap();
///     println!("{:?}", row);
/// }
/// ```
#[derive(Clone)]
pub struct Reader<R> {
    rdr: R,
    buf: Vec<u8>,
    bufi: usize,
    fieldbuf: Vec<u8>,
    state: State,
    eof: bool,
    first_row: Vec<ByteString>,
    first_row_done: bool,
    irecord: u64,
    ifield: u64,
    byte_offset: u64,
    delimiter: u8,
    quote: u8,
    escape: Option<u8>,
    double_quote: bool,
    record_term: RecordTerminator,
    flexible: bool,

    // When this is true, the first record is interpreted as a "header" row.
    // This is opaque to the raw iterator, but is used in any iterator that
    // allocates.
    //
    // TODO: This is exposed for use in the `index` sub-module. Is that OK?
    #[doc(hidden)]
    pub has_headers: bool,
    has_seeked: bool,
}

impl<R: io::Read> Reader<R> {
    /// Creates a new CSV reader from an arbitrary `io::Read`.
    ///
    /// The reader is buffered for you automatically.
    pub fn from_reader(rdr: R) -> Reader<R> {
        Reader {
            rdr: rdr,
            buf: vec![0; BUF_SIZE],
            bufi: BUF_SIZE,
            fieldbuf: Vec::with_capacity(1024),
            state: StartRecord,
            eof: false,
            first_row: vec![],
            first_row_done: false,
            irecord: 1,
            ifield: 1,
            byte_offset: 0,
            delimiter: b',',
            quote: b'"',
            escape: None,
            double_quote: true,
            record_term: RecordTerminator::CRLF,
            flexible: false,
            has_headers: true,
            has_seeked: false,
        }
    }
}

impl Reader<fs::File> {
    /// Creates a new CSV reader for the data at the file path given.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Reader<fs::File>> {
        Ok(Reader::from_reader(try!(fs::File::open(path))))
    }
}

impl Reader<io::Cursor<Vec<u8>>> {
    /// Creates a CSV reader for an in memory string buffer.
    pub fn from_string<'a, S>(s: S) -> Reader<io::Cursor<Vec<u8>>>
            where S: Into<String> {
        Reader::from_bytes(s.into().into_bytes())
    }

    /// Creates a CSV reader for an in memory buffer of bytes.
    pub fn from_bytes<'a, V>(bytes: V) -> Reader<io::Cursor<Vec<u8>>>
            where V: Into<Vec<u8>> {
        Reader::from_reader(io::Cursor::new(bytes.into()))
    }
}

impl<R: io::Read> Reader<R> {
    /// Uses type-based decoding to read a single record from CSV data.
    ///
    /// The type that is being decoded into should correspond to *one full
    /// CSV record*. A tuple, struct or `Vec` fit this category. A tuple,
    /// struct or `Vec` should consist of primitive types like integers,
    /// floats, characters and strings which map to single fields. If a field
    /// cannot be decoded into the type requested, an error is returned.
    ///
    /// Enums are also supported in a limited way. Namely, its variants must
    /// have exactly `1` parameter each. Each variant decodes based on its
    /// constituent type and variants are tried in the order that they appear
    /// in their `enum` definition. See below for examples.
    ///
    /// ### Examples
    ///
    /// This example shows how to decode records into a struct. (Note that
    /// currently, the *names* of the struct members are irrelevant.)
    ///
    /// ```rust
    /// extern crate rustc_serialize;
    /// # extern crate csv;
    /// # fn main() {
    ///
    /// #[derive(RustcDecodable)]
    /// struct Pair {
    ///     name1: String,
    ///     name2: String,
    ///     dist: u32,
    /// }
    ///
    /// let mut rdr = csv::Reader::from_string("foo,bar,1\nfoo,baz,2")
    ///                           .has_headers(false);
    /// // Instantiating a specific type when decoding is usually necessary.
    /// let rows = rdr.decode().collect::<csv::Result<Vec<Pair>>>().unwrap();
    ///
    /// assert_eq!(rows[0].dist, 1);
    /// assert_eq!(rows[1].dist, 2);
    /// # }
    /// ```
    ///
    /// We can get a little crazier with custon enum types or `Option` types.
    /// An `Option` type in particular is useful when a column doesn't contain
    /// valid data in every record (whether it be empty or malformed).
    ///
    /// ```rust
    /// extern crate rustc_serialize;
    /// # extern crate csv;
    /// # fn main() {
    ///
    /// #[derive(RustcDecodable, PartialEq, Debug)]
    /// struct MyUint(u32);
    ///
    /// #[derive(RustcDecodable, PartialEq, Debug)]
    /// enum Number { Integer(i64), Float(f64) }
    ///
    /// #[derive(RustcDecodable)]
    /// struct Row {
    ///     name1: String,
    ///     name2: String,
    ///     dist: Option<MyUint>,
    ///     dist2: Number,
    /// }
    ///
    /// let mut rdr = csv::Reader::from_string("foo,bar,1,1\nfoo,baz,,1.5")
    ///                           .has_headers(false);
    /// let rows = rdr.decode().collect::<csv::Result<Vec<Row>>>().unwrap();
    ///
    /// assert_eq!(rows[0].dist, Some(MyUint(1)));
    /// assert_eq!(rows[1].dist, None);
    /// assert_eq!(rows[0].dist2, Number::Integer(1));
    /// assert_eq!(rows[1].dist2, Number::Float(1.5));
    /// # }
    /// ```
    ///
    /// Finally, as a special case, a tuple/struct/`Vec` can be used as the
    /// "tail" of another tuple/struct/`Vec` to capture all remaining fields:
    ///
    /// ```rust
    /// extern crate rustc_serialize;
    /// # extern crate csv;
    /// # fn main() {
    ///
    /// #[derive(RustcDecodable)]
    /// struct Pair {
    ///     name1: String,
    ///     name2: String,
    ///     attrs: Vec<u32>,
    /// }
    ///
    /// let mut rdr = csv::Reader::from_string("a,b,1,2,3,4\ny,z,5,6,7,8")
    ///                           .has_headers(false);
    /// let rows = rdr.decode().collect::<csv::Result<Vec<Pair>>>().unwrap();
    ///
    /// assert_eq!(rows[0].attrs, vec![1,2,3,4]);
    /// assert_eq!(rows[1].attrs, vec![5,6,7,8]);
    /// # }
    /// ```
    ///
    /// If a tuple/struct/`Vec` appears any where other than the "tail" of a
    /// record, then the behavior is undefined. (You'll likely get a runtime
    /// error. I believe this is a limitation of the current decoding machinery
    /// in the `serialize` crate.)
    /// ```
    pub fn decode<'a, D: Decodable>(&'a mut self) -> DecodedRecords<'a, R, D> {
        DecodedRecords {
            p: self.byte_records(),
            _phantom: ::std::marker::PhantomData,
        }
    }

    /// Returns an iterator of records in the CSV data where each field is
    /// a `String`.
    ///
    /// ### Example
    ///
    /// This is your standard CSV interface with no type decoding magic.
    ///
    /// ```rust
    /// let data = "
    /// sticker,mortals,7
    /// bribed,personae,7
    /// wobbling,poncing,4
    /// interposed,emmett,9
    /// chocolate,refile,7";
    ///
    /// let mut rdr = csv::Reader::from_string(data).has_headers(false);
    /// for row in rdr.records() {
    ///     let row = row.unwrap();
    ///     println!("{:?}", row);
    /// }
    /// ```
    pub fn records<'a>(&'a mut self) -> StringRecords<'a, R> {
        StringRecords { p: self.byte_records() }
    }

    /// Returns a *copy* of the first record in the CSV data as strings.
    ///
    /// This method may be called at any time and regardless of whether
    /// `no_headers` is set or not.
    ///
    /// ### Example
    ///
    /// ```rust
    /// let mut rdr = csv::Reader::from_string("a,b,c\n1,2,3");
    ///
    /// let headers1 = rdr.headers().unwrap();
    /// let rows = rdr.records().collect::<csv::Result<Vec<_>>>().unwrap();
    /// let headers2 = rdr.headers().unwrap();
    ///
    /// let s = |s: &'static str| s.to_string();
    /// assert_eq!(headers1, headers2);
    /// assert_eq!(headers1, vec![s("a"), s("b"), s("c")]);
    /// assert_eq!(rows.len(), 1);
    /// assert_eq!(rows[0], vec![s("1"), s("2"), s("3")]);
    /// ```
    ///
    /// Note that if `no_headers` is called on the CSV reader, the rows
    /// returned in this example include the first record:
    ///
    /// ```rust
    /// let mut rdr = csv::Reader::from_string("a,b,c\n1,2,3")
    ///                           .has_headers(false);
    ///
    /// let headers1 = rdr.headers().unwrap();
    /// let rows = rdr.records().collect::<csv::Result<Vec<_>>>().unwrap();
    /// let headers2 = rdr.headers().unwrap();
    ///
    /// let s = |s: &'static str| s.to_string();
    /// assert_eq!(headers1, headers2);
    /// assert_eq!(headers1, vec![s("a"), s("b"), s("c")]);
    ///
    /// // The header rows are now part of the record iterators.
    /// assert_eq!(rows.len(), 2);
    /// assert_eq!(rows[0], headers1);
    /// assert_eq!(rows[1], vec![s("1"), s("2"), s("3")]);
    /// ```
    pub fn headers(&mut self) -> Result<Vec<String>> {
        byte_record_to_utf8(try!(self.byte_headers()))
    }
}

impl<R: io::Read> Reader<R> {
    /// The delimiter to use when reading CSV data.
    ///
    /// Since the CSV reader is meant to be mostly encoding agnostic, you must
    /// specify the delimiter as a single ASCII byte. For example, to read
    /// tab-delimited data, you would use `b'\t'`.
    ///
    /// The default value is `b','`.
    pub fn delimiter(mut self, delimiter: u8) -> Reader<R> {
        self.delimiter = delimiter;
        self
    }

    /// Whether to treat the first row as a special header row.
    ///
    /// By default, the first row is treated as a special header row, which
    /// means it is excluded from iterators returned by the `decode`, `records`
    /// or `byte_records` methods. When `yes` is set to `false`, the first row
    /// is included in those iterators.
    ///
    /// Note that the `headers` method is unaffected by whether this is set.
    pub fn has_headers(mut self, yes: bool) -> Reader<R> {
        self.has_headers = yes;
        self
    }

    /// Whether to allow flexible length records when reading CSV data.
    ///
    /// When this is set to `true`, records in the CSV data can have different
    /// lengths. By default, this is disabled, which will cause the CSV reader
    /// to return an error if it tries to read a record that has a different
    /// length than the first record that it read.
    pub fn flexible(mut self, yes: bool) -> Reader<R> {
        self.flexible = yes;
        self
    }

    /// Set the record terminator to use when reading CSV data.
    ///
    /// In the vast majority of situations, you'll want to use the default
    /// value, `RecordTerminator::CRLF`, which automatically handles `\r`,
    /// `\n` or `\r\n` as record terminators. (Notably, this is a special
    /// case since two characters can correspond to a single terminator token.)
    ///
    /// However, you may use `RecordTerminator::Any` to specify any ASCII
    /// character to use as the record terminator. For example, you could
    /// use `RecordTerminator::Any(b'\n')` to only accept line feeds as
    /// record terminators, or `b'\x1e'` for the ASCII record separator.
    pub fn record_terminator(mut self, term: RecordTerminator) -> Reader<R> {
        self.record_term = term;
        self
    }

    /// Set the quote character to use when reading CSV data.
    ///
    /// Since the CSV reader is meant to be mostly encoding agnostic, you must
    /// specify the quote as a single ASCII byte. For example, to read
    /// single quoted data, you would use `b'\''`.
    ///
    /// The default value is `b'"'`.
    ///
    /// If `quote` is `None`, then no quoting will be used.
    pub fn quote(mut self, quote: u8) -> Reader<R> {
        self.quote = quote;
        self
    }

    /// Set the escape character to use when reading CSV data.
    ///
    /// Since the CSV reader is meant to be mostly encoding agnostic, you must
    /// specify the escape as a single ASCII byte.
    ///
    /// When set to `None` (which is the default), the "doubling" escape
    /// is used for quote character.
    ///
    /// When set to something other than `None`, it is used as the escape
    /// character for quotes. (e.g., `b'\\'`.)
    pub fn escape(mut self, escape: Option<u8>) -> Reader<R> {
        self.escape = escape;
        self
    }

    /// Enable double quote escapes.
    ///
    /// When disabled, doubled quotes are not interpreted as escapes.
    pub fn double_quote(mut self, yes: bool) -> Reader<R> {
        self.double_quote = yes;
        self
    }

    /// A convenience method for reading ASCII delimited text.
    ///
    /// This sets the delimiter and record terminator to the ASCII unit
    /// separator (`\x1f`) and record separator (`\x1e`), respectively.
    ///
    /// Since ASCII delimited text is meant to be unquoted, this also sets
    /// `quote` to `None`.
    pub fn ascii(self) -> Reader<R> {
        self.delimiter(b'\x1f')
            .record_terminator(RecordTerminator::Any(b'\x1e'))
    }
}

/// NextField is the result of parsing a single CSV field.
///
/// This is only useful if you're using the low level `next_bytes` method.
#[derive(Debug)]
pub enum NextField<'a, T: ?Sized + 'a> {
    /// A single CSV field as a borrowed slice of the parser's internal buffer.
    Data(&'a T),

    /// A CSV error found during parsing. When an error is found, it is
    /// first returned. All subsequent calls of `next_bytes` will return
    /// `EndOfCsv`. (EOF is exempt from this. Depending on the state of the
    /// parser, an EOF could trigger `Data`, `EndOfRecord` and `EndOfCsv`,
    /// all in succession.)
    ///
    /// In general, once `EndOfCsv` is returned, no other return value is
    /// possible on subsequent calls.
    Error(Error),

    /// Indicates the end of a record.
    EndOfRecord,

    /// Indicates the end of the CSV data. Once this state is entered, the
    /// parser can never leave it.
    EndOfCsv,
}

impl<'a, T: ?Sized + ::std::fmt::Debug> NextField<'a, T> {
    /// Transform NextField into an iterator result.
    pub fn into_iter_result(self) -> Option<Result<&'a T>> {
        match self {
            NextField::EndOfRecord | NextField::EndOfCsv => None,
            NextField::Error(err) => Some(Err(err)),
            NextField::Data(field) => Some(Ok(field)),
        }
    }

    /// Returns true if and only if the end of CSV data has been reached.
    pub fn is_end(&self) -> bool {
        if let NextField::EndOfCsv = *self { true } else { false }
    }

    /// Returns the underlying field data.
    ///
    /// If `NextField` is an error or an end of record/CSV marker, this will
    /// panic.
    pub fn unwrap(self) -> &'a T {
        match self {
            NextField::Data(field) => field,
            v => panic!("Cannot unwrap '{:?}'", v),
        }
    }
}

/// These are low level methods for dealing with the raw bytes of CSV records.
/// You should only need to use these when you need the performance or if
/// your CSV data isn't UTF-8 encoded.
impl<R: io::Read> Reader<R> {
    /// This is just like `headers`, except fields are `ByteString`s instead
    /// of `String`s.
    pub fn byte_headers(&mut self) -> Result<Vec<ByteString>> {
        if !self.first_row.is_empty() {
            Ok(self.first_row.clone())
        } else {
            let mut headers = vec![];
            loop {
                let field = match self.next_bytes() {
                    NextField::EndOfRecord | NextField::EndOfCsv => break,
                    NextField::Error(err) => return Err(err),
                    NextField::Data(field) => field,
                };
                headers.push(field.to_vec());
            }
            assert!(headers.len() > 0 || self.done());
            Ok(headers)
        }
    }

    /// This is just like `records`, except fields are `ByteString`s instead
    /// of `String`s.
    pub fn byte_records<'a>(&'a mut self) -> ByteRecords<'a, R> {
        let first = self.has_seeked;
        ByteRecords { p: self, first: first, errored: false }
    }

    /// Returns `true` if the CSV parser has reached its final state. When
    /// this method returns `true`, all iterators will always return `None`.
    ///
    /// This is not needed in typical usage since the record iterators will
    /// stop for you when the parser completes. This method is useful when
    /// you're accessing the parser's lowest-level iterator.
    ///
    /// ### Example
    ///
    /// This is the *fastest* way to compute the number of records in CSV data
    /// using this crate. (It is fast because it does not allocate space for
    /// every field.)
    ///
    /// ```rust
    /// let data = "
    /// sticker,mortals,7
    /// bribed,personae,7
    /// wobbling,poncing,4
    /// interposed,emmett,9
    /// chocolate,refile,7";
    ///
    /// let mut rdr = csv::Reader::from_string(data);
    /// let mut count = 0u64;
    /// while !rdr.done() {
    ///     loop {
    ///         // This case analysis is necessary because we only want to
    ///         // increment the count when `EndOfRecord` is seen. (If the
    ///         // CSV data is empty, then it will never be emitted.)
    ///         match rdr.next_bytes() {
    ///             csv::NextField::EndOfCsv => break,
    ///             csv::NextField::EndOfRecord => { count += 1; break; },
    ///             csv::NextField::Error(err) => panic!(err),
    ///             csv::NextField::Data(_) => {}
    ///         }
    ///     }
    /// }
    ///
    /// assert_eq!(count, 5);
    /// ```
    pub fn done(&self) -> bool {
        self.eof
    }

    /// An iterator over fields in the current record.
    ///
    /// This provides low level access to CSV records as raw byte slices.
    /// Namely, no allocation is performed. Unlike other iterators in this
    /// crate, this yields *fields* instead of records. Notably, this cannot
    /// implement the `Iterator` trait safely. As such, it cannot be used with
    /// a `for` loop.
    ///
    /// See the documentation for the `NextField` type on how the iterator
    /// works.
    ///
    /// This iterator always returns all records (i.e., it won't skip the
    /// header row).
    ///
    /// ### Example
    ///
    /// This method is most useful when used in conjunction with the the
    /// `done` method:
    ///
    /// ```rust
    /// let data = "
    /// sticker,mortals,7
    /// bribed,personae,7
    /// wobbling,poncing,4
    /// interposed,emmett,9
    /// chocolate,refile,7";
    ///
    /// let mut rdr = csv::Reader::from_string(data);
    /// while !rdr.done() {
    ///     while let Some(r) = rdr.next_bytes().into_iter_result() {
    ///         print!("{:?} ", r.unwrap());
    ///     }
    ///     println!("");
    /// }
    /// ```
    pub fn next_bytes(&mut self) -> NextField<[u8]> {
        unsafe { self.fieldbuf.set_len(0); }
        loop {
            if let Err(err) = self.fill_buf() {
                return NextField::Error(Error::Io(err));
            }
            if self.buf.len() == 0 {
                self.eof = true;
                if let StartRecord = self.state {
                    return self.next_eoc();
                } else if let EndRecord = self.state {
                    self.state = StartRecord;
                    return self.next_eor();
                } else {
                    self.state = EndRecord;
                    return self.next_data();
                }
            }
            while self.bufi < self.buf.len() {
                let c = self.buf[self.bufi];
                match self.state {
                    StartRecord => {
                        if self.is_record_term(c) {
                            self.bump();
                        } else {
                            self.state = StartField;
                        }
                    }
                    EndRecord => {
                        if self.record_term.is_crlf() && c == b'\n' {
                            self.bump();
                        }
                        self.state = StartRecord;
                        return self.next_eor();
                    }
                    StartField => {
                        self.bump();
                        if c == self.quote {
                            self.state = InQuotedField;
                        } else if c == self.delimiter {
                            return self.next_data();
                        } else if self.is_record_term(c) {
                            self.state = EndRecord;
                            return self.next_data();
                        } else {
                            self.add(c);
                            self.state = InField;
                        }
                    }
                    InField => {
                        self.bump();
                        if c == self.delimiter {
                            self.state = StartField;
                            return self.next_data();
                        } else if self.is_record_term(c) {
                            self.state = EndRecord;
                            return self.next_data();
                        } else {
                            self.add(c);
                        }
                    }
                    InQuotedField => {
                        self.bump();
                        if c == self.quote {
                            self.state = InDoubleEscapedQuote;
                        } else if self.escape == Some(c) {
                            self.state = InEscapedQuote;
                        } else {
                            self.add(c);
                        }
                    }
                    InEscapedQuote => {
                        self.bump();
                        self.add(c);
                        self.state = InQuotedField;
                    }
                    InDoubleEscapedQuote => {
                        self.bump();
                        if self.double_quote && c == self.quote {
                            self.add(c);
                            self.state = InQuotedField;
                        } else if c == self.delimiter {
                            self.state = StartField;
                            return self.next_data();
                        } else if self.is_record_term(c) {
                            self.state = EndRecord;
                            return self.next_data();
                        } else {
                            self.add(c);
                            self.state = InField; // degrade gracefully?
                        }
                    }
                }
            }
        }
    }

    /// This is just like `next_bytes` except it converts each field to
    /// a Unicode string in place.
    pub fn next_str(&mut self) -> NextField<str> {
        // This really grates me. Once we call `next_bytes`, we initiate a
        // *mutable* borrow of `self` that doesn't stop until the return value
        // goes out of scope. Since we have to return that value, it will never
        // go out of scope in this function.
        //
        // Therefore, we can't get access to any state information after
        // calling `next_bytes`. But we might need it to report an error.
        //
        // One possible way around this is to use interior mutability...
        let (record, field) = (self.irecord, self.ifield);
        match self.next_bytes() {
            NextField::EndOfRecord => NextField::EndOfRecord,
            NextField::EndOfCsv => NextField::EndOfCsv,
            NextField::Error(err) => NextField::Error(err),
            NextField::Data(bytes) => {
                match str::from_utf8(bytes) {
                    Ok(s) => NextField::Data(s),
                    Err(_) => NextField::Error(Error::Parse(LocatableError {
                        record: record,
                        field: field,
                        err: ParseError::InvalidUtf8,
                    })),
                }
            }
        }
    }

    /// An unsafe iterator over byte fields.
    ///
    /// This iterator calls `next_bytes` at each step.
    ///
    /// It is (wildly) unsafe because the lifetime yielded for each element
    /// is incorrect. It refers to the lifetime of the CSV reader instead of
    /// the lifetime of the internal buffer. Which means you can `collect`
    /// it into a vector and obliterate memory safety.
    ///
    /// The reason it exists is because it appears extremely difficult to write
    /// a fast streaming iterator. (But iterators are wildly convenient.)
    #[doc(hidden)]
    pub unsafe fn byte_fields<'a>(&'a mut self) -> UnsafeByteFields<'a, R> {
        UnsafeByteFields { rdr: self }
    }

    /// Returns the byte offset at which the current record started.
    pub fn byte_offset(&self) -> u64 {
        self.byte_offset
    }

    #[inline]
    fn next_data(&mut self) -> NextField<[u8]> {
        if !self.first_row_done {
            self.first_row.push(self.fieldbuf.to_vec());
        }
        self.ifield += 1;
        NextField::Data(&self.fieldbuf)
    }

    #[inline]
    fn next_eor(&mut self) -> NextField<[u8]> {
        if !self.flexible
                && self.first_row_done
                && self.ifield != self.first_row.len() as u64 {
            return self.parse_error(ParseError::UnequalLengths {
                expected: self.first_row.len() as u64,
                got: self.ifield as u64,
            });
        }
        self.irecord += 1;
        self.ifield = 0;
        self.first_row_done = true;
        NextField::EndOfRecord
    }

    #[inline]
    fn next_eoc(&self) -> NextField<[u8]> {
        NextField::EndOfCsv
    }

    #[inline]
    fn fill_buf(&mut self) -> io::Result<()> {
        if self.bufi == self.buf.len() {
            unsafe { let cap = self.buf.capacity(); self.buf.set_len(cap); }
            let n = try!(self.rdr.read(&mut self.buf));
            unsafe { self.buf.set_len(n); }
            self.bufi = 0;
        }
        Ok(())
    }

    #[inline]
    fn bump(&mut self) {
        self.bufi += 1;
        self.byte_offset += 1;
    }

    #[inline]
    fn add(&mut self, c: u8) {
        self.fieldbuf.push(c);
    }

    #[inline]
    fn is_record_term(&self, c: u8) -> bool {
        self.record_term == c
    }

    fn parse_error(&self, err: ParseError) -> NextField<[u8]> {
        NextField::Error(Error::Parse(LocatableError {
            record: self.irecord,
            field: self.ifield,
            err: err,
        }))
    }
}

#[derive(Debug)]
enum State {
    StartRecord,
    EndRecord,
    StartField,
    InField,
    InQuotedField,
    InEscapedQuote,
    InDoubleEscapedQuote,
}

impl<R: io::Read + io::Seek> Reader<R> {
    /// Seeks the underlying reader to the file cursor specified.
    ///
    /// This comes with several caveats:
    ///
    /// * The existing buffer is dropped and a new one is created.
    /// * If you seek to a position other than the start of a record, you'll
    ///   probably get an incorrect parse. (This is *not* unsafe.)
    ///
    /// Mostly, this is intended for use with the `index` sub module.
    ///
    /// Note that if `pos` is equivalent to the current *parsed* byte offset,
    /// then no seeking is performed. (In this case, `seek` is a no-op.)
    pub fn seek(&mut self, pos: u64) -> Result<()> {
        self.has_seeked = true;
        self.state = StartRecord;
        if pos == self.byte_offset() {
            return Ok(())
        }
        self.bufi = self.buf.len(); // will force a buffer refresh
        self.eof = false;
        self.byte_offset = pos;
        try!(self.rdr.seek(io::SeekFrom::Start(pos)));
        Ok(())
    }
}

#[doc(hidden)]
pub struct UnsafeByteFields<'a, R: 'a> {
    rdr: &'a mut Reader<R>,
}

#[doc(hidden)]
impl<'a, R> Iterator for UnsafeByteFields<'a, R> where R: io::Read {
    type Item = Result<&'a [u8]>;

    fn next(&mut self) -> Option<Result<&'a [u8]>> {
        unsafe {
            ::std::mem::transmute(self.rdr.next_bytes().into_iter_result())
        }
    }
}

/// An iterator of decoded records.
///
/// The lifetime parameter `'a` refers to the lifetime of the underlying
/// CSV reader.
///
/// The `R` type parameter refers to the type of the underlying reader.
///
/// The `D` type parameter refers to the decoded type.
pub struct DecodedRecords<'a, R: 'a, D> {
    p: ByteRecords<'a, R>,
    _phantom: ::std::marker::PhantomData<D>,
}

impl<'a, R, D> Iterator for DecodedRecords<'a, R, D>
        where R: io::Read, D: Decodable {
    type Item = Result<D>;

    fn next(&mut self) -> Option<Result<D>> {
        self.p.next().map(|res| {
            res.and_then(|byte_record| {
                Decodable::decode(&mut Decoded::new(byte_record))
            })
        })
    }
}

/// An iterator of `String` records.
///
/// The lifetime parameter `'a` refers to the lifetime of the underlying
/// CSV reader.
///
/// The `R` type parameter refers to the type of the underlying reader.
pub struct StringRecords<'a, R: 'a> {
    p: ByteRecords<'a, R>,
}

impl<'a, R> Iterator for StringRecords<'a, R> where R: io::Read {
    type Item = Result<Vec<String>>;

    fn next(&mut self) -> Option<Result<Vec<String>>> {
        self.p.next().map(|res| {
            res.and_then(|byte_record| {
                byte_record_to_utf8(byte_record)
            })
        })
    }
}

/// An iterator of `ByteString` records.
///
/// The lifetime parameter `'a` refers to the lifetime of the underlying
/// CSV reader.
///
/// The `R` type parameter refers to the type of the underlying reader.
pub struct ByteRecords<'a, R: 'a> {
    p: &'a mut Reader<R>,
    first: bool,
    errored: bool,
}

impl<'a, R> Iterator for ByteRecords<'a, R> where R: io::Read {
    type Item = Result<Vec<ByteString>>;

    fn next(&mut self) -> Option<Result<Vec<ByteString>>> {
        // We check this before checking `done` because the parser could
        // be done after a call to `byte_headers` but before any iterator
        // traversal. Once we start the iterator, we must allow the first
        // row to be returned if the caller has said that this CSV data
        // has no headers.
        if !self.first {
            // Never do this special first record processing again.
            self.first = true;

            // Always consume the header record. This let's us choose to
            // return it or ignore it and move on to the next record.
            // If headers have been read before this point, then this is
            // equivalent to a harmless clone (and no parser progression).
            let headers = self.p.byte_headers();

            // If the header row is empty, then the CSV data contains
            // no records. Never return zero-length records!
            if headers.as_ref().map(|r| r.is_empty()).unwrap_or(false) {
                assert!(self.p.done());
                return None;
            }

            // This is important. If the client says this CSV data has no
            // headers but calls `headers` before iterating records (which is
            // perfectly valid), then we need to make sure to return that
            // first record.
            //
            // If the client says the CSV data has headers, then the first
            // record should always be ignored.
            if !self.p.has_headers {
                return Some(headers);
            }
        }
        // OK, we're done checking the weird first-record-corner-case.
        if self.p.done() || self.errored {
            return None;
        }
        let mut record = Vec::with_capacity(self.p.first_row.len());
        loop {
            match self.p.next_bytes() {
                NextField::EndOfRecord | NextField::EndOfCsv => {
                    if record.len() == 0 {
                        return None
                    }
                    break
                }
                NextField::Error(err) => {
                    self.errored = true;
                    return Some(Err(err));
                }
                NextField::Data(field) => record.push(field.to_vec()),
            }
        }
        Some(Ok(record))
    }
}

fn byte_record_to_utf8(record: Vec<ByteString>) -> Result<Vec<String>> {
    for bytes in record.iter() {
        if let Err(err) = ::std::str::from_utf8(&**bytes) {
            return Err(Error::Decode(format!(
                "Could not decode the following bytes as UTF-8 \
                 because {}: {:?}", err.to_string(), bytes)));
        }
    }
    Ok(unsafe { ::std::mem::transmute(record) })
}
