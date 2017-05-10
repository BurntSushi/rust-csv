use std::fs::File;
use std::io;
use std::path::Path;
use std::result;

use csv_core::{
    Writer as CoreWriter, WriterBuilder as CoreWriterBuilder,
    QuoteStyle, Terminator, WriteResult,
};
use serde::Serialize;

use error::{Error, Result, IntoInnerError, new_into_inner_error};
use serializer::serialize;

/// Builds a CSV writer with various configuration knobs.
///
/// This builder can be used to tweak the field delimiter, record terminator
/// and more. Once a CSV `Writer` is built, its configuration cannot be
/// changed.
#[derive(Debug)]
pub struct WriterBuilder {
    builder: CoreWriterBuilder,
    capacity: usize,
    flexible: bool,
    has_headers: bool,
}

impl Default for WriterBuilder {
    fn default() -> WriterBuilder {
        WriterBuilder {
            builder: CoreWriterBuilder::default(),
            capacity: 8 * (1<<10),
            flexible: false,
            has_headers: true,
        }
    }
}

impl WriterBuilder {
    /// Create a new builder for configuring CSV writing.
    ///
    /// To convert a builder into a writer, call one of the methods starting
    /// with `from_`.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::WriterBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new().from_writer(vec![]);
    ///     wtr.write_record(&["a", "b", "c"])?;
    ///     wtr.write_record(&["x", "y", "z"])?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "a,b,c\nx,y,z\n");
    ///     Ok(())
    /// }
    /// ```
    pub fn new() -> WriterBuilder {
        WriterBuilder::default()
    }

