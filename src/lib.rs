/*!
The `csv` crate provides a fast and flexible CSV reader and writer, with
support for Serde.

The [tutorial](tutorial/index.html) is a good place to start if you're new to
Rust.

The [cookbook](cookbook/index.html) will give you a variety of complete Rust
programs that do CSV reading and writing.

# Brief overview

**If you're new to Rust**, you might find the
[tutorial](tutorial/index.html)
to be a good place to start.

The primary types in this crate are
[`Reader`](struct.Reader.html)
and
[`Writer`](struct.Writer.html),
for reading and writing CSV data respectively.
Correspondingly, to support CSV data with custom field or record delimiters
(among many other things), you should use either a
[`ReaderBuilder`](struct.ReaderBuilder.html)
or a
[`WriterBuilder`](struct.WriterBuilder.html),
depending on whether you're reading or writing CSV data.

Unless you're using Serde, the standard CSV record types are
[`StringRecord`](struct.StringRecord.html)
and
[`ByteRecord`](struct.ByteRecord.html).
`StringRecord` should be used when you know your data to be valid UTF-8.
For data that may be invalid UTF-8, `ByteRecord` is suitable.

Finally, the set of errors is described by the
[`Error`](enum.Error.html)
type.

The rest of the types in this crate mostly correspond to more detailed errors,
position information, configuration knobs or iterator types.

# Setup

Add this to your `Cargo.toml`:

```toml
[dependencies]
csv = "1.0.0-beta.1"
```

and this to your crate root:

```rust
extern crate csv;
```

# Simple example

This example shows how to read CSV data from a file and print each record to
stdout.

There are more examples in the [cookbook](cookbook/index.html).

```no_run
extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

fn example() -> Result<(), Box<Error>> {
    // Build the CSV reader and iterate over each record.
    let file_path = get_first_arg()?;
    let mut rdr = csv::Reader::from_path(&file_path)?;
    for result in rdr.records() {
        // The iterator yields Result<StringRecord, Error>, so we check the
        // error here..
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}

fn get_first_arg() -> Result<OsString, Box<Error>> {
    match env::args_os().nth(1) {
        Some(file_path) => Ok(file_path),
        None => Err(From::from("expected 1 argument, but got none")),
    }
}

fn main() {
    if let Err(err) = example() {
        println!("error running example: {}", err);
        process::exit(1);
    }
}
```

The above example can be run like so:

```ignore
$ git clone git://github.com/BurntSushi/rust-csv
$ cd rust-csv
$ cargo run --example simple examples/data/simplepop.csv
```
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
pub mod cookbook;
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
    Option::<T>::deserialize(de).or_else(|_| Ok(None))
}
