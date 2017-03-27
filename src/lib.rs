//! This crate provides a streaming CSV (comma separated values) writer and
//! reader that works with the `serialize` crate to do type based encoding
//! and decoding. There are two primary goals of this project:
//!
//! 1. The default mode of parsing should *just work*. This means the parser
//!    will bias toward providing *a* parse over a *correct* parse (with
//!    respect to RFC 4180).
//! 2. Convenient to use by default, but when performance is needed, the
//!    API will provide an escape hatch.
//!
//! ## Simple example
//!
//! This shows how you can decode records into Rust types. This saves a ton
//! of boiler plate, e.g., converting strings to numeric types.
//!
//! ```rust
//! let data = "
//! sticker,mortals,7
//! bribed,personae,7
//! wobbling,poncing,4
//! interposed,emmett,9
//! chocolate,refile,7";
//!
//! let mut rdr = csv::Reader::from_string(data).has_headers(false);
//! for row in rdr.decode() {
//!     let (n1, n2, dist): (String, String, u32) = row.unwrap();
//!     println!("{}, {}: {}", n1, n2, dist);
//! }
//! ```
//!
//! If you just want a `Vec` of all the records, then you can use the
//! `collect` method defined on iterators:
//!
//! ```rust
//! let data = "
//! sticker,mortals,7
//! bribed,personae,7
//! wobbling,poncing,4
//! interposed,emmett,9
//! chocolate,refile,7";
//!
//! type Row = (String, String, u32);
//!
//! let mut rdr = csv::Reader::from_string(data).has_headers(false);
//! let rows = rdr.decode().collect::<csv::Result<Vec<Row>>>().unwrap();
//! assert_eq!(rows.len(), 5);
//! ```
//!
//! Please see the `Reader` type for more documentation and examples.
//!
//! ## Iteratoring over records
//!
//! This crate exposes **4** distinct ways of iterating over CSV records. In
//! the majority of use cases, you should use the `decode` method as shown
//! above because it is the most convenient. But other types of iterators are
//! exposed for when you need them.
//!
//! The iterators listed below are presented in order of performance. The first
//! (type based decoding) is the slowest and the last (zero allocation) is the
//! fastest. There is clear evidence of this claim in the benchmarks. (Just
//! run `cargo bench`.)
//!
//! ### Decoded records
//!
//! As shown above. This uses type based decoding on each record.
//!
//! ### String records
//!
//! Yields each record as a `Vec<String>`. Namely, this assumes that all CSV
//! data is UTF-8 encoded. This is the standard CSV interface that you've
//! probably come to expect from using other CSV parsers.
//!
//! ```rust
//! let data = "
//! sticker,mortals,7
//! bribed,personae,7
//! wobbling,poncing,4
//! interposed,emmett,9
//! chocolate,refile,7";
//!
//! let mut rdr = csv::Reader::from_string(data).has_headers(false);
//! for row in rdr.records().map(|r| r.unwrap()) {
//!     println!("{:?}", row);
//! }
//! ```
//!
//! ### Byte string records
//!
//! Yields each record as a `Vec<ByteString>`. Namely, this allows reading CSV
//! data that is not UTF-8 encoded (or improperly encoded!).
//!
//! ```rust
//! let data = b"
//! sti\xffcker,mortals,7
//! chocolate,refile,7";
//!
//! let mut rdr = csv::Reader::from_bytes(&data[..]).has_headers(false);
//! for row in rdr.byte_records().map(|r| r.unwrap()) {
//!     println!("{:?}", row);
//! }
//! ```
//!
//! ### Byte slice records
//!
//! This iterator is defined on the `Reader` type itself and yields *fields*
//! instead of records (unlike the other iterators). Each field is a `&[u8]`.
//! No allocation is performed during parsing (unlike the other iterators,
//! which at least allocate a `Vec<u8>` for each field and a `Vec<_>` for each
//! record). Since no allocation is performed, this "iterator" doesn't actually
//! implement the `Iterator` trait (since it cannot be done safely).
//!
//! This is the lowest level interface and should only be used when you need
//! the performance.
//!
//! ```rust
//! let data = "
//! sticker,mortals,7
//! bribed,personae,7
//! wobbling,poncing,4
//! interposed,emmett,9
//! chocolate,refile,7";
//!
//! let mut rdr = csv::Reader::from_string(data);
//! while !rdr.done() {
//!     while let Some(r) = rdr.next_bytes().into_iter_result() {
//!         print!("{:?} ", r.unwrap());
//!     }
//!     println!("");
//! }
//! ```
//!
//! There is more explanation for how this iterator interface works on the
//! `Reader` type.
//!
//! ## Indexing
//!
//! This crate has experimental support for CSV record indexing. It's very
//! simplistic, but once the index is created, you can seek a `csv::Reader`
//! to any record instantly. See the
//! [`csv::index`](/rustdoc/csv/index/index.html)
//! sub-module for more details and examples.
//!
//! ## Compliance with RFC 4180
//!
//! [RFC 4180](http://tools.ietf.org/html/rfc4180) seems to the closest thing
//! to an official specification for CSV. Currently, the parser in this crate
//! will read a strict superset of RFC 4180 while the writer will always write
//! CSV data that conforms to RFC 4180 (unless configured to do otherwise).
//! This approach was taken because CSV data is commonly malformed and there is
//! nothing worse than trying to read busted CSV data with a library that says
//! it can't do it.
//!
//! With that said, a "strict" mode may be added that will only read CSV data
//! that conforms to RFC 4180.
//!
//! Here are a few notes on compatibility with RFC 4180:
//!
//!   * Both CRLF and LF line endings are supported. This is seamless in the
//!     parser. By default, the encoder uses LF line endings but can be
//!     instructed to use CRLF with the `crlf` method.
//!   * The first record is read as a "header" by default, but this can be
//!     disabled by calling `has_headers(false)` before reading any records.
//!     (N.B. The encoder has no explicit support for headers. Simply encode a
//!     vector of strings instead.)
//!   * By default, the delimiter is a comma, but it can be changed to any
//!     **ASCII** byte character with the `delimiter` method (for either
//!     writing or reading).
//!   * By default, both the writer and reader will enforce the invariant
//!     that all records are the same length. (This is what RFC 4180 demands.)
//!     If a record with a different length is found, an error is returned.
//!     This behavior may be turned off by calling `flexible` with `true`.
//!   * Empty lines (that do not include other whitespace) are ignored
//!     by the parser.
//!   * This crate parses CSV data at the *byte* level, which means all
//!     delimiter and quote characters must be ASCII. While unfortunate, this
//!     means that CSV data that is not UTF-8 encoded can be parsed. In
//!     general, the writer and reader API biases toward using Unicode strings
//!     while providing an outlet to use byte strings.