    /// Build a CSV writer from this configuration that writes data to the
    /// given file path. The file is truncated if it already exists.
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
    /// use csv::WriterBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new().from_path("foo.csv")?;
    ///     wtr.write_record(&["a", "b", "c"])?;
    ///     wtr.write_record(&["x", "y", "z"])?;
    ///     wtr.flush()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn from_path<P: AsRef<Path>>(&self, path: P) -> Result<Writer<File>> {
        Ok(Writer::new(self, File::create(path)?))
    }

    /// Build a CSV writer from this configuration that writes data to `wtr`.
    ///
    /// Note that the CSV writer is buffered automatically, so you should not
    /// wrap `wtr` in a buffered writer like `io::BufWriter`.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::WriterBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new().from_writer(vec![]);
    ///     wtr.write_record(&["a", "b", "c"])?;
    ///     wtr.write_record(&["x", "y", "z"])?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "a,b,c\nx,y,z\n");
    ///     Ok(())
    /// }
    /// ```
    pub fn from_writer<W: io::Write>(&self, wtr: W) -> Writer<W> {
        Writer::new(self, wtr)
    }

    /// The field delimiter to use when writing CSV.
    ///
    /// The default is `b','`.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::WriterBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new()
    ///         .delimiter(b';')
    ///         .from_writer(vec![]);
    ///     wtr.write_record(&["a", "b", "c"])?;
    ///     wtr.write_record(&["x", "y", "z"])?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "a;b;c\nx;y;z\n");
    ///     Ok(())
    /// }
    /// ```
    pub fn delimiter(&mut self, delimiter: u8) -> &mut WriterBuilder {
        self.builder.delimiter(delimiter);
        self
    }

    /// Whether to write a header row before writing any other row.
    ///
    /// When this is enabled and the `serialize` method is used to write data
    /// with something that contains field names (i.e., a struct), then a
    /// header row is written containing the field names before any other row
    /// is written.
    ///
    /// This option has no effect when using other methods to write rows. That
    /// is, if you don't use `serialize`, then you must write your header row
    /// explicitly if you want a header row.
    ///
    /// This is enabled by default.
    ///
    /// # Example: with headers
    ///
    /// This shows how the header will be automatically written from the field
    /// names of a struct.
    ///
    /// ```
    /// extern crate csv;
    /// #[macro_use]
    /// extern crate serde_derive;
    ///
    /// use std::error::Error;
    /// use csv::WriterBuilder;
    ///
    /// #[derive(Serialize)]
    /// struct Row<'a> {
    ///     city: &'a str,
    ///     country: &'a str,
    ///     // Serde allows us to name our headers exactly,
    ///     // even if they don't match our struct field names.
    ///     #[serde(rename = "popcount")]
    ///     population: u64,
    /// }
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new().from_writer(vec![]);
    ///     wtr.serialize(Row {
    ///         city: "Boston",
    ///         country: "United States",
    ///         population: 4628910,
    ///     })?;
    ///     wtr.serialize(Row {
    ///         city: "Concord",
    ///         country: "United States",
    ///         population: 42695,
    ///     })?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "\
    ///city,country,popcount
    ///Boston,United States,4628910
    ///Concord,United States,42695
    ///");
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Example: without headers
    ///
    /// This shows that serializing things that aren't structs (in this case,
    /// a tuple struct) won't result in a header row being written. This means
    /// you usually don't need to set `has_headers(false)` unless you
    /// explicitly want to both write custom headers and serialize structs.
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::WriterBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new().from_writer(vec![]);
    ///     wtr.serialize(("Boston", "United States", 4628910))?;
    ///     wtr.serialize(("Concord", "United States", 42695))?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "\
    ///Boston,United States,4628910
    ///Concord,United States,42695
    ///");
    ///     Ok(())
    /// }
    /// ```
    pub fn has_headers(&mut self, yes: bool) -> &mut WriterBuilder {
        self.has_headers = yes;
        self
    }

    /// Whether the number of fields in records is allowed to change or not.
    ///
    /// When disabled (which is the default), writing CSV data will return an
    /// error if a record is written with a number of fields different from the
    /// number of fields written in a previous record.
    ///
    /// When enabled, this error checking is turned off.
    ///
    /// # Example: writing flexible records
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::WriterBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new()
    ///         .flexible(true)
    ///         .from_writer(vec![]);
    ///     wtr.write_record(&["a", "b"])?;
    ///     wtr.write_record(&["x", "y", "z"])?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "a,b\nx,y,z\n");
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Example: error when `flexible` is disabled
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::WriterBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new()
    ///         .flexible(false)
    ///         .from_writer(vec![]);
    ///     wtr.write_record(&["a", "b"])?;
    ///     let err = wtr.write_record(&["x", "y", "z"]).unwrap_err();
    ///     match err {
    ///         csv::Error::UnequalLengths { expected_len, len, .. } => {
    ///             assert_eq!(expected_len, 2);
    ///             assert_eq!(len, 3);
    ///         }
    ///         wrong => panic!("expected UnequalLengths but got {:?}", wrong),
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn flexible(&mut self, yes: bool) -> &mut WriterBuilder {
        self.flexible = yes;
        self
    }

    /// The record terminator to use when writing CSV.
    ///
    /// A record terminator can be any single byte. The default is a special
    /// value, `Terminator::CRLF`, which uses `\r\n` as the record terminator.
    ///
    /// The default is `b'\n'`.
    ///
    /// # Example: CRLF
    ///
    /// This shows how to use RFC 4180 compliant record terminators.
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::{Terminator, WriterBuilder};
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new()
    ///         .terminator(Terminator::CRLF)
    ///         .from_writer(vec![]);
    ///     wtr.write_record(&["a", "b", "c"])?;
    ///     wtr.write_record(&["x", "y", "z"])?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "a,b,c\r\nx,y,z\r\n");
    ///     Ok(())
    /// }
    /// ```
    pub fn terminator(
        &mut self,
        term: Terminator,
    ) -> &mut WriterBuilder {
        self.builder.terminator(term);
        self
    }

    /// The quoting style to use when writing CSV.
    ///
    /// By default, this is set to `QuoteStyle::Necessary`, which will only
    /// use quotes when they are necessary to preserve the integrity of data.
    ///
    /// Note that unless the quote style is set to `Never`, an empty field is
    /// quoted if it is the only field in a record.
    ///
    /// # Example: non-numeric quoting
    ///
    /// This shows how to quote non-numeric fields only.
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::{QuoteStyle, WriterBuilder};
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new()
    ///         .quote_style(QuoteStyle::NonNumeric)
    ///         .from_writer(vec![]);
    ///     wtr.write_record(&["a", "5", "c"])?;
    ///     wtr.write_record(&["3.14", "y", "z"])?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "\"a\",5,\"c\"\n3.14,\"y\",\"z\"\n");
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Example: never quote
    ///
    /// This shows how the CSV writer can be made to never write quotes, even
    /// if it sacrifices the integrity of the data.
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::{QuoteStyle, WriterBuilder};
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new()
    ///         .quote_style(QuoteStyle::Never)
    ///         .from_writer(vec![]);
    ///     wtr.write_record(&["a", "foo\nbar", "c"])?;
    ///     wtr.write_record(&["g\"h\"i", "y", "z"])?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "a,foo\nbar,c\ng\"h\"i,y,z\n");
    ///     Ok(())
    /// }
    /// ```
    pub fn quote_style(&mut self, style: QuoteStyle) -> &mut WriterBuilder {
        self.builder.quote_style(style);
        self
    }

    /// The quote character to use when writing CSV.
    ///
    /// The default is `b'"'`.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::WriterBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new()
    ///         .quote(b'\'')
    ///         .from_writer(vec![]);
    ///     wtr.write_record(&["a", "foo\nbar", "c"])?;
    ///     wtr.write_record(&["g'h'i", "y\"y\"y", "z"])?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "a,'foo\nbar',c\n'g''h''i',y\"y\"y,z\n");
    ///     Ok(())
    /// }
    /// ```
    pub fn quote(&mut self, quote: u8) -> &mut WriterBuilder {
        self.builder.quote(quote);
        self
    }

    /// Enable double quote escapes.
    ///
    /// This is enabled by default, but it may be disabled. When disabled,
    /// quotes in field data are escaped instead of doubled.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::WriterBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new()
    ///         .double_quote(false)
    ///         .from_writer(vec![]);
    ///     wtr.write_record(&["a", "foo\"bar", "c"])?;
    ///     wtr.write_record(&["x", "y", "z"])?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "a,\"foo\\\"bar\",c\nx,y,z\n");
    ///     Ok(())
    /// }
    /// ```
    pub fn double_quote(&mut self, yes: bool) -> &mut WriterBuilder {
        self.builder.double_quote(yes);
        self
    }

    /// The escape character to use when writing CSV.
    ///
    /// In some variants of CSV, quotes are escaped using a special escape
    /// character like `\` (instead of escaping quotes by doubling them).
    ///
    /// By default, writing these idiosyncratic escapes is disabled, and is
    /// only used when `double_quote` is disabled.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::WriterBuilder;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new()
    ///         .double_quote(false)
    ///         .escape(b'$')
    ///         .from_writer(vec![]);
    ///     wtr.write_record(&["a", "foo\"bar", "c"])?;
    ///     wtr.write_record(&["x", "y", "z"])?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "a,\"foo$\"bar\",c\nx,y,z\n");
    ///     Ok(())
    /// }
    /// ```
    pub fn escape(&mut self, escape: u8) -> &mut WriterBuilder {
        self.builder.escape(escape);
        self
    }

    /// Set the capacity (in bytes) of the internal buffer used in the CSV
    /// writer. This defaults to a reasonable setting.
    pub fn buffer_capacity(&mut self, capacity: usize) -> &mut WriterBuilder {
        self.capacity = capacity;
        self
    }
}

