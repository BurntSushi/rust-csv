use std::error;
use std::fmt;
use std::io;
use std::result;
use std::str;

use reader::Position;

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
        /// The position at which this error occurred.
        pos: Position,
        /// The field that has invalid UTF-8 (indexed starting at `0`).
        field: u64,
        /// The corresponding UTF-8 error.
        err: str::Utf8Error,
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
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::Utf8 { ref err, .. } => Some(err),
            Error::UnequalLengths{..} => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => err.fmt(f),
            Error::Utf8 { ref pos, field, ref err } => {
                write!(
                    f, "CSV parse error: record {} (byte {}, line {}): {}",
                    pos.record(), pos.byte(), pos.line(), err)
            }
            Error::UnequalLengths { expected_len, ref pos, len } => {
                write!(
                    f, "CSV parse error: record {} (byte {}, line {}): \
                        found record with {} fields, but the previous record \
                        has {} fields",
                    pos.record(), pos.byte(), pos.line(), len, expected_len)
            }
        }
    }
}
