use std::fs::File;
use std::io::{self, BufRead, Seek};
use std::marker::PhantomData;
use std::path::Path;
use std::result;

use csv_core::{
    Reader as CoreReader, ReaderBuilder as CoreReaderBuilder, Terminator,
};
use serde::de::DeserializeOwned;

use byte_record::{self, ByteRecord, Position};
use error::{Error, Result, Utf8Error};
use string_record::{self, StringRecord};

/// Builds a CSV reader with various configuration knobs.
///
/// This builder can be used to tweak the field delimiter, record terminator
/// and more. Once a CSV `Reader` is built, its configuration cannot be
/// changed.
#[derive(Debug)]
pub struct ReaderBuilder {
    capacity: usize,
    flexible: bool,
    has_headers: bool,
    /// The underlying CSV parser builder.
    ///
    /// We explicitly put this on the heap because CoreReaderBuilder embeds an
    /// entire DFA transition table, which along with other things, tallies up
    /// to almost 500 bytes on the stack.
    builder: Box<CoreReaderBuilder>,
}

impl Default for ReaderBuilder {
    fn default() -> ReaderBuilder {
        ReaderBuilder {
            capacity: 8 * (1<<10),
            flexible: false,
            has_headers: true,
            builder: Box::new(CoreReaderBuilder::default()),
        }
    }
}

impl ReaderBuilder {
    /// Create a new builder for configuring CSV parsing.
    ///
    /// To convert a builder into a reader, call one of the methods starting
    /// with `from_`.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::{ReaderBuilder, StringRecord};
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut data = "\
    ///city,country,pop
    ///Boston,United States,4628910
    ///Concord,United States,42695
    ///";
    ///     let mut rdr = ReaderBuilder::new().from_reader(data.as_bytes());
    ///
    ///     let records = rdr
    ///         .records()
    ///         .collect::<Result<Vec<StringRecord>, csv::Error>>()?;
    ///     assert_eq!(records, vec![
    ///         vec!["Boston", "United States", "4628910"],
    ///         vec!["Concord", "United States", "42695"],
    ///     ]);
    ///     Ok(())
    /// }
    /// ```
    pub fn new() -> ReaderBuilder {
        ReaderBuilder::default()
    }