#![crate_name = "csv"]
#![doc(html_root_url = "http://burntsushi.net/rustdoc/csv")]

#![deny(missing_docs)]

extern crate byteorder;
extern crate memchr;
extern crate rustc_serialize;

use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::result;

pub use borrow_bytes::BorrowBytes;
pub use encoder::Encoded;
pub use decoder::Decoded;
pub use reader::{
    Reader, DecodedRecords, StringRecords, ByteRecords, NextField,
    RecordTerminator,
};
pub use writer::{Writer, QuoteStyle};

macro_rules! lg {
    ($($tt:tt)*) => ({
        use std::io::Write;
        writeln!(&mut ::std::io::stderr(), $($tt)*).unwrap();
    });
}

pub mod index;

mod borrow_bytes;
mod encoder;
mod decoder;
mod reader;
mod writer;

#[cfg(test)]
mod tests;

/// A convenience type for representing the result of most CSV reader/writer
/// operations.
pub type Result<T> = result::Result<T, Error>;

/// A convenience type for referring to a plain byte string.
pub type ByteString = Vec<u8>;

/// An error produced by an operation on CSV data.
#[derive(Debug)]
pub enum Error {
    /// An error reported by the type-based encoder.
    Encode(String),
    /// An error reported by the type-based decoder.
    Decode(String),
    /// An error reported by the CSV parser.
    Parse(LocatableError<ParseError>),
    /// An error originating from reading or writing to the underlying buffer.
    Io(io::Error),
    /// An error originating from using a CSV index.
    Index(String),
}

/// An error tagged with a location at which it occurred.
#[derive(Clone, Copy, Debug)]
pub struct LocatableError<T> {
    /// The record number (starting at 1).
    pub record: u64,
    /// The field number (starting at 1).
    pub field: u64,
    /// The error.
    pub err: T,
}

/// A description of a CSV parse error.
#[derive(Clone, Copy, Debug)]
pub enum ParseError {
    /// A record was found that has a different size than other records.
    ///
    /// This is only reported when `flexible` is set to `false` on the
    /// corresponding CSV reader/writer.
    UnequalLengths {
        /// Expected a record with this many fields.
        expected: u64,
        /// Got a record with this many fields.
        got: u64,
    },
    /// An error occurred when trying to convert a field to a Unicode string.
    ///
    /// TODO: Include the real Utf8Error, but it is not stabilized yet.
    InvalidUtf8,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Encode(ref msg) => write!(f, "CSV encode error: {}", msg),
            Error::Decode(ref msg) => write!(f, "CSV decode error: {}", msg),
            Error::Parse(ref err) => write!(f, "{}", err),
            Error::Io(ref err) => write!(f, "{}", err),
            Error::Index(ref msg) => write!(f, "CSV index error: {}", msg),
        }
    }
}

impl<T: fmt::Display> fmt::Display for LocatableError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CSV error (at record {}, field {}): {}",
               self.record, self.field, self.err)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseError::UnequalLengths { expected, got } =>
                write!(f, "First record has length {}, but found record \
                           with length {}.", expected, got),
            ParseError::InvalidUtf8 =>
                write!(f, "Invalid UTF8 encoding."),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Encode(..) => "CSV encoding error",
            Error::Decode(..) => "CSV decoding error",
            Error::Parse(..) => "CSV parse error",
            Error::Io(..) => "CSV IO error",
            Error::Index(..) => "CSV indexing error",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Error::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error { Error::Io(err) }
}