/// A already configured CSV writer.
///
/// A CSV writer takes as input Rust values and writes those values in a valid
/// CSV format as output.
///
/// While CSV writing is considerably easier than parsing CSV, a proper writer
/// will do a number of things for you:
///
/// 1. Quote fields when necessary.
/// 2. Check that all records have the same number of fields.
/// 3. Write records with a single empty field correctly.
/// 4. Automatically serialize normal Rust types to CSV records. When that
///    type is a struct, a header row is automatically written corresponding
///    to the fields of that struct.
/// 5. Use buffering intelligently and otherwise avoid allocation. (This means
///    that callers should not do their own buffering.)
///
/// All of the above can be configured using a
/// [`WriterBuilder`](struct.WriterBuilder.html).
/// However, a `Writer` has a couple of convenience constructors (`from_path`
/// and `from_writer`) that use the default configuration.
#[derive(Debug)]
pub struct Writer<W: io::Write> {
    core: CoreWriter,
    wtr: Option<W>,
    buf: Buffer,
    state: WriterState,
}

#[derive(Debug)]
struct WriterState {
    /// Whether the Serde serializer should attempt to write a header row.
    header: HeaderState,
    /// Whether inconsistent record lengths are allowed.
    flexible: bool,
    /// The number of fields writtein in the first record. This is compared
    /// with `fields_written` on all subsequent records to check for
    /// inconsistent record lengths.
    first_field_count: Option<u64>,
    /// The number of fields written in this record. This is used to report
    /// errors for inconsistent record lengths if `flexible` is disabled.
    fields_written: u64,
    /// This is set immediately before flushing the buffer and then unset
    /// immediately after flushing the buffer. This avoids flushing the buffer
    /// twice if the inner writer panics.
    panicked: bool,
}