    /// Build a CSV parser from this configuration that reads data from the
    /// given file path.
    ///
    /// If there was a problem opening the file at the given path, then this
    /// returns the corresponding error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::ReaderBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut rdr = ReaderBuilder::new().from_path("foo.csv")?;
    ///     for result in rdr.records() {
    ///         let record = result?;
    ///         println!("{:?}", record);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn from_path<P: AsRef<Path>>(&self, path: P) -> Result<Reader<File>> {
        Ok(Reader::new(self, File::open(path)?))
    }

    /// Build a CSV parser from this configuration that reads data from `rdr`.
    ///
    /// Note that the CSV reader is buffered automatically, so you should not
    /// wrap `rdr` in a buffered reader like `io::BufReader`.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::ReaderBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut data = "\
    ///city,country,pop
    ///Boston,United States,4628910
    ///Concord,United States,42695
    ///";
    ///     let mut rdr = ReaderBuilder::new().from_reader(data.as_bytes());
    ///     for result in rdr.records() {
    ///         let record = result?;
    ///         println!("{:?}", record);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn from_reader<R: io::Read>(&self, rdr: R) -> Reader<R> {
        Reader::new(self, rdr)
    }

    /// The field delimiter to use when parsing CSV.
    ///
    /// The default is `b','`.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::ReaderBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut data = "\
    ///city;country;pop
    ///Boston;United States;4628910
    ///";
    ///     let mut rdr = ReaderBuilder::new()
    ///         .delimiter(b';')
    ///         .from_reader(data.as_bytes());
    ///
    ///     if let Some(result) = rdr.records().next() {
    ///         let record = result?;
    ///         assert_eq!(record, vec!["Boston", "United States", "4628910"]);
    ///         Ok(())
    ///     } else {
    ///         Err(From::from("expected at least one record but got none"))
    ///     }
    /// }
    /// ```
    pub fn delimiter(&mut self, delimiter: u8) -> &mut ReaderBuilder {
        self.builder.delimiter(delimiter);
        self
    }

    /// Whether to treat the first row as a special header row.
    ///
    /// By default, the first row is treated as a special header row, which
    /// means the header is never returned by any of the record reading methods
    /// or iterators. When this is disabled (`yes` set to `false`), the first
    /// row is not treated specially.
    ///
    /// Note that the `headers` and `byte_headers` methods are unaffected by
    /// whether this is set. Those methods always return the first record.
    ///
    /// # Example
    ///
    /// This example shows what happens when `has_headers` is disabled.
    /// Namely, the first row is treated just like any other row.
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::ReaderBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut data = "\
    ///city,country,pop
    ///Boston,United States,4628910
    ///";
    ///     let mut rdr = ReaderBuilder::new()
    ///         .has_headers(false)
    ///         .from_reader(data.as_bytes());
    ///     let mut iter = rdr.records();
    ///
    ///     // Read the first record.
    ///     if let Some(result) = iter.next() {
    ///         let record = result?;
    ///         assert_eq!(record, vec!["city", "country", "pop"]);
    ///     } else {
    ///         return Err(From::from(
    ///             "expected at least two records but got none"));
    ///     }
    ///
    ///     // Read the second record.
    ///     if let Some(result) = iter.next() {
    ///         let record = result?;
    ///         assert_eq!(record, vec!["Boston", "United States", "4628910"]);
    ///     } else {
    ///         return Err(From::from(
    ///             "expected at least two records but got one"))
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn has_headers(&mut self, yes: bool) -> &mut ReaderBuilder {
        self.has_headers = yes;
        self
    }

    /// Whether the number of fields in records is allowed to change or not.
    ///
    /// When disabled (which is the default), parsing CSV data will return an
    /// error if a record is found with a number of fields different from the
    /// number of fields in a previous record.
    ///
    /// When enabled, this error checking is turned off.
    ///
    /// # Example: flexible records enabled
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::ReaderBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     // Notice that the first row is missing the population count.
    ///     let mut data = "\
    ///city,country,pop
    ///Boston,United States
    ///";
    ///     let mut rdr = ReaderBuilder::new()
    ///         .flexible(true)
    ///         .from_reader(data.as_bytes());
    ///
    ///     if let Some(result) = rdr.records().next() {
    ///         let record = result?;
    ///         assert_eq!(record, vec!["Boston", "United States"]);
    ///         Ok(())
    ///     } else {
    ///         Err(From::from("expected at least one record but got none"))
    ///     }
    /// }
    /// ```
    ///
    /// # Example: flexible records disabled
    ///
    /// This shows the error that appears when records of unequal length
    /// are found and flexible records have been disabled (which is the
    /// default).
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::ReaderBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     // Notice that the first row is missing the population count.
    ///     let mut data = "\
    ///city,country,pop
    ///Boston,United States
    ///";
    ///     let mut rdr = ReaderBuilder::new()
    ///         .flexible(false)
    ///         .from_reader(data.as_bytes());
    ///
    ///     if let Some(Err(err)) = rdr.records().next() {
    ///         match err {
    ///             csv::Error::UnequalLengths { expected_len, len, .. } => {
    ///                 // The header row has 3 fields...
    ///                 assert_eq!(expected_len, 3);
    ///                 // ... but the first row has only 2 fields.
    ///                 assert_eq!(len, 2);
    ///                 Ok(())
    ///             }
    ///             wrong => {
    ///                 Err(From::from(format!(
    ///                     "expected UnequalLengths error but got {:?}",
    ///                     wrong)))
    ///             }
    ///         }
    ///     } else {
    ///         Err(From::from(
    ///             "expected at least one errored record but got none"))
    ///     }
    /// }
    /// ```
    pub fn flexible(&mut self, yes: bool) -> &mut ReaderBuilder {
        self.flexible = yes;
        self
    }

    /// The record terminator to use when parsing CSV.
    ///
    /// A record terminator can be any single byte. The default is a special
    /// value, `Terminator::CRLF`, which treats any occurrence of `\r`, `\n`
    /// or `\r\n` as a single record terminator.
    ///
    /// # Example: `$` as a record terminator
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::{ReaderBuilder, Terminator};
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut data = "city,country,pop$Boston,United States,4628910";
    ///     let mut rdr = ReaderBuilder::new()
    ///         .terminator(Terminator::Any(b'$'))
    ///         .from_reader(data.as_bytes());
    ///
    ///     if let Some(result) = rdr.records().next() {
    ///         let record = result?;
    ///         assert_eq!(record, vec!["Boston", "United States", "4628910"]);
    ///         Ok(())
    ///     } else {
    ///         Err(From::from("expected at least one record but got none"))
    ///     }
    /// }
    /// ```
    pub fn terminator(
        &mut self,
        term: Terminator,
    ) -> &mut ReaderBuilder {
        self.builder.terminator(term);
        self
    }

    /// The quote character to use when parsing CSV.
    ///
    /// The default is `b'"'`.
    ///
    /// # Example: single quotes instead of double quotes
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::ReaderBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut data = "\
    ///city,country,pop
    ///Boston,'United States',4628910
    ///";
    ///     let mut rdr = ReaderBuilder::new()
    ///         .quote(b'\'')
    ///         .from_reader(data.as_bytes());
    ///
    ///     if let Some(result) = rdr.records().next() {
    ///         let record = result?;
    ///         assert_eq!(record, vec!["Boston", "United States", "4628910"]);
    ///         Ok(())
    ///     } else {
    ///         Err(From::from("expected at least one record but got none"))
    ///     }
    /// }
    /// ```
    pub fn quote(&mut self, quote: u8) -> &mut ReaderBuilder {
        self.builder.quote(quote);
        self
    }

    /// The escape character to use when parsing CSV.
    ///
    /// In some variants of CSV, quotes are escaped using a special escape
    /// character like `\` (instead of escaping quotes by doubling them).
    ///
    /// By default, recognizing these idiosyncratic escapes is disabled.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::ReaderBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut data = "\
    ///city,country,pop
    ///Boston,\"The \\\"United\\\" States\",4628910
    ///";
    ///     let mut rdr = ReaderBuilder::new()
    ///         .escape(Some(b'\\'))
    ///         .from_reader(data.as_bytes());
    ///
    ///     if let Some(result) = rdr.records().next() {
    ///         let record = result?;
    ///         assert_eq!(record, vec![
    ///             "Boston", "The \"United\" States", "4628910",
    ///         ]);
    ///         Ok(())
    ///     } else {
    ///         Err(From::from("expected at least one record but got none"))
    ///     }
    /// }
    /// ```
    pub fn escape(&mut self, escape: Option<u8>) -> &mut ReaderBuilder {
        self.builder.escape(escape);
        self
    }

    /// Enable double quote escapes.
    ///
    /// This is enabled by default, but it may be disabled. When disabled,
    /// doubled quotes are not interpreted as escapes.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::ReaderBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut data = "\
    ///city,country,pop
    ///Boston,\"The \"\"United\"\" States\",4628910
    ///";
    ///     let mut rdr = ReaderBuilder::new()
    ///         .double_quote(false)
    ///         .from_reader(data.as_bytes());
    ///
    ///     if let Some(result) = rdr.records().next() {
    ///         let record = result?;
    ///         assert_eq!(record, vec![
    ///             "Boston", "The \"United\"\" States\"", "4628910",
    ///         ]);
    ///         Ok(())
    ///     } else {
    ///         Err(From::from("expected at least one record but got none"))
    ///     }
    /// }
    /// ```
    pub fn double_quote(&mut self, yes: bool) -> &mut ReaderBuilder {
        self.builder.double_quote(yes);
        self
    }

    /// The comment character to use when parsing CSV.
    ///
    /// If the start of a record begins with the byte given here, then that
    /// line is ignored by the CSV parser.
    ///
    /// This is disabled by default.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::ReaderBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut data = "\
    ///city,country,pop
    ///#Concord,United States,42695
    ///Boston,United States,4628910
    ///";
    ///     let mut rdr = ReaderBuilder::new()
    ///         .comment(Some(b'#'))
    ///         .from_reader(data.as_bytes());
    ///
    ///     if let Some(result) = rdr.records().next() {
    ///         let record = result?;
    ///         assert_eq!(record, vec!["Boston", "United States", "4628910"]);
    ///         Ok(())
    ///     } else {
    ///         Err(From::from("expected at least one record but got none"))
    ///     }
    /// }
    /// ```
    pub fn comment(&mut self, comment: Option<u8>) -> &mut ReaderBuilder {
        self.builder.comment(comment);
        self
    }

    /// A convenience method for specifying a configuration to read ASCII
    /// delimited text.
    ///
    /// This sets the delimiter and record terminator to the ASCII unit
    /// separator (`\x1F`) and record separator (`\x1E`), respectively.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::ReaderBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut data = "\
    ///city\x1Fcountry\x1Fpop\x1EBoston\x1FUnited States\x1F4628910";
    ///     let mut rdr = ReaderBuilder::new()
    ///         .ascii()
    ///         .from_reader(data.as_bytes());
    ///
    ///     if let Some(result) = rdr.records().next() {
    ///         let record = result?;
    ///         assert_eq!(record, vec!["Boston", "United States", "4628910"]);
    ///         Ok(())
    ///     } else {
    ///         Err(From::from("expected at least one record but got none"))
    ///     }
    /// }
    /// ```
    pub fn ascii(&mut self) -> &mut ReaderBuilder {
        self.builder.ascii();
        self
    }

    /// Set the capacity (in bytes) of the buffer used in the CSV reader.
    /// This defaults to a reasonable setting.
    pub fn buffer_capacity(&mut self, capacity: usize) -> &mut ReaderBuilder {
        self.capacity = capacity;
        self
    }

    /// Enable or disable the NFA for parsing CSV.
    ///
    /// This is intended to be a debug option. The NFA is always slower than
    /// the DFA.
    #[doc(hidden)]
    pub fn nfa(&mut self, yes: bool) -> &mut ReaderBuilder {
        self.builder.nfa(yes);
        self
    }
}

