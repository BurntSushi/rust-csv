#![allow(dead_code, unused_imports, unused_variables)]

extern crate csv_core;
extern crate serde;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

pub use csv_core::{QuoteStyle, Terminator};

pub use byte_record::ByteRecord;
pub use error::{Error, FromUtf8Error, Result, Utf8Error};
pub use reader::{Position, Reader, ReaderBuilder};
pub use string_record::StringRecord;

mod byte_record;
mod deserializer;
mod error;
mod reader;
mod string_record;