/// HeaderState encodes a small state machine for handling header writes.
#[derive(Debug)]
enum HeaderState {
    /// Indicates that we should attempt to write a header.
    Write,
    /// Indicates that writing a header was attempt, and a header was written.
    DidWrite,
    /// Indicates that writing a header was attempted, but no headers were
    /// written.
    DidNotWrite,
    /// This state is used when headers are disabled. It cannot transition
    /// to any other state.
    None,
}

/// A simple internal buffer for buffering writes.
///
/// We need this because the `csv_core` APIs want to write into a `&mut [u8]`,
/// which is not available with the `std::io::BufWriter` API.
#[derive(Debug)]
struct Buffer {
    /// The contents of the buffer.
    buf: Vec<u8>,
    /// The number of bytes written to the buffer.
    len: usize,
}

impl<W: io::Write> Drop for Writer<W> {
    fn drop(&mut self) {
        if self.wtr.is_some() && !self.state.panicked {
            let _ = self.flush();
        }
    }
}

impl Writer<File> {
    /// Build a CSV writer with a default configuration that writes data to the
    /// given file path. The file is truncated if it already exists.
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
    /// use csv::Writer;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = Writer::from_path("foo.csv")?;
    ///     wtr.write_record(&["a", "b", "c"])?;
    ///     wtr.write_record(&["x", "y", "z"])?;
    ///     wtr.flush()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Writer<File>> {
        WriterBuilder::new().from_path(path)
    }
}

impl<W: io::Write> Writer<W> {
    fn new(builder: &WriterBuilder, wtr: W) -> Writer<W> {
        let header_state =
            if builder.has_headers {
                HeaderState::Write
            } else {
                HeaderState::None
            };
        Writer {
            core: builder.builder.build(),
            wtr: Some(wtr),
            buf: Buffer {
                buf: vec![0; builder.capacity],
                len: 0,
            },
            state: WriterState {
                header: header_state,
                flexible: builder.flexible,
                first_field_count: None,
                fields_written: 0,
                panicked: false,
            },
        }
    }

    /// Build a CSV writer with a default configuration that writes data to
    /// `wtr`.
    ///
    /// Note that the CSV writer is buffered automatically, so you should not
    /// wrap `wtr` in a buffered writer like `io::BufWriter`.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::Writer;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = Writer::from_writer(vec![]);
    ///     wtr.write_record(&["a", "b", "c"])?;
    ///     wtr.write_record(&["x", "y", "z"])?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "a,b,c\nx,y,z\n");
    ///     Ok(())
    /// }
    /// ```
    pub fn from_writer(wtr: W) -> Writer<W> {
        WriterBuilder::new().from_writer(wtr)
    }

