#![allow(dead_code, unused_imports, unused_variables)]

extern crate bytecount;
extern crate csv_core;

pub use csv_core::{QuoteStyle, Terminator};

pub use error::{Error, Result};
pub use reader::{Position, Reader, ReaderBuilder, ReadField};
pub use record::ByteRecord;

mod error;
mod reader;
mod record;
