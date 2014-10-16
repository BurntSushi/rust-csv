use std::io::{mod, MemReader};
use std::mem::transmute;

use serialize::Decodable;

use buffered::BufferedReader;
use {
    ByteString, CsvResult, Decoded, IntoVector,
    Error, ErrDecode, ErrIo, ErrParse,
    ParseError,
    ParseErrorKind, UnequalLengths,
};

// TODO: Make these parameters?
static QUOTE: u8 = b'"';
static ESCAPE: u8 = b'\\';

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
///     let (n1, n2, dist): (String, String, uint) = row.unwrap();
///     println!("{}, {}: {:u}", n1, n2, dist);
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
///     println!("{}", row);
/// }
/// ```
pub struct Reader<R> {
    delimiter: u8,
    flexible: bool, // true => records of varying length are allowed
    buffer: BufferedReader<R>,
    fieldbuf: Vec<u8>, // reusable buffer used to store fields
    state: ParseState, // current state in parsing machine
    err: Option<Error>, // current error; when `Some`, parsing is done forever

    // Keep a copy of the first record parsed.
    first_record: Vec<ByteString>,

    // When this is true, the first record is interpreted as a "header" row.
    // This is opaque to the raw iterator, but is used in any iterator that
    // allocates.
    //
    // TODO: This is exposed for use in the `index` sub-module. Is that OK?
    #[doc(hidden)]
    pub has_headers: bool,

    // Various book-keeping counts.
    field_count: u64, // number of fields in current record
    column: u64, // current column (by byte, *shrug*)
    line_record: u64, // line at which current record started
    line_current: u64, // current line
    byte_offset: u64, // current byte offset
}

impl Reader<io::IoResult<io::File>> {
    /// Creates a new CSV reader for the data at the file path given.
    pub fn from_file(path: &Path) -> Reader<io::IoResult<io::File>> {
        Reader::from_reader(io::File::open(path))
    }
}

impl<R: io::Reader> Reader<R> {
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
    /// let rows = csv::collect(rdr.records()).unwrap();
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
    /// let rows = csv::collect(rdr.records()).unwrap();
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
    pub fn headers(&mut self) -> CsvResult<Vec<String>> {
        byte_record_to_utf8(try!(self.byte_headers()))
    }

    /// Uses type-based decoding to read a single record from CSV data.
    ///
    /// The type that is being decoded into should correspond to *one full
    /// CSV record*. A tuple, struct or `Vec` fit this category. A tuple,
    /// struct or `Vec` should consist of primitive types like integers,
    /// floats, characters and strings which map to single fields. If a field
    /// cannot be decoded into the type requested, an error is returned.
    ///
    /// Enums are also supported in a limited way. Namely, its variants must
    /// have exactly `0` or `1` parameters. Variants with `0` parameters decode
    /// based on a case-insensitive string match. Variants with `1` decode
    /// based on its constituent type. Examples follow.
    ///
    /// ### Examples
    ///
    /// This example shows how to decode records into a struct. (Note that
    /// currently, the *names* of the struct members are irrelevant.)
    ///
    /// ```rust
    /// extern crate serialize;
    /// # extern crate csv;
    /// # fn main() {
    ///
    /// #[deriving(Decodable)]
    /// struct Pair {
    ///     name1: String,
    ///     name2: String,
    ///     dist: uint,
    /// }
    ///
    /// let mut rdr = csv::Reader::from_string("foo,bar,1\nfoo,baz,2")
    ///                           .has_headers(false);
    /// // Instantiating a specific type when decoding is usually necessary.
    /// let rows = csv::collect(rdr.decode::<Pair>()).unwrap();
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
    /// extern crate serialize;
    /// # extern crate csv;
    /// # fn main() {
    ///
    /// #[deriving(Decodable, PartialEq, Show)]
    /// struct MyUint(uint);
    ///
    /// #[deriving(Decodable, PartialEq, Show)]
    /// enum Color { Red, Green, Blue };
    ///
    /// #[deriving(Decodable)]
    /// struct Pair {
    ///     name1: String,
    ///     name2: String,
    ///     dist: Option<MyUint>,
    ///     color: Color,
    /// }
    ///
    /// let mut rdr = csv::Reader::from_string("foo,bar,1,red\nfoo,baz,,green")
    ///                           .has_headers(false);
    /// let rows = csv::collect(rdr.decode::<Pair>()).unwrap();
    ///
    /// assert_eq!(rows[0].dist, Some(MyUint(1)));
    /// assert_eq!(rows[1].dist, None);
    ///
    /// assert_eq!(rows[0].color, Red);
    /// assert_eq!(rows[1].color, Green);
    /// # }
    /// ```
    ///
    /// Finally, as a special case, a tuple/struct/`Vec` can be used as the
    /// "tail" of another tuple/struct/`Vec` to capture all remaining fields:
    ///
    /// ```rust
    /// extern crate serialize;
    /// # extern crate csv;
    /// # fn main() {
    ///
    /// #[deriving(Decodable)]
    /// struct Pair {
    ///     name1: String,
    ///     name2: String,
    ///     attrs: Vec<uint>,
    /// }
    ///
    /// let mut rdr = csv::Reader::from_string("a,b,1,2,3,4\ny,z,5,6,7,8")
    ///                           .has_headers(false);
    /// let rows = csv::collect(rdr.decode::<Pair>()).unwrap();
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
    pub fn decode<'a, D: Decodable<Decoded, Error>>
                 (&'a mut self) -> DecodedRecords<'a, R, D> {
        DecodedRecords { p: self.byte_records() }
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
    ///     println!("{}", row);
    /// }
    /// ```
    pub fn records<'a>(&'a mut self) -> StringRecords<'a, R> {
        StringRecords { p: self.byte_records() }
    }
}