    /// Serialize a single record using Serde.
    ///
    /// # Example
    ///
    /// This shows how to serialize normal Rust structs as CSV records. The
    /// fields of the struct are used to write a header row automatically.
    /// (Writing the header row automatically can be disabled by building the
    /// CSV writer with a [`WriterBuilder`](struct.WriterBuilder.html) and
    /// calling the `has_headers` method.)
    ///
    /// ```
    /// extern crate csv;
    /// #[macro_use]
    /// extern crate serde_derive;
    ///
    /// use std::error::Error;
    /// use csv::Writer;
    ///
    /// #[derive(Serialize)]
    /// struct Row<'a> {
    ///     city: &'a str,
    ///     country: &'a str,
    ///     // Serde allows us to name our headers exactly,
    ///     // even if they don't match our struct field names.
    ///     #[serde(rename = "popcount")]
    ///     population: u64,
    /// }
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = Writer::from_writer(vec![]);
    ///     wtr.serialize(Row {
    ///         city: "Boston",
    ///         country: "United States",
    ///         population: 4628910,
    ///     })?;
    ///     wtr.serialize(Row {
    ///         city: "Concord",
    ///         country: "United States",
    ///         population: 42695,
    ///     })?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "\
    ///city,country,popcount
    ///Boston,United States,4628910
    ///Concord,United States,42695
    ///");
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Rules
    ///
    /// For the most part, any Rust type that maps straight-forwardly to a CSV
    /// record is supported. This includes structs, tuples and tuple structs.
    /// Other Rust types, such as `Vec`s, arrays, maps and enums have a more
    /// complicated story. In general, when working with CSV data, one should
    /// avoid *nested sequences* as much as possible.
    ///
    /// Structs, tuples and tuple structs map to CSV records in a simple way.
    /// Tuples and tuple structs encode their fields in the order that they
    /// are defined. Structs will do the same only if `has_headers` has been
    /// disabled using [`WriterBuilder`](struct.WriterBuilder.html).
    ///
    /// Nested sequences are supported in a limited capacity. Namely, they
    /// are flattened only when headers are not being written automatically.
    /// For example:
    ///
    /// ```
    /// extern crate csv;
    /// #[macro_use]
    /// extern crate serde_derive;
    ///
    /// use std::error::Error;
    /// use csv::WriterBuilder;
    ///
    /// #[derive(Serialize)]
    /// struct Row {
    ///     label: String,
    ///     values: Vec<f64>,
    /// }
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = WriterBuilder::new()
    ///         .has_headers(false)
    ///         .from_writer(vec![]);
    ///     wtr.serialize(Row {
    ///         label: "foo".to_string(),
    ///         values: vec![1.1234, 2.5678, 3.14],
    ///     })?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "\
    ///foo,1.1234,2.5678,3.14
    ///");
    ///     Ok(())
    /// }
    /// ```
    ///
    /// If `has_headers` were enabled in the above example, then serialization
    /// would return an error. This applies to all forms of nested composite
    /// types because there's no obvious way to write headers that are in
    /// correspondence with the records.
    ///
    /// Simple enums in Rust can be serialized. Namely, enums must either be
    /// variants with no arguments or variants with a single argument. For
    /// example, to serialize a field from either an integer or a float type,
    /// one can do this:
    ///
    /// ```
    /// extern crate csv;
    /// #[macro_use]
    /// extern crate serde_derive;
    ///
    /// use std::error::Error;
    /// use csv::Writer;
    ///
    /// #[derive(Serialize)]
    /// struct Row {
    ///     label: String,
    ///     value: Value,
    /// }
    ///
    /// #[derive(Serialize)]
    /// enum Value {
    ///     Integer(i64),
    ///     Float(f64),
    /// }
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = Writer::from_writer(vec![]);
    ///     wtr.serialize(Row {
    ///         label: "foo".to_string(),
    ///         value: Value::Integer(3),
    ///     })?;
    ///     wtr.serialize(Row {
    ///         label: "bar".to_string(),
    ///         value: Value::Float(3.14),
    ///     })?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "\
    ///label,value
    ///foo,3
    ///bar,3.14
    ///");
    ///     Ok(())
    /// }
    /// ```
    pub fn serialize<S: Serialize>(&mut self, mut record: S) -> Result<()> {
        match self.state.header {
            HeaderState::None | HeaderState::DidNotWrite => {
                serialize(self, record, false, false)?;
                self.write_terminator()?;
            }
            HeaderState::DidWrite => {
                serialize(self, record, false, true)?;
                self.write_terminator()?;
            }
            HeaderState::Write => {
                let did = serialize(self, &mut record, true, false)?;
                self.state.header =
                    if did {
                        HeaderState::DidWrite
                    } else {
                        HeaderState::DidNotWrite
                    };
                self.write_terminator()?;
                if did {
                    serialize(self, record, false, true)?;
                    self.write_terminator()?;
                }
            }
        }
        Ok(())
    }

