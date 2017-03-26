/*!
`csv-core` provides a fast CSV reader and writer for use in a `no_std` context.

This crate will never use the standard library. `no_std` support is therefore
enabled by default.

If you're looking for more ergonomic CSV parsing routines, please use the
[`csv`](https://docs.rs/csv) crate.

# Overview

This crate has two primary APIs. The `Reader` API provides a CSV parser, and
the `Writer` API provides a CSV writer.

# Example: counting fields and records

This example shows how to count the number of fields and records in CSV data.

```
use csv_core::{Reader, ReadResult};

let data = "
foo,bar,baz
a,b,c
xxx,yyy,zzz
";

let mut rdr = Reader::new();
let mut bytes = data.as_bytes();
let mut count_fields = 0;
let mut count_records = 0;
loop {
    // We skip handling the output since we don't need it for counting.
    let (result, nin, _) = rdr.read(bytes, &mut [0; 1024]);
    bytes = &bytes[nin..];
    match result {
        ReadResult::InputEmpty => {},
        ReadResult::OutputFull => panic!("field too large"),
        ReadResult::Field { record_end } => {
            count_fields += 1;
            if record_end {
                count_records += 1;
            }
        }
        ReadResult::End => break,
    }
}
assert_eq!(3, count_records);
assert_eq!(9, count_fields);
```
*/

// #![deny(missing_docs)]
#![no_std]

#![allow(dead_code, unused_variables, unused_imports)]

#[cfg(test)]
extern crate arrayvec;

pub use reader::{Reader, ReaderBuilder, ReadResult, Terminator};

mod reader;
mod writer;