impl<R: io::Reader> Reader<R> {
    /// Creates a new CSV reader from an arbitrary `io::Reader`.
    ///
    /// The reader is buffered for you automatically.
    pub fn from_reader(rdr: R) -> Reader<R> {
        Reader::from_buffer(BufferedReader::new(rdr))
    }

    /// Creates a new CSV reader from a buffer.
    ///
    /// This allows you to create your own buffer with a capacity of your
    /// choosing. In all other constructors, a buffer with default capacity
    /// is created for you.
    fn from_buffer(buf: BufferedReader<R>) -> Reader<R> {
        Reader {
            delimiter: b',',
            flexible: false,
            buffer: buf,
            fieldbuf: Vec::with_capacity(1024),
            state: StartRecord,
            err: None,
            first_record: vec![],
            has_headers: true,
            field_count: 0,
            column: 1,
            line_record: 1,
            line_current: 1,
            byte_offset: 0,
        }
    }

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
}

impl Reader<MemReader> {
    /// Creates a CSV reader for an in memory string buffer.
    pub fn from_string<S: StrAllocating>(s: S) -> Reader<MemReader> {
        Reader::from_bytes(s.into_string().into_bytes())
    }

    /// Creates a CSV reader for an in memory buffer of bytes.
    pub fn from_bytes<V: IntoVector<u8>>(bytes: V) -> Reader<MemReader> {
        Reader::from_reader(MemReader::new(bytes.into_vec()))
    }
}

/// These are low level methods for dealing with the raw bytes of CSV records.
/// You should only need to use these when you need the performance or if
/// your CSV data isn't UTF-8 encoded.
impl<R: io::Reader> Reader<R> {
    /// This is just like `headers`, except fields are `ByteString`s instead
    /// of `String`s.
    pub fn byte_headers(&mut self) -> CsvResult<Vec<ByteString>> {
        if !self.first_record.is_empty() {
            Ok(self.first_record.clone())
        } else {
            let mut headers = vec![];
            loop {
                let field = match self.next_field() {
                    None => break,
                    Some(field) => try!(field),
                };
                headers.push(ByteString::from_bytes(field));
            }
            assert!(headers.len() > 0 || self.done());
            Ok(headers)
        }
    }