#[derive(Debug)]
pub struct Reader<R> {
    /// The underlying CSV parser.
    ///
    /// We explicitly put this on the heap because CoreReader embeds an entire
    /// DFA transition table, which along with other things, tallies up to
    /// almost 500 bytes on the stack.
    core: Box<CoreReader>,
    /// The underlying reader.
    rdr: io::BufReader<R>,
    /// Various state tracking.
    ///
    /// There is more state embedded in the `CoreReader`.
    state: ReaderState,
}

#[derive(Debug)]
struct ReaderState {
    /// When set, this contains the first row of any parsed CSV data.
    ///
    /// This is always populated, regardless of whether `has_headers` is set.
    headers: Option<Headers>,
    /// When set, the first row of parsed CSV data is excluded from things
    /// that read records, like iterators and `read_record`.
    has_headers: bool,
    /// When set, there is no restriction on the length of records. When not
    /// set, every record must have the same number of fields, or else an error
    /// is reported.
    flexible: bool,
    /// The number of fields in the first record parsed.
    first_field_count: Option<u64>,
    /// The current position of the parser.
    ///
    /// Note that this position is only observable by callers at the start
    /// of a record. More granular positions are not supported.
    cur_pos: Position,
    /// Whether this reader has been seeked or not.
    seeked: bool,
    /// Whether the first record has been read or not.
    first: bool,
    /// Whether EOF of the underlying reader has been reached or not.
    eof: bool,
}

/// Headers encapsulates any data associated with the headers of CSV data.
///
/// The headers always correspond to the first row.
#[derive(Debug)]
struct Headers {
    /// The header, as raw bytes.
    byte_record: ByteRecord,
    /// The header, as valid UTF-8 (or a UTF-8 error).
    string_record: result::Result<StringRecord, Utf8Error>,
}

impl<R: io::Read> Reader<R> {
    /// Create a new CSV reader given a builder and a source of underlying
    /// bytes.
    fn new(builder: &ReaderBuilder, rdr: R) -> Reader<R> {
        Reader {
            core: Box::new(builder.builder.build()),
            rdr: io::BufReader::with_capacity(builder.capacity, rdr),
            state: ReaderState {
                headers: None,
                has_headers: builder.has_headers,
                flexible: builder.flexible,
                first_field_count: None,
                cur_pos: Position::new(),
                seeked: false,
                first: false,
                eof: false,
            },
        }
    }

