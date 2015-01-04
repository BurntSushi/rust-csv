use std::io::{self, MemReader};

use rustc_serialize::Decodable;

use buffered::BufferedReader;
use {
    ByteString, CsvResult, Decoded, IntoVector,
    Error, ParseError, ParseErrorKind,
    StrAllocating,
};

use self::ParseState::{
    StartRecord, EndRecord, StartField,
    RecordTermCR, RecordTermLF, RecordTermAny,
    InField, InQuotedField, InQuotedFieldEscape, InQuotedFieldQuote,
};

/// A record terminator.
///
/// Ideally, this would just be a `u8` like any other delimiter, but a useful
/// CSV parser must special case CRLF handling. Hence, this enum.
///
/// Generally, you won't need to use this type because `CRLF` is the default,
/// which is by far the most widely used record terminator.
#[derive(Copy)]
pub enum RecordTerminator {
    /// Parses `\r`, `\n` or `\r\n` as a single record terminator.
    CRLF,
    /// Parses the byte given as a record terminator.
    Any(u8),
}

impl PartialEq<u8> for RecordTerminator {
    fn eq(&self, other: &u8) -> bool {
        match *self {
            RecordTerminator::CRLF => *other == b'\r' || *other == b'\n',
            RecordTerminator::Any(b) => *other == b
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
///     let (n1, n2, dist): (String, String, uint) = row.unwrap();
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
///     println!("{}", row);
/// }
/// ```
pub struct Reader<R> {
    pmachine: ParseMachine, // various parsing settings
    flexible: bool, // true => records of varying length are allowed
    buffer: BufferedReader<R>,
    fieldbuf: Vec<u8>, // reusable buffer used to store fields
    state: ParseState, // current state in parsing machine
    err: Option<Error>, // current error; when `Some`, parsing is done forever

    // Keep a copy of the first record parsed.
    first_record: Vec<ByteString>,
    parsing_first_record: bool, // true only before first EndRecord state

    // Is set if `seek` is ever called.
    // This subtlely modifies the behavior of iterators so that there is
    // no special handling of headers. (After you seek, iterators should
    // just give whatever records are being parsed.)
    has_seeked: bool,

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
    ///
    /// ... but this isn't public right now because we're using our own
    /// implemented of `BufferedReader`.
    fn from_buffer(buf: BufferedReader<R>) -> Reader<R> {
        Reader {
            pmachine: ParseMachine {
                delimiter: b',',
                record_terminator: RecordTerminator::CRLF,
                quote: Some(b'"'),
                escape: b'\\',
                double_quote: true,
            },
            flexible: false,
            buffer: buf,
            fieldbuf: Vec::with_capacity(1024),
            state: StartRecord,
            err: None,
            first_record: vec![],
            parsing_first_record: true,
            has_seeked: false,
            has_headers: true,
            field_count: 0,
            column: 1,
            line_record: 1,
            line_current: 1,
            byte_offset: 0,
        }
    }
}

impl Reader<io::IoResult<io::File>> {
    /// Creates a new CSV reader for the data at the file path given.
    pub fn from_file(path: &Path) -> Reader<io::IoResult<io::File>> {
        Reader::from_reader(io::File::open(path))
    }
}

impl Reader<MemReader> {
    /// Creates a CSV reader for an in memory string buffer.
    pub fn from_string<S>(s: S) -> Reader<MemReader> where S: StrAllocating {
        Reader::from_bytes(s.into_str().into_bytes())
    }

    /// Creates a CSV reader for an in memory buffer of bytes.
    pub fn from_bytes<V: IntoVector<u8>>(bytes: V) -> Reader<MemReader> {
        Reader::from_reader(MemReader::new(bytes.into_vec()))
    }
}

impl<R: io::Reader> Reader<R> {
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
    /// extern crate "rustc-serialize" as rustc_serialize;
    /// # extern crate csv;
    /// # fn main() {
    ///
    /// #[deriving(RustcDecodable)]
    /// struct Pair {
    ///     name1: String,
    ///     name2: String,
    ///     dist: uint,
    /// }
    ///
    /// let mut rdr = csv::Reader::from_string("foo,bar,1\nfoo,baz,2")
    ///                           .has_headers(false);
    /// // Instantiating a specific type when decoding is usually necessary.
    /// let rows = rdr.decode().collect::<Result<Vec<Pair>, _>>().unwrap();
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
    /// extern crate "rustc-serialize" as rustc_serialize;
    /// # extern crate csv;
    /// # fn main() {
    ///
    /// #[deriving(RustcDecodable, PartialEq, Show)]
    /// struct MyUint(uint);
    ///
    /// #[deriving(RustcDecodable, PartialEq, Show)]
    /// enum Number { Integer(i64), Float(f64) }
    ///
    /// #[deriving(RustcDecodable)]
    /// struct Row {
    ///     name1: String,
    ///     name2: String,
    ///     dist: Option<MyUint>,
    ///     dist2: Number,
    /// }
    ///
    /// let mut rdr = csv::Reader::from_string("foo,bar,1,1\nfoo,baz,,1.5")
    ///                           .has_headers(false);
    /// let rows = rdr.decode().collect::<Result<Vec<Row>, _>>().unwrap();
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
    /// extern crate "rustc-serialize" as rustc_serialize;
    /// # extern crate csv;
    /// # fn main() {
    ///
    /// #[deriving(RustcDecodable)]
    /// struct Pair {
    ///     name1: String,
    ///     name2: String,
    ///     attrs: Vec<uint>,
    /// }
    ///
    /// let mut rdr = csv::Reader::from_string("a,b,1,2,3,4\ny,z,5,6,7,8")
    ///                           .has_headers(false);
    /// let rows = rdr.decode().collect::<Result<Vec<Pair>, _>>().unwrap();
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
    /// let rows = rdr.records().collect::<Result<Vec<_>, _>>().unwrap();
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
    /// let rows = rdr.records().collect::<Result<Vec<_>, _>>().unwrap();
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
}

impl<R: io::Reader> Reader<R> {
    /// The delimiter to use when reading CSV data.
    ///
    /// Since the CSV reader is meant to be mostly encoding agnostic, you must
    /// specify the delimiter as a single ASCII byte. For example, to read
    /// tab-delimited data, you would use `b'\t'`.
    ///
    /// The default value is `b','`.
    pub fn delimiter(mut self, delimiter: u8) -> Reader<R> {
        self.pmachine.delimiter = delimiter;
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
        self.pmachine.record_terminator = term;
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
    pub fn quote(mut self, quote: Option<u8>) -> Reader<R> {
        self.pmachine.quote = quote;
        self
    }

    /// Set the escape character to use when reading CSV data.
    ///
    /// This is only used when `double_quote` is set to false.
    ///
    /// Since the CSV reader is meant to be mostly encoding agnostic, you must
    /// specify the escape as a single ASCII byte.
    ///
    /// The default value is `b'\\'`.
    pub fn escape(mut self, escape: u8) -> Reader<R> {
        self.pmachine.escape = escape;
        self
    }

    /// Set the quoting escape mechanism.
    ///
    /// When enabled (which is the default), quotes are escaped by doubling
    /// them. e.g., `""` resolves to a single `"`.
    ///
    /// When disabled, double quotes have no significance. Instead, they can
    /// be escaped with the escape character (which is `\\` by default).
    pub fn double_quote(mut self, yes: bool) -> Reader<R> {
        self.pmachine.double_quote = yes;
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
        self.quote(None)
            .double_quote(false)
            .delimiter(b'\x1f')
            .record_terminator(RecordTerminator::Any(b'\x1e'))
    }
}

/// NextField is the result of parsing a single CSV field.
///
/// This is only useful if you're using the low level `next_field` method.
pub enum NextField<'a> {
    /// A single CSV field as a borrow slice of bytes from the
    /// parser's internal buffer.
    Data(&'a [u8]),

    /// A CSV error found during parsing. When an error is found, it is
    /// first returned. All subsequent calls of `next_field` will return
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

impl<'a> NextField<'a> {
    /// Transform NextField into an iterator result.
    pub fn into_iter_result(self) -> Option<CsvResult<&'a [u8]>> {
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
                    NextField::EndOfRecord | NextField::EndOfCsv => break,
                    NextField::Error(err) => return Err(err),
                    NextField::Data(field) => field,
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
        let first = self.has_seeked;
        ByteRecords { p: self, first: first }
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
    ///         // This case analysis is necessary because we only want to
    ///         // increment the count when `EndOfRecord` is seen. (If the
    ///         // CSV data is empty, then it will never be emitted.)
    ///         match rdr.next_field() {
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
    ///     while let Some(r) = rdr.next_field().into_iter_result() {
    ///         print!("{} ", r.unwrap());
    ///     }
    ///     println!("");
    /// }
    /// ```
    pub fn next_field<'a>(&'a mut self) -> NextField<'a> {
        unsafe { self.fieldbuf.set_len(0); }

        // The EndRecord state indicates what you'd expect: stop the current
        // iteration, check for same-length records and reset a little
        // record-based book keeping.
        if self.state == EndRecord {
            let first_len = self.first_record.len() as u64;
            if !self.flexible && first_len != self.field_count {
                let err = self.parse_err(ParseErrorKind::UnequalLengths(
                    self.first_record.len() as u64, self.field_count));
                self.err = Some(err.clone());
                return NextField::Error(err);
            }
            // After processing an EndRecord (and determined there are no
            // errors), we should always start parsing the next record.
            self.state = StartRecord;
            self.parsing_first_record = false;
            self.line_record = self.line_current;
            self.field_count = 0;
            return NextField::EndOfRecord;
        }

        // Check to see if we've recorded an error and quit parsing if we have.
        // This serves two purposes:
        // 1) When CSV parsing reaches an error, it is unrecoverable. So the
        //    parse function will initially return that error (unless it is
        //    EOF) and then return `None` indefinitely.
        // 2) EOF errors are handled specially and can be returned "lazily".
        //    e.g., EOF in the middle of parsing a field. First we have to
        //    return the field and then return EOF on the next call.
        if let Some(_) = self.err {
            // We don't return the error here because it is always returned
            // immediately when it is first found (unless it's EOF, but if it's
            // EOF, we just want to stop the iteration anyway).
            return NextField::EndOfCsv;
        }

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
                    self.err = Some(Error::Io(err));
                    break 'TOPLOOP;
                }
                Ok(bs) => {
                    // This "batch" processing of bytes is critical for
                    // performance.
                    for &b in bs.iter() {
                        let (accept, next) =
                            self.pmachine.parse_byte(self.state, b);
                        self.state = next;
                        if accept { self.fieldbuf.push(b); }
                        if self.state == EndRecord {
                            // Don't consume the byte we just read, because
                            // it is the first byte of the next record.
                            break 'TOPLOOP;
                        } else {
                            consumed += 1;
                            self.column += 1;
                            self.byte_offset += 1;
                            if b == b'\n' {
                                self.line_current += 1;
                                self.column = 1;
                            }
                            if self.state == StartField {
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
            Some(Error::Io(io::IoError { kind: io::EndOfFile, .. })) => {
                // If we get EOF while we're trying to parse a new record
                // but haven't actually seen any fields yet (i.e., trailing
                // new lines in a file), then we should immediately stop the
                // parser.
                if self.state == StartRecord {
                    return NextField::EndOfCsv;
                }
                self.state = EndRecord;
                // fallthrough to return current field.
                // On the next call, `None` will be returned.
            }
            Some(ref err) => {
                // Reset the state to the beginning so that bad errors
                // are always reported. (i.e., Don't let an EndRecord state
                // slip in here.)
                self.state = StartRecord;
                return NextField::Error(err.clone());
            }
        }
        if self.parsing_first_record {
            // This is only copying bytes for the first record.
            let bytes = ByteString::from_bytes(self.fieldbuf.as_slice());
            self.first_record.push(bytes);
        }
        self.field_count += 1;
        NextField::Data(self.fieldbuf.as_slice())
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
    /// a fast streaming iterator. (But iterators are wildly convenient.)
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
        Error::Parse(ParseError {
            line: self.line_record,
            column: self.column,
            kind: kind,
        })
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
        self.has_seeked = true;
        if pos as u64 == self.byte_offset() {
            return Ok(())
        }
        self.buffer.clear();
        self.err = None;
        self.byte_offset = pos as u64;
        try!(self.buffer.get_mut().seek(pos, style));
        Ok(())
    }
}

#[derive(Copy)]
struct ParseMachine {
    delimiter: u8,
    record_terminator: RecordTerminator,
    quote: Option<u8>,
    escape: u8,
    double_quote: bool,
}

#[derive(Copy, Eq, PartialEq, Show)]
enum ParseState {
    StartRecord,
    EndRecord,
    StartField,
    RecordTermCR,
    RecordTermLF,
    RecordTermAny,
    InField,
    InQuotedField,
    InQuotedFieldEscape,
    InQuotedFieldQuote,
}

type NextState = (bool, ParseState);

impl ParseMachine {
    #[inline]
    fn parse_byte(&self, state: ParseState, b: u8) -> NextState {
        match state {
            StartRecord => self.parse_start_record(b),
            EndRecord => unreachable!(),
            StartField => self.parse_start_field(b),
            RecordTermCR => self.parse_record_term_cr(b),
            RecordTermLF => self.parse_record_term_lf(b),
            RecordTermAny => self.parse_record_term_any(b),
            InField => self.parse_in_field(b),
            InQuotedField => self.parse_in_quoted_field(b),
            InQuotedFieldEscape => self.parse_in_quoted_field_escape(b),
            InQuotedFieldQuote => self.parse_in_quoted_field_quote(b),
        }
    }

    #[inline]
    fn parse_start_record(&self, b: u8) -> NextState {
        if self.is_record_term(b) {
            // Skip empty new lines.
            (false, StartRecord)
        } else {
            self.parse_start_field(b)
        }
    }

    #[inline]
    fn parse_start_field(&self, b: u8) -> NextState {
        if self.is_record_term(b) {
            (false, self.record_term_next_state(b))
        } else if Some(b) == self.quote {
            (false, InQuotedField)
        } else if b == self.delimiter {
            // empty field, so return in StartField state,
            // which causes a new empty field to be returned
            (false, StartField)
        } else {
            (true, InField)
        }
    }

    #[inline]
    fn parse_record_term_cr(&self, b: u8) -> NextState {
        if b == b'\n' {
            (false, RecordTermLF)
        } else if b == b'\r' {
            (false, RecordTermCR)
        } else {
            (false, EndRecord)
        }
    }

    #[inline]
    fn parse_record_term_lf(&self, b: u8) -> NextState {
        if b == b'\r' {
            (false, RecordTermCR)
        } else if b == b'\n' {
            (false, RecordTermLF)
        } else {
            (false, EndRecord)
        }
    }

    #[inline]
    fn parse_record_term_any(&self, b: u8) -> NextState {
        match self.record_terminator {
            RecordTerminator::CRLF => unreachable!(),
            RecordTerminator::Any(bb) => {
                if b == bb {
                    (false, RecordTermAny)
                } else {
                    (false, EndRecord)
                }
            }
        }
    }

    #[inline]
    fn parse_in_field(&self, b: u8) -> NextState {
        if self.is_record_term(b) {
            (false, self.record_term_next_state(b))
        } else if b == self.delimiter {
            (false, StartField)
        } else {
            (true, InField)
        }
    }

    #[inline]
    fn parse_in_quoted_field(&self, b: u8) -> NextState {
        if Some(b) == self.quote {
            (false, InQuotedFieldQuote)
        } else if !self.double_quote && b == self.escape {
            (false, InQuotedFieldEscape)
        } else {
            (true, InQuotedField)
        }
    }

    #[inline]
    fn parse_in_quoted_field_escape(&self, _: u8) -> NextState {
        (true, InQuotedField)
    }

    #[inline]
    fn parse_in_quoted_field_quote(self, b: u8) -> NextState {
        if self.double_quote && Some(b) == self.quote {
            (true, InQuotedField)
        } else if b == self.delimiter {
            (false, StartField)
        } else if self.is_record_term(b) {
            (false, self.record_term_next_state(b))
        } else {
            // Should we provide a strict variant that disallows
            // random chars after a quote?
            (true, InField)
        }
    }

    #[inline]
    fn is_record_term(self, b: u8) -> bool {
        self.record_terminator == b
    }

    #[inline]
    fn record_term_next_state(self, b: u8) -> ParseState {
        match self.record_terminator {
            RecordTerminator::CRLF => {
                if b == b'\r' {
                    RecordTermCR
                } else if b == b'\n' {
                    RecordTermLF
                } else {
                    unreachable!()
                }
            }
            RecordTerminator::Any(_) => RecordTermAny,
        }
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
        unsafe {
            ::std::mem::transmute(self.rdr.next_field().into_iter_result())
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
        if self.p.done() {
            return None;
        }
        let mut record = Vec::with_capacity(self.p.first_record.len());
        loop {
            match self.p.next_field() {
                NextField::EndOfRecord | NextField::EndOfCsv => break,
                NextField::Error(err) => return Some(Err(err)),
                NextField::Data(field) =>
                    record.push(ByteString::from_bytes(field)),
            }
        }
        Some(Ok(record))
    }
}

fn byte_record_to_utf8(record: Vec<ByteString>) -> CsvResult<Vec<String>> {
    for bytes in record.iter() {
        if let Err(err) = ::std::str::from_utf8(bytes[]) {
            return Err(Error::Decode(format!(
                "Could not decode the following bytes as UTF-8 because {}: {}",
                err.to_string(), bytes)));
        }
    }
    Ok(unsafe { ::std::mem::transmute(record) })
}