    /// Write a single record.
    ///
    /// This method accepts something that can be turned into an iterator that
    /// yields elements that can be represented by a `&[u8]`.
    ///
    /// This may be called with an empty iterator, which will cause a record
    /// terminator to be written. If no fields had been written, then a single
    /// empty field is written before the terminator.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::Writer;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = Writer::from_writer(vec![]);
    ///     wtr.write_record(&["a", "b", "c"])?;
    ///     wtr.write_record(&["x", "y", "z"])?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "a,b,c\nx,y,z\n");
    ///     Ok(())
    /// }
    /// ```
    pub fn write_record<I, T>(&mut self, record: I) -> Result<()>
        where I: IntoIterator<Item=T>, T: AsRef<[u8]>
    {
        for field in record.into_iter() {
            self.write_field(field)?;
        }
        self.write_terminator()
    }

    /// Write a single field.
    ///
    /// One should prefer using `write_record` over this method. It is provided
    /// for cases where writing a field at a time is more convenient than
    /// writing a record at a time.
    ///
    /// Note that if this API is used, `write_record` should be called with an
    /// empty iterator to write a record terminator.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate csv;
    ///
    /// use std::error::Error;
    /// use csv::Writer;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<Error>> {
    ///     let mut wtr = Writer::from_writer(vec![]);
    ///     wtr.write_field("a")?;
    ///     wtr.write_field("b")?;
    ///     wtr.write_field("c")?;
    ///     wtr.write_record(None::<&[u8]>)?;
    ///     wtr.write_field("x")?;
    ///     wtr.write_field("y")?;
    ///     wtr.write_field("z")?;
    ///     wtr.write_record(None::<&[u8]>)?;
    ///
    ///     let data = String::from_utf8(wtr.into_inner()?)?;
    ///     assert_eq!(data, "a,b,c\nx,y,z\n");
    ///     Ok(())
    /// }
    /// ```
    pub fn write_field<T: AsRef<[u8]>>(&mut self, field: T) -> Result<()> {
        if self.state.fields_written > 0 {
            self.write_delimiter()?;
        }
        let mut field = field.as_ref();
        loop {
            let (res, nin, nout) = self.core.field(field, self.buf.writable());
            field = &field[nin..];
            self.buf.written(nout);
            match res {
                WriteResult::InputEmpty => {
                    self.state.fields_written += 1;
                    return Ok(());
                }
                WriteResult::OutputFull => self.flush()?,
            }
        }
    }

    /// Flush the contents of the internal buffer to the underlying writer.
    ///
    /// If there was a problem writing to the underlying writer, then an error
    /// is returned.
    ///
    /// Note that this also flushes the underlying writer.
    pub fn flush(&mut self) -> io::Result<()> {
        self.state.panicked = true;
        let result = self.wtr.as_mut().unwrap().write_all(self.buf.readable());
        self.state.panicked = false;
        result?;
        self.buf.clear();
        self.wtr.as_mut().unwrap().flush()?;
        Ok(())
    }

    /// Flush the contents of the internal buffer and return the underlying
    /// writer.
    pub fn into_inner(
        mut self,
    ) -> result::Result<W, IntoInnerError<Writer<W>>> {
        match self.flush() {
            Ok(()) => Ok(self.wtr.take().unwrap()),
            Err(err) => Err(new_into_inner_error(self, err)),
        }
    }

    /// Write a CSV delimiter.
    fn write_delimiter(&mut self) -> Result<()> {
        loop {
            let (res, nout) = self.core.delimiter(self.buf.writable());
            self.buf.written(nout);
            match res {
                WriteResult::InputEmpty => return Ok(()),
                WriteResult::OutputFull => self.flush()?,
            }
        }
    }