    /// Create a new CSV parser with a default configuration for the given
    /// file path.
    ///
    /// To customize CSV parsing, use a `ReaderBuilder`.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Reader<File>> {
        ReaderBuilder::new().from_path(path)
    }

    /// Create a new CSV parser with a default configuration for the given
    /// reader.
    ///
    /// To customize CSV parsing, use a `ReaderBuilder`.
    pub fn from_reader(rdr: R) -> Reader<R> {
        ReaderBuilder::new().from_reader(rdr)
    }

    /// Returns a borrowed iterator over deserialized records.
    ///
    /// If `has_headers` is enabled, then this does not include the first
    /// record. Additionally, if `has_headers` is enabled, then deserialization
    /// uses the field names of structs.
    pub fn deserializer<D>(&mut self) -> DeserializeRecordsIter<R, D>
            where D: DeserializeOwned
    {
        DeserializeRecordsIter::new(self)
    }

    /// Returns an owned iterator over deserialized records.
    ///
    /// If `has_headers` is enabled, then this does not include the first
    /// record. Additionally, if `has_headers` is enabled, then deserialization
    /// uses the field names of structs.
    pub fn into_deserializer<D>(self) -> DeserializeRecordsIntoIter<R, D>
            where D: DeserializeOwned
    {
        DeserializeRecordsIntoIter::new(self)
    }

    /// Returns a borrowed iterator over all records as strings.
    ///
    /// If `has_headers` is enabled, then this does not include the first
    /// record.
    pub fn records(&mut self) -> StringRecordsIter<R> {
        StringRecordsIter::new(self)
    }

    /// Returns an owned iterator over all records as strings.
    ///
    /// If `has_headers` is enabled, then this does not include the first
    /// record.
    pub fn into_records(self) -> StringRecordsIntoIter<R> {
        StringRecordsIntoIter::new(self)
    }

    /// Returns a borrowed iterator over all records as raw bytes.
    ///
    /// If `has_headers` is enabled, then this does not include the first
    /// record.
    pub fn byte_records(&mut self) -> ByteRecordsIter<R> {
        ByteRecordsIter::new(self)
    }

    /// Returns an owned iterator over all records as raw bytes.
    ///
    /// If `has_headers` is enabled, then this does not include the first
    /// record.
    pub fn into_byte_records(self) -> ByteRecordsIntoIter<R> {
        ByteRecordsIntoIter::new(self)
    }

    /// Returns a reference to the first row read by this parser.
    ///
    /// If no row has been read yet, then this will force parsing of the first
    /// row.
    ///
    /// If there was a problem parsing the row or if it wasn't valid UTF-8,
    /// then this returns an error.
    ///
    /// Note that this method may be used regardless of whether `has_headers`
    /// is enabled.
    pub fn headers(&mut self) -> Result<&StringRecord> {
        if self.state.headers.is_none() {
            if self.state.seeked {
                return Err(Error::Seek);
            }
            let mut record = ByteRecord::new();
            self.read_byte_record_impl(&mut record)?;
            self.set_headers_impl(Err(record));
        }
        let headers = self.state.headers.as_ref().unwrap();
        match headers.string_record {
            Ok(ref record) => Ok(record),
            Err(ref err) => Err(Error::Utf8 {
                pos: headers.byte_record.position().map(Clone::clone),
                err: err.clone(),
            }),
        }
    }

    /// Set the headers of this CSV parser manually.
    ///
    /// This overrides any other setting (including `set_byte_headers`). Any
    /// automatic detection of headers is disabled.
    pub fn set_headers(&mut self, headers: StringRecord) {
        self.set_headers_impl(Ok(headers));
    }

    /// Returns a reference to the first row read by this parser as raw bytes.
    ///
    /// If no row has been read yet, then this will force parsing of the first
    /// row.
    ///
    /// If there was a problem parsing the row then this returns an error.
    ///
    /// Note that this method may be used regardless of whether `has_headers`
    /// is enabled.
    pub fn byte_headers(&mut self) -> Result<&ByteRecord> {
        if self.state.headers.is_none() {
            if self.state.seeked {
                return Err(Error::Seek);
            }
            let mut record = ByteRecord::new();
            self.read_byte_record_impl(&mut record)?;
            self.set_headers_impl(Err(record));
        }
        Ok(&self.state.headers.as_ref().unwrap().byte_record)
    }

    /// Set the headers of this CSV parser manually as raw bytes.
    ///
    /// This overrides any other setting (including `set_headers`). Any
    /// automatic detection of headers is disabled.
    pub fn set_byte_headers(&mut self, headers: ByteRecord) {
        self.set_headers_impl(Err(headers));
    }

    fn set_headers_impl(
        &mut self,
        headers: result::Result<StringRecord, ByteRecord>,
    ) {
        // If we have string headers, then get byte headers. But if we have
        // byte headers, then get the string headers (or a UTF-8 error).
        let (str_headers, byte_headers) = match headers {
            Ok(string) => {
                let bytes = string.clone().into_byte_record();
                (Ok(string), bytes)
            }
            Err(bytes) => {
                match StringRecord::from_byte_record(bytes.clone()) {
                    Ok(str_headers) => (Ok(str_headers), bytes),
                    Err(err) => (Err(err.utf8_error().clone()), bytes),
                }
            }
        };
        self.state.headers = Some(Headers {
            byte_record: byte_headers,
            string_record: str_headers,
        });
    }

    /// Return the current position of this CSV reader.
    ///
    /// The byte offset in the position returned can be used to `seek` this
    /// reader. In particular, seeking to a position returned here on the same
    /// data will result in parsing the same subsequent record.
    pub fn position(&self) -> &Position {
        &self.state.cur_pos
    }

    /// Returns true if and only if this reader has been exhausted.
    ///
    /// When this returns true, no more records can be read from this reader
    /// (unless it has been seeked to another position).
    pub fn is_done(&self) -> bool {
        self.state.eof
    }

    pub fn read_record(&mut self, record: &mut StringRecord) -> Result<bool> {
        string_record::read(self, record)
    }

    pub fn read_byte_record(
        &mut self,
        record: &mut ByteRecord,
    ) -> Result<bool> {
        if !self.state.has_headers && !self.state.first {
            // If the caller indicated "no headers" and we haven't yield
            // the first record yet, then we should yield our header row
            // if we have one.
            if let Some(ref headers) = self.state.headers {
                self.state.first = true;
                record.clone_from(&headers.byte_record);
                return Ok(self.state.eof);
            }
        }
        let eof = self.read_byte_record_impl(record)?;
        self.state.first = true;
        if !self.state.seeked && self.state.headers.is_none() {
            self.set_headers_impl(Err(record.clone()));
            // If the end user indicated that we have headers, then we should
            // never return the first row. Instead, we should attempt to
            // read and return the next one.
            if self.state.has_headers {
                // Since we just read the first row and will treat it as the
                // special header row, undo the record index increment.
                let i = self.state.cur_pos.record();
                self.state.cur_pos.set_record(i.checked_sub(1).unwrap());
                return self.read_byte_record_impl(record);
            }
        }
        Ok(eof)
    }

    /// Read a byte record from the underlying CSV reader, without accounting
    /// for headers.
    #[inline(always)]
    fn read_byte_record_impl(
        &mut self,
        record: &mut ByteRecord,
    ) -> Result<bool> {
        use csv_core::ReadRecordResult::*;

        record.clear();
        record.set_position(Some(self.state.cur_pos.clone()));
        if self.state.eof {
            return Ok(true);
        }
        let (mut outlen, mut endlen) = (0, 0);
        loop {
            let (res, nin, nout, nend) = {
                let input = self.rdr.fill_buf()?;
                let (mut fields, mut ends) = byte_record::as_parts(record);
                self.core.read_record(
                    input, &mut fields[outlen..], &mut ends[endlen..])
            };
            self.rdr.consume(nin);
            let byte = self.state.cur_pos.byte();
            self.state.cur_pos.set_byte(byte + nin as u64);
            self.state.cur_pos.set_line(self.core.line());
            outlen += nout;
            endlen += nend;
            match res {
                InputEmpty => continue,
                OutputFull => {
                    byte_record::expand_fields(record);
                    continue;
                }
                OutputEndsFull => {
                    byte_record::expand_ends(record);
                    continue;
                }
                Record => {
                    byte_record::set_len(record, endlen);
                    self.state.add_record(record)?;
                    break;
                }
                End => {
                    self.state.eof = true;
                    break;
                }
            }
        }
        Ok(self.state.eof)
    }
}

