/*!
Docs.
*/

#![deny(missing_docs)]

extern crate csv_core;
extern crate serde;
#[cfg(test)]
extern crate serde_bytes;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

use std::result;

pub use csv_core::{QuoteStyle, Terminator};
use serde::{Deserialize, Deserializer};

pub use byte_record::{ByteRecord, ByteRecordIter, Position};
pub use deserializer::{DeserializeError, DeserializeErrorKind};
pub use error::{Error, FromUtf8Error, IntoInnerError, Result, Utf8Error};
pub use reader::{
    Reader, ReaderBuilder,
    DeserializeRecordsIntoIter, DeserializeRecordsIter,
    StringRecordsIntoIter, StringRecordsIter,
    ByteRecordsIntoIter, ByteRecordsIter,
};
pub use string_record::{StringRecord, StringRecordIter};
pub use writer::{Writer, WriterBuilder};

mod byte_record;
mod deserializer;
mod error;
mod reader;
mod serializer;
mod string_record;
pub mod tutorial;
mod writer;

/// A custom Serde deserializer for possibly invalid `Option<T>` fields.
///
/// When deserializing CSV data, it is sometimes desirable to simply ignore
/// fields with invalid data. For example, there might be a field that is
/// usually a number, but will occasionally contain garbage data that causes
/// number parsing to fail.
///
/// You might be inclined to use, say, `Option<i32>` for fields such at this.
/// By default, however, `Option<i32>` will either capture *empty* fields with
/// `None` or valid numeric fields with `Some(the_number)`. If the field is
/// non-empty and not a valid number, then deserialization will return an error
/// instead of using `None`.
///
/// This function allows you to override this default behavior. Namely, if
/// `Option<T>` is deserialized with non-empty but invalid data, then the value
/// will be `None` and the error will be ignored.
///
/// # Example
///
/// This example shows how to parse CSV records with numerical data, even if
/// some numerical data is absent or invalid. Without the
/// `serde(deserialize_with = "...")` annotations, this example would return
/// an error.
///
/// ```
/// extern crate csv;
/// #[macro_use]
/// extern crate serde_derive;
///
/// use std::error::Error;
/// use csv::Reader;
///
/// #[derive(Debug, Deserialize, Eq, PartialEq)]
/// struct Row {
///     #[serde(deserialize_with = "csv::invalid_option")]
///     a: Option<i32>,
///     #[serde(deserialize_with = "csv::invalid_option")]
///     b: Option<i32>,
///     #[serde(deserialize_with = "csv::invalid_option")]
///     c: Option<i32>,
/// }
///
/// # fn main() { example().unwrap(); }
/// fn example() -> Result<(), Box<Error>> {
///     let data = "\
///a,b,c
///5,\"\",xyz
///";
///     let mut rdr = Reader::from_reader(data.as_bytes());
///     if let Some(result) = rdr.deserialize().next() {
///         let record: Row = result?;
///         assert_eq!(record, Row { a: Some(5), b: None, c: None });
///         Ok(())
///     } else {
///         Err(From::from("expected at least one record but got none"))
///     }
/// }
/// ```
pub fn invalid_option<'de, D, T>(de: D) -> result::Result<Option<T>, D::Error>
    where D: Deserializer<'de>, Option<T>: Deserialize<'de>
{
    match Option::<T>::deserialize(de) {
        Ok(some_t) => Ok(some_t),
        Err(_) => Ok(None),
    }
}
