extern crate csv_core;
extern crate serde;
#[cfg(test)]
extern crate serde_bytes;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

pub use csv_core::{QuoteStyle, Terminator};

pub use byte_record::{ByteRecord, ByteRecordIter, Position};
pub use deserializer::{DeserializeError, DeserializeErrorKind};
pub use error::{Error, FromUtf8Error, IntoInnerError, Result, Utf8Error};
pub use reader::{Reader, ReaderBuilder};
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