impl<R: io::Read + io::Seek> Reader<R> {
    /// Seeks the underlying reader to the position given.
    ///
    /// This comes with a few caveats:
    ///
    /// * If the headers of this data have not already been read, then
    ///   `byte_headers` and `headers` will always return an error after a
    ///   call to `seek`.
    /// * Any internal buffer associated with this reader is cleared.
    /// * If the given position does not correspond to a position immediately
    ///   before the start of a record, then the behavior of this reader is
    ///   unspecified.
    ///
    /// If the given position has a byte offset equivalent to the current
    /// position, then no seeking is performed.
    pub fn seek(&mut self, pos: &Position) -> Result<()> {
        if pos.byte() == self.state.cur_pos.byte() {
            return Ok(());
        }
        self.seek_raw(io::SeekFrom::Start(pos.byte()), pos)
    }

    /// This is like `seek`, but provides direct control over how the seeking
    /// operation is performed via `io::SeekFrom`.
    ///
    /// The `pos` position given *should* correspond the position indicated
    /// by `seek_from`, but there is no requirement. If the `pos` position
    /// given is incorrect, then the position information returned by this
    /// reader will be similarly incorrect.
    ///
    /// Unlike `seek`, this will always cause an actual seek to be performed.
    pub fn seek_raw(
        &mut self,
        seek_from: io::SeekFrom,
        pos: &Position,
    ) -> Result<()> {
        self.rdr.seek(seek_from)?;
        self.core.reset();
        self.core.set_line(pos.line());
        self.state.seeked = true;
        self.state.cur_pos = pos.clone();
        self.state.eof = false;
        Ok(())
    }
}