    /// Write a CSV terminator.
    fn write_terminator(&mut self) -> Result<()> {
        if !self.state.flexible {
            match self.state.first_field_count {
                None => {
                    self.state.first_field_count =
                        Some(self.state.fields_written);
                }
                Some(expected) if expected != self.state.fields_written => {
                    return Err(Error::UnequalLengths {
                        pos: None,
                        expected_len: expected,
                        len: self.state.fields_written,
                    })
                }
                Some(_) => {}
            }
        }
        loop {
            let (res, nout) = self.core.terminator(self.buf.writable());
            self.buf.written(nout);
            match res {
                WriteResult::InputEmpty => {
                    self.state.fields_written = 0;
                    return Ok(());
                }
                WriteResult::OutputFull => self.flush()?,
            }
        }
    }
}

impl Buffer {
    /// Returns a slice of the buffer's current contents.
    ///
    /// The slice returned may be empty.
    fn readable(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    /// Returns a mutable slice of the remaining space in this buffer.
    ///
    /// The slice returned may be empty.
    fn writable(&mut self) -> &mut [u8] {
        &mut self.buf[self.len..]
    }

    /// Indicates that `n` bytes have been written to this buffer.
    fn written(&mut self, n: usize) {
        self.len += n;
    }

    /// Clear the buffer.
    fn clear(&mut self) {
        self.len = 0;
    }
}

#[cfg(test)]
mod tests {
    use byte_record::ByteRecord;
    use error::Error;
    use string_record::StringRecord;

    use super::{Writer, WriterBuilder};

    fn wtr_as_string(wtr: Writer<Vec<u8>>) -> String {
        String::from_utf8(wtr.into_inner().unwrap()).unwrap()
    }

    #[test]
    fn one_record() {
        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        wtr.write_record(vec!["a", "b", "c"]).unwrap();

        assert_eq!(wtr_as_string(wtr), "a,b,c\n");
    }

    #[test]
    fn one_string_record() {
        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        wtr.write_record(&StringRecord::from(vec!["a", "b", "c"])).unwrap();

        assert_eq!(wtr_as_string(wtr), "a,b,c\n");
    }

    #[test]
    fn one_byte_record() {
        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        wtr.write_record(&ByteRecord::from(vec!["a", "b", "c"])).unwrap();

        assert_eq!(wtr_as_string(wtr), "a,b,c\n");
    }

    #[test]
    fn unequal_records_bad() {
        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        wtr.write_record(&ByteRecord::from(vec!["a", "b", "c"])).unwrap();
        let err = wtr.write_record(&ByteRecord::from(vec!["a"])).unwrap_err();
        match err {
            Error::UnequalLengths { pos, expected_len, len } => {
                assert!(pos.is_none());
                assert_eq!(expected_len, 3);
                assert_eq!(len, 1);
            }
            x => panic!("expected UnequalLengths error, but got '{:?}'", x),
        }
    }

    #[test]
    fn unequal_records_ok() {
        let mut wtr = WriterBuilder::new().flexible(true).from_writer(vec![]);
        wtr.write_record(&ByteRecord::from(vec!["a", "b", "c"])).unwrap();
        wtr.write_record(&ByteRecord::from(vec!["a"])).unwrap();
        assert_eq!(wtr_as_string(wtr), "a,b,c\na\n");
    }

    #[test]
    fn serialize_with_headers() {
        #[derive(Serialize)]
        struct Row {
            foo: i32,
            bar: f64,
            baz: bool,
        }

        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        wtr.serialize(Row { foo: 42, bar: 42.5, baz: true }).unwrap();
        assert_eq!(wtr_as_string(wtr), "foo,bar,baz\n42,42.5,true\n");
    }

    #[test]
    fn serialize_no_headers() {
        #[derive(Serialize)]
        struct Row {
            foo: i32,
            bar: f64,
            baz: bool,
        }

        let mut wtr = WriterBuilder::new()
            .has_headers(false)
            .from_writer(vec![]);
        wtr.serialize(Row { foo: 42, bar: 42.5, baz: true }).unwrap();
        assert_eq!(wtr_as_string(wtr), "42,42.5,true\n");
    }

    #[test]
    fn serialize_tuple() {
        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        wtr.serialize((true, 1.3, "hi")).unwrap();
        assert_eq!(wtr_as_string(wtr), "true,1.3,hi\n");
    }
}