    /// This is just like `records`, except fields are `ByteString`s instead
    /// of `String`s.
    pub fn byte_records<'a>(&'a mut self) -> ByteRecords<'a, R> {
        ByteRecords { p: self, first: false }
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
    /// let mut count = 0u;
    /// while !rdr.done() {
    ///     loop {
    ///         match rdr.next_field() {
    ///             None => break,
    ///             Some(r) => { r.unwrap(); }
    ///         }
    ///     }
    ///     count += 1;
    /// }
    ///
    /// assert_eq!(count, 5);
    /// ```
    pub fn done(&self) -> bool {
        self.err.is_some()
    }

    /// An iterator over fields in the current record.
    ///
    /// This provides low level access to CSV records as raw byte slices.
    /// Namely, no allocation is performed. Unlike other iterators in this
    /// crate, this yields *fields* instead of records. Notably, this cannot
    /// implement the `Iterator` trait safely. As such, it cannot be used with
    /// a `for` loop.
    ///
    /// The semantics of this iterator are a little strange. The iterator will
    /// produce `None` at the end of a record, but the next invocation of the
    /// iterator will restart at the next record. Once all CSV data has been
    /// parsed, `None` will be returned indefinitely.
    ///
    /// This iterator always returns all records (i.e., it won't skip the
    /// header row).
    ///
    /// ### Example
    ///
    /// Typically, once an iterator returns `None`, it will always return `None`.
    /// Since this iterator varies from that behavior, it should be used in
    /// conjunction with the `done` method to traverse records.
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
    ///     loop {
    ///         let field = match rdr.next_field() {
    ///             None => break,
    ///             Some(result) => result.unwrap(),
    ///         };
    ///         print!("{}", field);
    ///     }
    ///     println!("");
    /// }
    /// ```
    pub fn next_field<'a>(&'a mut self) -> Option<CsvResult<&'a [u8]>> {
        unsafe { self.fieldbuf.set_len(0); }
        
        // The EndRecord state indicates what you'd expect: stop the current
        // iteration, check for same-length records and reset a little
        // record-based book keeping.
        if self.state == EndRecord {
            let first_len = self.first_record.len() as u64;
            if !self.flexible && first_len != self.field_count {
                let err = self.parse_err(UnequalLengths(
                    self.first_record.len() as u64, self.field_count));
                self.err = Some(err.clone());
                return Some(Err(err));
            }
            // After processing an EndRecord (and determined there are no
            // errors), we should always start parsing the next record.
            self.state = StartRecord;
            self.line_record = self.line_current;
            self.field_count = 0;
            return None;
        }

        // Check to see if we've recorded an error and quit parsing if we have.
        // This serves two purposes:
        // 1) When CSV parsing reaches an error, it is unrecoverable. So the
        //    parse function will initially return that error (unless it is
        //    EOF) and then return `None` indefinitely.
        // 2) EOF errors are handled specially and can be returned "lazily".
        //    e.g., EOF in the middle of parsing a field. First we have to
        //    return the field and then return EOF on the next call.
        match self.err {
            None => {},
            // We don't return the error here because it is always returned
            // immediately when it is first found (unless it's EOF, but if it's
            // EOF, we just want to stop the iteration anyway).
            Some(_) => return None,
        }

        // A parser machine encapsulates the main parsing state transitions.
        // Normally, the state machine would be written as methods on the
        // Reader type, but mutable borrows become troublesome. So we isolate
        // the things we need to mutate during state transitions with
        // the ParseMachine type.
        let mut pmachine = ParseMachine {
            fieldbuf: &mut self.fieldbuf,
            state: &mut self.state,
            delimiter: self.delimiter,
        };
        let mut consumed = 0; // tells the buffer how much we consumed
        'TOPLOOP: loop {
            // The following code is basically, "fill a buffer with data from
            // the underlying reader if it's empty, and then run the parser
            // over each byte in the slice returned."
            //
            // This technique is critical for performance, because it lifts
            // a lot of case analysis off of each byte. (i.e., This loop could
            // be more simply written with `buf.read_byte()`, but it is much
            // slower.)
            match self.buffer.fill_buf() {
                Err(err) => {
                    // The error is processed below.
                    // We don't handle it here because we need to do some
                    // book keeping first.
                    self.err = Some(ErrIo(err));
                    break 'TOPLOOP;
                }
                Ok(bs) => {
                    // This "batch" processing of bytes is critical for
                    // performance.
                    for &b in bs.iter() {
                        pmachine.parse_byte(b);
                        if *pmachine.state == EndRecord {
                            // Don't consume the byte we just read, because
                            // it is the first byte of the next record.
                            break 'TOPLOOP;
                        } else {
                            consumed += 1;
                            self.column += 1;
                            self.byte_offset += 1;
                            if is_crlf(b) {
                                if b == b'\n' {
                                    self.line_current += 1;
                                }
                                self.column = 1;
                            }
                            if *pmachine.state == StartField {
                                break 'TOPLOOP
                            }
                        }
                    }
                }
            }
            self.buffer.consume(consumed);
            consumed = 0;
        }
        // We get here when we break out of the loop, so make sure the buffer
        // knows how much we read.
        self.buffer.consume(consumed);

        // Handle the error. EOF is a bit tricky, but otherwise, we just stop
        // the parser cold.
        match self.err {
            None => {}
            Some(ErrIo(io::IoError { kind: io::EndOfFile, .. })) => {
                // If we get EOF while we're trying to parse a new record
                // but haven't actually seen any fields yet (i.e., trailing
                // new lines in a file), then we should immediately stop the
                // parser.
                if *pmachine.state == StartRecord {
                    return None;
                }
                *pmachine.state = EndRecord;
                // fallthrough to return current field.
                // On the next call, `None` will be returned.
            }
            Some(ref err) => {
                // Reset the state to the beginning so that bad errors
                // are always reported. (i.e., Don't let an EndRecord state
                // slip in here.)
                *pmachine.state = StartRecord;
                return Some(Err(err.clone()));
            }
        }
        if self.line_record == 1 {
            // This is only copying bytes for the first record.
            let bytes = ByteString::from_bytes((*pmachine.fieldbuf)[]);
            self.first_record.push(bytes);
        }
        self.field_count += 1;
        Some(Ok((*pmachine.fieldbuf)[]))
    }

    /// An unsafe iterator over byte fields.
    ///
    /// This iterator calls `next_field` at each step.
    ///
    /// It is (wildly) unsafe because the lifetime yielded for each element
    /// is incorrect. It refers to the lifetime of the CSV reader instead of
    /// the lifetime of the internal buffer. Which means you can `collect`
    /// it into a vector and obliterate memory safety.
    ///
    /// The reason it exists is because it appears extremely difficult to write
    /// a fast streaming iterator.
    #[doc(hidden)]
    pub unsafe fn byte_fields<'a>(&'a mut self) -> UnsafeByteFields<'a, R> {
        UnsafeByteFields { rdr: self }
    }

    /// Returns the line at which the current record started.
    pub fn line(&self) -> u64 {
        self.line_record
    }

    /// Returns the byte offset at which the current record started.
    pub fn byte_offset(&self) -> u64 {
        self.byte_offset
    }

    fn parse_err(&self, kind: ParseErrorKind) -> Error {
        ErrParse(ParseError {
            line: self.line_record,
            column: self.column,
            kind: kind,
        })
    }
}

