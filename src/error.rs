use std::error;
use std::fmt;
use std::io;
use std::result;
use std::str;

use reader::Position;
use byte_record::ByteRecord;

/// A type alias for `Result<T, csv::Error>`.
pub type Result<T> = result::Result<T, Error>;

/// An error that can occur when processing CSV data.
///
/// This error can happen when writing or reading CSV data.
///
/// Note that there are some important scenarios where an error is impossible
/// to occur. For example, if a CSV reader is used on an in-memory buffer with
/// the `flexible` option enabled and one is reading records as raw byte
/// strings, then no error can occur.
#[derive(Debug)]
pub enum Error {
    /// An I/O error that occurred while reading CSV data.
    Io(io::Error),
    /// A UTF-8 decoding error that occured while reading CSV data into Rust
    /// `String`s.
    Utf8 {
        /// The position of the record in which this error occurred, if
        /// available.
        pos: Option<Position>,
        /// The corresponding UTF-8 error.
        err: Utf8Error,
    },
    /// This error occurs when two records with an unequal number of fields
    /// are found. This error only occurs when the `flexible` option in a
    /// CSV reader is disabled.
    UnequalLengths {
        /// The expected number of fields in a record. This is the number of
        /// fields in the record read prior to the record indicated by
        /// `pos`.
        expected_len: u64,
        /// The position of the first record with an unequal number of fields
        /// to the previous record.
        pos: Position,
        /// The number of fields in the bad record.
        len: u64,
    },
    /// This error occurs when either the `byte_headers` or `headers` methods
    /// are called on a CSV reader that was asked to `seek` before it parsed
    /// the first record.
    Seek,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref err) => err.description(),
            Error::Utf8 { ref err, .. } => err.description(),
            Error::UnequalLengths{..} => "record of different length found",
            Error::Seek => "headers unavailable on seeked CSV reader",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::Utf8 { ref err, .. } => Some(err),
            Error::UnequalLengths{..} => None,
            Error::Seek => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => err.fmt(f),
            Error::Utf8 { pos: None, ref err } => {
                write!(f, "CSV parse error: field {}: {}", err.field(), err)
            }
            Error::Utf8 { pos: Some(ref pos), ref err } => {
                write!(
                    f,
                    "CSV parse error: record {} \
                     (byte {}, line {}, field: {}): {}",
                    pos.record(), pos.byte(), pos.line(), err.field(), err)
            }
            Error::UnequalLengths { expected_len, ref pos, len } => {
                write!(
                    f, "CSV parse error: record {} (byte {}, line {}): \
                        found record with {} fields, but the previous record \
                        has {} fields",
                    pos.record(), pos.byte(), pos.line(), len, expected_len)
            }
            Error::Seek => {
                write!(f, "CSV error: cannot access headers of CSV data \
                           when the parser was seeked before the first record \
                           could be read")
            }
        }
    }
}

/// A UTF-8 validation error that occurs when attempting to convert a
/// `ByteRecord` into a `StringRecord`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FromUtf8Error {
    record: ByteRecord,
    err: Utf8Error,
}

/// Create a new FromUtf8Error.
pub fn new_from_utf8_error(rec: ByteRecord, err: Utf8Error) -> FromUtf8Error {
    FromUtf8Error { record: rec, err: err }
}

impl FromUtf8Error {
    /// Access the underlying `ByteRecord` that failed UTF-8 validation.
    pub fn into_byte_record(self) -> ByteRecord {
        self.record
    }

    /// Access the underlying UTF-8 validation error.
    pub fn utf8_error(&self) -> &Utf8Error {
        &self.err
    }
}

impl fmt::Display for FromUtf8Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.err.fmt(f)
    }
}

impl error::Error for FromUtf8Error {
    fn description(&self) -> &str { self.err.description() }
    fn cause(&self) -> Option<&error::Error> { Some(&self.err) }
}

/// A UTF-8 validation error that occurred when attempting to convert a
/// `ByteRecord` into a `StringRecord`.
///
/// The error includes the index of the field that failed validation, and the
/// last byte at which valid UTF-8 was verified.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Utf8Error {
    /// The field index of a byte record in which UTF-8 validation failed.
    field: usize,
    /// The index into the given field up to which valid UTF-8 was verified.
    valid_up_to: usize,
}

/// Create a new UTF-8 error.
pub fn new_utf8_error(field: usize, valid_up_to: usize) -> Utf8Error {
    Utf8Error { field: field, valid_up_to: valid_up_to }
}

impl Utf8Error {
    /// The field index of a byte record in which UTF-8 validation failed.
    pub fn field(&self) -> usize { self.field }
    /// The index into the given field up to which valid UTF-8 was verified.
    pub fn valid_up_to(&self) -> usize { self.valid_up_to }
}

impl fmt::Display for Utf8Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "invalid utf-8: invalid UTF-8 in field {} near byte index {}",
            self.field,
            self.valid_up_to)
    }
}

impl error::Error for Utf8Error {
    fn description(&self) -> &str { "invalid utf-8 in CSV record" }
}