impl ReaderState {
    #[inline(always)]
    fn add_record(&mut self, record: &ByteRecord) -> Result<()> {
        let i = self.cur_pos.record();
        self.cur_pos.set_record(i.checked_add(1).unwrap());
        if !self.flexible {
            match self.first_field_count {
                None => self.first_field_count = Some(record.len() as u64),
                Some(expected) => {
                    if record.len() as u64 != expected {
                        return Err(Error::UnequalLengths {
                            pos: record.position().map(Clone::clone),
                            expected_len: expected,
                            len: record.len() as u64,
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

/// An owned iterator over deserialized records.
///
/// The type parameter `R` refers to the underlying `io::Read` type, and `D`
/// refers to the type that this iterator will deserialize a record into.
pub struct DeserializeRecordsIntoIter<R, D> {
    rdr: Reader<R>,
    rec: StringRecord,
    headers: Option<StringRecord>,
    _priv: PhantomData<D>,
}

impl<R: io::Read, D: DeserializeOwned> DeserializeRecordsIntoIter<R, D> {
    fn new(mut rdr: Reader<R>) -> DeserializeRecordsIntoIter<R, D> {
        let headers =
            if !rdr.state.has_headers {
                None
            } else {
                rdr.headers().ok().map(Clone::clone)
            };
        DeserializeRecordsIntoIter {
            rdr: rdr,
            rec: StringRecord::new(),
            headers: headers,
            _priv: PhantomData,
        }
    }

    /// Return a mutable reference to the underlying CSV reader.
    pub fn reader(&mut self) -> &mut Reader<R> {
        &mut self.rdr
    }
}

impl<R: io::Read, D: DeserializeOwned>
    Iterator for DeserializeRecordsIntoIter<R, D>
{
    type Item = Result<D>;

    fn next(&mut self) -> Option<Result<D>> {
        match self.rdr.read_record(&mut self.rec) {
            Err(err) => Some(Err(err)),
            Ok(true) => None,
            Ok(false) => Some(self.rec.deserialize(self.headers.as_ref())),
        }
    }
}

/// A borrowed iterator over deserialized records.
///
/// The lifetime parameter `'r` refers to the lifetime of the underlying
/// CSV `Reader`. The type parameter `R` refers to the underlying `io::Read`
/// type, and `D` refers to the type that this iterator will deserialize a
/// record into.
pub struct DeserializeRecordsIter<'r, R: 'r, D> {
    rdr: &'r mut Reader<R>,
    rec: StringRecord,
    headers: Option<StringRecord>,
    _priv: PhantomData<D>,
}

impl<'r, R: io::Read, D: DeserializeOwned> DeserializeRecordsIter<'r, R, D> {
    fn new(rdr: &'r mut Reader<R>) -> DeserializeRecordsIter<'r, R, D> {
        let headers =
            if !rdr.state.has_headers {
                None
            } else {
                rdr.headers().ok().map(Clone::clone)
            };
        DeserializeRecordsIter {
            rdr: rdr,
            rec: StringRecord::new(),
            headers: headers,
            _priv: PhantomData,
        }
    }

    /// Return a mutable reference to the underlying CSV reader.
    pub fn reader(&mut self) -> &mut Reader<R> {
        self.rdr
    }
}

impl<'r, R: io::Read, D: DeserializeOwned>
    Iterator for DeserializeRecordsIter<'r, R, D>
{
    type Item = Result<D>;

    fn next(&mut self) -> Option<Result<D>> {
        match self.rdr.read_record(&mut self.rec) {
            Err(err) => Some(Err(err)),
            Ok(true) => None,
            Ok(false) => Some(self.rec.deserialize(self.headers.as_ref())),
        }
    }
}

/// An owned iterator over records as strings.
pub struct StringRecordsIntoIter<R> {
    rdr: Reader<R>,
    rec: StringRecord,
}

impl<R: io::Read> StringRecordsIntoIter<R> {
    fn new(rdr: Reader<R>) -> StringRecordsIntoIter<R> {
        StringRecordsIntoIter { rdr: rdr, rec: StringRecord::new() }
    }

    /// Return a mutable reference to the underlying CSV reader.
    pub fn reader(&mut self) -> &mut Reader<R> {
        &mut self.rdr
    }
}

impl<R: io::Read> Iterator for StringRecordsIntoIter<R> {
    type Item = Result<StringRecord>;

    fn next(&mut self) -> Option<Result<StringRecord>> {
        match self.rdr.read_record(&mut self.rec) {
            Err(err) => Some(Err(err)),
            Ok(false) => Some(Ok(self.rec.clone())),
            Ok(true) => None,
        }
    }
}

/// A borrowed iterator over records as strings.
///
/// The lifetime parameter `'r` refers to the lifetime of the underlying
/// CSV `Reader`.
pub struct StringRecordsIter<'r, R: 'r> {
    rdr: &'r mut Reader<R>,
    rec: StringRecord,
}

impl<'r, R: io::Read> StringRecordsIter<'r, R> {
    fn new(rdr: &'r mut Reader<R>) -> StringRecordsIter<'r, R> {
        StringRecordsIter { rdr: rdr, rec: StringRecord::new() }
    }

    /// Return a mutable reference to the underlying CSV reader.
    pub fn reader(&mut self) -> &mut Reader<R> {
        self.rdr
    }
}

impl<'r, R: io::Read> Iterator for StringRecordsIter<'r, R> {
    type Item = Result<StringRecord>;

    fn next(&mut self) -> Option<Result<StringRecord>> {
        match self.rdr.read_record(&mut self.rec) {
            Err(err) => Some(Err(err)),
            Ok(false) => Some(Ok(self.rec.clone())),
            Ok(true) => None,
        }
    }
}

/// An owned iterator over records as raw bytes.
pub struct ByteRecordsIntoIter<R> {
    rdr: Reader<R>,
    rec: ByteRecord,
}

impl<R: io::Read> ByteRecordsIntoIter<R> {
    fn new(rdr: Reader<R>) -> ByteRecordsIntoIter<R> {
        ByteRecordsIntoIter { rdr: rdr, rec: ByteRecord::new() }
    }

    /// Return a mutable reference to the underlying CSV reader.
    pub fn reader(&mut self) -> &mut Reader<R> {
        &mut self.rdr
    }
}

impl<R: io::Read> Iterator for ByteRecordsIntoIter<R> {
    type Item = Result<ByteRecord>;

    fn next(&mut self) -> Option<Result<ByteRecord>> {
        match self.rdr.read_byte_record(&mut self.rec) {
            Err(err) => Some(Err(err)),
            Ok(false) => Some(Ok(self.rec.clone())),
            Ok(true) => None,
        }
    }
}

/// A borrowed iterator over records as raw bytes.
///
/// The lifetime parameter `'r` refers to the lifetime of the underlying
/// CSV `Reader`.
pub struct ByteRecordsIter<'r, R: 'r> {
    rdr: &'r mut Reader<R>,
    rec: ByteRecord,
}

impl<'r, R: io::Read> ByteRecordsIter<'r, R> {
    fn new(rdr: &'r mut Reader<R>) -> ByteRecordsIter<'r, R> {
        ByteRecordsIter { rdr: rdr, rec: ByteRecord::new() }
    }

    /// Return a mutable reference to the underlying CSV reader.
    pub fn reader(&mut self) -> &mut Reader<R> {
        self.rdr
    }
}

impl<'r, R: io::Read> Iterator for ByteRecordsIter<'r, R> {
    type Item = Result<ByteRecord>;

    fn next(&mut self) -> Option<Result<ByteRecord>> {
        match self.rdr.read_byte_record(&mut self.rec) {
            Err(err) => Some(Err(err)),
            Ok(false) => Some(Ok(self.rec.clone())),
            Ok(true) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use byte_record::ByteRecord;
    use error::Error;
    use string_record::StringRecord;

    use super::{ReaderBuilder, Position};

    fn b(s: &str) -> &[u8] { s.as_bytes() }
    fn s(b: &[u8]) -> &str { ::std::str::from_utf8(b).unwrap() }

    fn newpos(byte: u64, line: u64, record: u64) -> Position {
        let mut p = Position::new();
        p.set_byte(byte);
        p.set_line(line);
        p.set_record(record);
        p
    }

    macro_rules! assert_match {
        ($e:expr, $p:pat) => {{
            match $e {
                $p => {}
                e => panic!("match failed, got {:?}", e),
            }
        }}
    }

    #[test]
    fn read_byte_record() {
        let data = b("foo,\"b,ar\",baz\nabc,mno,xyz");
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(data);
        let mut rec = ByteRecord::new();

        assert!(!rdr.read_byte_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("foo", s(&rec[0]));
        assert_eq!("b,ar", s(&rec[1]));
        assert_eq!("baz", s(&rec[2]));

        assert!(!rdr.read_byte_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("abc", s(&rec[0]));
        assert_eq!("mno", s(&rec[1]));
        assert_eq!("xyz", s(&rec[2]));

        assert!(rdr.read_byte_record(&mut rec).unwrap());
    }

    #[test]
    fn read_record_unequal_fails() {
        let data = b("foo\nbar,baz");
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(data);
        let mut rec = ByteRecord::new();

        assert!(!rdr.read_byte_record(&mut rec).unwrap());
        assert_eq!(1, rec.len());
        assert_eq!("foo", s(&rec[0]));

        match rdr.read_byte_record(&mut rec) {
            Err(Error::UnequalLengths {
                expected_len: 1,
                pos,
                len: 2,
            }) => {
                assert_eq!(pos, Some(newpos(4, 2, 1)));
            }
            wrong => panic!("match failed, got {:?}", wrong),
        }
    }

    #[test]
    fn read_record_unequal_ok() {
        let data = b("foo\nbar,baz");
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(data);
        let mut rec = ByteRecord::new();

        assert!(!rdr.read_byte_record(&mut rec).unwrap());
        assert_eq!(1, rec.len());
        assert_eq!("foo", s(&rec[0]));

        assert!(!rdr.read_byte_record(&mut rec).unwrap());
        assert_eq!(2, rec.len());
        assert_eq!("bar", s(&rec[0]));
        assert_eq!("baz", s(&rec[1]));

        assert!(rdr.read_byte_record(&mut rec).unwrap());
    }

    // This tests that even if we get a CSV error, we can continue reading
    // if we want.
    #[test]
    fn read_record_unequal_continue() {
        let data = b("foo\nbar,baz\nquux");
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(data);
        let mut rec = ByteRecord::new();

        assert!(!rdr.read_byte_record(&mut rec).unwrap());
        assert_eq!(1, rec.len());
        assert_eq!("foo", s(&rec[0]));

        match rdr.read_byte_record(&mut rec) {
            Err(Error::UnequalLengths {
                expected_len: 1,
                pos,
                len: 2,
            }) => {
                assert_eq!(pos, Some(newpos(4, 2, 1)));
            }
            wrong => panic!("match failed, got {:?}", wrong),
        }

        assert!(!rdr.read_byte_record(&mut rec).unwrap());
        assert_eq!(1, rec.len());
        assert_eq!("quux", s(&rec[0]));

        assert!(rdr.read_byte_record(&mut rec).unwrap());
    }

    #[test]
    fn read_record_headers() {
        let data = b("foo,bar,baz\na,b,c\nd,e,f");
        let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(data);
        let mut rec = StringRecord::new();

        assert!(!rdr.read_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("a", &rec[0]);

        assert!(!rdr.read_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("d", &rec[0]);

        assert!(rdr.read_record(&mut rec).unwrap());

        {
            let headers = rdr.byte_headers().unwrap();
            assert_eq!(3, headers.len());
            assert_eq!(b"foo", &headers[0]);
            assert_eq!(b"bar", &headers[1]);
            assert_eq!(b"baz", &headers[2]);
        }
        {
            let headers = rdr.headers().unwrap();
            assert_eq!(3, headers.len());
            assert_eq!("foo", &headers[0]);
            assert_eq!("bar", &headers[1]);
            assert_eq!("baz", &headers[2]);
        }
    }

    #[test]
    fn read_record_headers_invalid_utf8() {
        let data = &b"foo,b\xFFar,baz\na,b,c\nd,e,f"[..];
        let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(data);
        let mut rec = StringRecord::new();

        assert!(!rdr.read_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("a", &rec[0]);

        assert!(!rdr.read_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("d", &rec[0]);

        assert!(rdr.read_record(&mut rec).unwrap());

        // Check that we can read the headers as raw bytes, but that
        // if we read them as strings, we get an appropriate UTF-8 error.
        {
            let headers = rdr.byte_headers().unwrap();
            assert_eq!(3, headers.len());
            assert_eq!(b"foo", &headers[0]);
            assert_eq!(b"b\xFFar", &headers[1]);
            assert_eq!(b"baz", &headers[2]);
        }
        match rdr.headers().unwrap_err() {
            Error::Utf8 { pos: Some(pos), err } => {
                assert_eq!(pos, newpos(0, 1, 0));
                assert_eq!(err.field(), 1);
                assert_eq!(err.valid_up_to(), 1);
            }
            err => panic!("match failed, got {:?}", err),
        }
    }

    #[test]
    fn read_record_no_headers_before() {
        let data = b("foo,bar,baz\na,b,c\nd,e,f");
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(data);
        let mut rec = StringRecord::new();

        {
            let headers = rdr.headers().unwrap();
            assert_eq!(3, headers.len());
            assert_eq!("foo", &headers[0]);
            assert_eq!("bar", &headers[1]);
            assert_eq!("baz", &headers[2]);
        }

        assert!(!rdr.read_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("foo", &rec[0]);

        assert!(!rdr.read_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("a", &rec[0]);

        assert!(!rdr.read_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("d", &rec[0]);

        assert!(rdr.read_record(&mut rec).unwrap());
    }

    #[test]
    fn read_record_no_headers_after() {
        let data = b("foo,bar,baz\na,b,c\nd,e,f");
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(data);
        let mut rec = StringRecord::new();

        assert!(!rdr.read_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("foo", &rec[0]);

        assert!(!rdr.read_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("a", &rec[0]);

        assert!(!rdr.read_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("d", &rec[0]);

        assert!(rdr.read_record(&mut rec).unwrap());

        let headers = rdr.headers().unwrap();
        assert_eq!(3, headers.len());
        assert_eq!("foo", &headers[0]);
        assert_eq!("bar", &headers[1]);
        assert_eq!("baz", &headers[2]);
    }

    #[test]
    fn seek() {
        let data = b("foo,bar,baz\na,b,c\nd,e,f\ng,h,i");
        let mut rdr = ReaderBuilder::new()
            .from_reader(io::Cursor::new(data));
        rdr.seek(&newpos(18, 3, 2)).unwrap();

        let mut rec = StringRecord::new();

        assert_eq!(18, rdr.position().byte());
        assert!(!rdr.read_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("d", &rec[0]);

        assert_eq!(24, rdr.position().byte());
        assert_eq!(4, rdr.position().line());
        assert_eq!(3, rdr.position().record());
        assert!(!rdr.read_record(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("g", &rec[0]);

        assert!(rdr.read_record(&mut rec).unwrap());
    }

    // Test that asking for headers after a seek returns an error if the
    // headers weren't read before seeking.
    #[test]
    fn seek_headers_error() {
        let data = b("foo,bar,baz\na,b,c\nd,e,f\ng,h,i");
        let mut rdr = ReaderBuilder::new()
            .from_reader(io::Cursor::new(data));
        rdr.seek(&newpos(18, 3, 2)).unwrap();
        assert_match!(rdr.headers(), Err(Error::Seek));
    }

    // Test that we can read headers after seeking if the headers were read
    // before seeking.
    #[test]
    fn seek_headers() {
        let data = b("foo,bar,baz\na,b,c\nd,e,f\ng,h,i");
        let mut rdr = ReaderBuilder::new()
            .from_reader(io::Cursor::new(data));
        let headers = rdr.headers().unwrap().clone();
        rdr.seek(&newpos(18, 3, 2)).unwrap();
        assert_eq!(&headers, rdr.headers().unwrap());
    }

    // Test that even if we didn't read headers before seeking, if we seek to
    // the current byte offset, then no seeking is done and therefore we can
    // still read headers after seeking.
    #[test]
    fn seek_headers_no_actual_seek() {
        let data = b("foo,bar,baz\na,b,c\nd,e,f\ng,h,i");
        let mut rdr = ReaderBuilder::new()
            .from_reader(io::Cursor::new(data));
        rdr.seek(&Position::new()).unwrap();
        assert_eq!("foo", &rdr.headers().unwrap()[0]);
    }

    // Test that position info is reported correctly in absence of headers.
    #[test]
    fn positions_no_headers() {
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_reader("a,b,c\nx,y,z".as_bytes())
            .into_records();

        let pos = rdr.next().unwrap().unwrap().position().unwrap().clone();
        assert_eq!(pos.byte(), 0);
        assert_eq!(pos.line(), 1);
        assert_eq!(pos.record(), 0);

        let pos = rdr.next().unwrap().unwrap().position().unwrap().clone();
        assert_eq!(pos.byte(), 6);
        assert_eq!(pos.line(), 2);
        assert_eq!(pos.record(), 1);
    }

    // Test that position info is reported correctly with headers.
    #[test]
    fn positions_headers() {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader("a,b,c\nx,y,z".as_bytes())
            .into_records();

        let pos = rdr.next().unwrap().unwrap().position().unwrap().clone();
        assert_eq!(pos.byte(), 6);
        assert_eq!(pos.line(), 2);
        assert_eq!(pos.record(), 0);
    }
}