#[doc(hidden)]
pub struct UnsafeByteFields<'a, R: 'a> {
    rdr: &'a mut Reader<R>,
}

#[doc(hidden)]
impl<'a, R: io::Reader> Iterator<CsvResult<&'a [u8]>>
    for UnsafeByteFields<'a, R> {
    fn next(&mut self) -> Option<CsvResult<&'a [u8]>> {
        unsafe { transmute(self.rdr.next_field()) }
    }
}

#[doc(hidden)]
impl<'a, R: io::Reader> Iterator<CsvResult<&'a [u8]>> for Reader<R> {
    fn next(&mut self) -> Option<CsvResult<&'a [u8]>> {
        unsafe { transmute(self.next_field()) }
    }
}

impl<R: io::Reader + io::Seek> Reader<R> {
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
    pub fn seek(&mut self, pos: i64, style: io::SeekStyle) -> CsvResult<()> {
        if pos as u64 == self.byte_offset() {
            return Ok(())
        }
        self.buffer.clear();
        self.err = None;
        self.byte_offset = pos as u64;
        try!(self.buffer.get_mut_ref().seek(pos, style).map_err(ErrIo));
        Ok(())
    }
}

#[deriving(Eq, PartialEq, Show)]
enum ParseState {
    StartRecord,
    EndRecord,
    StartField,
    EatCRLF,
    InField,
    InQuotedField,
    InQuotedFieldEscape,
    InQuotedFieldQuote,
}

