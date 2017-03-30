#![allow(dead_code, unused_imports, unused_variables)]

extern crate csv_core;

pub use csv_core::{QuoteStyle, Terminator};

pub use reader::{Reader, ReaderBuilder, ReadField};
pub use record::ByteRecord;

mod reader;
mod record;