struct ParseMachine<'a> {
    fieldbuf: &'a mut Vec<u8>,
    state: &'a mut ParseState,
    delimiter: u8,
}

impl<'a> ParseMachine<'a> {
    #[inline]
    fn parse_byte(&mut self, b: u8) {
        match *self.state {
            StartRecord => self.parse_start_record(b),
            EndRecord => unreachable!(),
            StartField => self.parse_start_field(b),
            EatCRLF => self.parse_eat_crlf(b),
            InField => self.parse_in_field(b),
            InQuotedField => self.parse_in_quoted_field(b),
            InQuotedFieldEscape => self.parse_in_quoted_field_escape(b),
            InQuotedFieldQuote => self.parse_in_quoted_field_quote(b),
        }
    }

    #[inline]
    fn parse_start_record(&mut self, b: u8) {
        if !is_crlf(b) {
            *self.state = StartField;
            self.parse_start_field(b);
        }
    }

    #[inline]
    fn parse_start_field(&mut self, b: u8) {
        if is_crlf(b) {
            *self.state = EatCRLF;
        } else if b == QUOTE {
            *self.state = InQuotedField;
        } else if b == self.delimiter {
            // empty field, so return in StartField state,
            // which causes a new empty field to be returned
        } else {
            *self.state = InField;
            self.fieldbuf.push(b);
        }
    }

    #[inline]
    fn parse_eat_crlf(&mut self, b: u8) {
        if !is_crlf(b) {
            *self.state = EndRecord;
        }
    }

    #[inline]
    fn parse_in_field(&mut self, b: u8) {
        if is_crlf(b) {
            *self.state = EatCRLF;
        } else if b == self.delimiter {
            *self.state = StartField;
        } else {
            self.fieldbuf.push(b);
        }
    }

    #[inline]
    fn parse_in_quoted_field(&mut self, b: u8) {
        if b == ESCAPE {
            *self.state = InQuotedFieldEscape;
        } else if b == QUOTE {
            *self.state = InQuotedFieldQuote;
        } else {
            self.fieldbuf.push(b);
        }
    }

    #[inline]
    fn parse_in_quoted_field_escape(&mut self, b: u8) {
        *self.state = InQuotedField;
        self.fieldbuf.push(b);
    }

    #[inline]
    fn parse_in_quoted_field_quote(&mut self, b: u8) {
        if b == QUOTE {
            *self.state = InQuotedField;
            self.fieldbuf.push(b);
        } else if b == self.delimiter {
            *self.state = StartField;
        } else if is_crlf(b) {
            *self.state = EatCRLF;
        } else {
            // Should we provide a strict variant that disallows
            // random chars after a quote?
            *self.state = InField;
            self.fieldbuf.push(b);
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
}

impl<'a, R: io::Reader, D: Decodable<Decoded, Error>> Iterator<CsvResult<D>>
    for DecodedRecords<'a, R, D> {
    fn next(&mut self) -> Option<CsvResult<D>> {
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

impl<'a, R: io::Reader> Iterator<CsvResult<Vec<String>>>
    for StringRecords<'a, R> {
    fn next(&mut self) -> Option<CsvResult<Vec<String>>> {
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
}

impl<'a, R: io::Reader> Iterator<CsvResult<Vec<ByteString>>>
    for ByteRecords<'a, R> {
    fn next(&mut self) -> Option<CsvResult<Vec<ByteString>>> {
        if self.p.done() {
            return None;
        }
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

        let mut record = Vec::with_capacity(self.p.first_record.len());
        loop {
            let field = match self.p.next_field() {
                None => break,
                Some(field) => field,
            };
            match field {
                Err(err) => return Some(Err(err)),
                Ok(bytes) => record.push(ByteString::from_bytes(bytes)),
            }
        }
        Some(Ok(record))
    }
}

#[inline]
fn is_crlf(b: u8) -> bool { b == b'\n' || b == b'\r' }

fn byte_record_to_utf8(record: Vec<ByteString>) -> CsvResult<Vec<String>> {
    for bytes in record.iter() {
        if !::std::str::is_utf8(bytes[]) {
            return Err(ErrDecode(format!(
                "Could not decode the following bytes as UTF-8: {}", bytes)));
        }
    }
    Ok(unsafe { transmute(record) })
}
