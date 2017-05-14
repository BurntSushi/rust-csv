/*!
`csv-core` provides a fast CSV reader and writer for use in a `no_std` context.

This crate will never use the standard library. `no_std` support is therefore
enabled by default.

If you're looking for more ergonomic CSV parsing routines, please use the
[`csv`](https://docs.rs/csv) crate.

# Overview

This crate has two primary APIs. The `Reader` API provides a CSV parser, and
the `Writer` API provides a CSV writer.

# Example: reading CSV

This example shows how to count the number of fields and records in CSV data.

```
use csv_core::{Reader, ReadFieldResult};

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
    let (result, nin, _) = rdr.read_field(bytes, &mut [0; 1024]);
    bytes = &bytes[nin..];
    match result {
        ReadFieldResult::InputEmpty => {},
        ReadFieldResult::OutputFull => panic!("field too large"),
        ReadFieldResult::Field { record_end } => {
            count_fields += 1;
            if record_end {
                count_records += 1;
            }
        }
        ReadFieldResult::End => break,
    }
}
assert_eq!(3, count_records);
assert_eq!(9, count_fields);
```

# Example: writing CSV

This example shows how to use the `Writer` API to write valid CSV data. Proper
quoting is handled automatically.

```
use csv_core::Writer;

// This is where we'll write out CSV data.
let mut out = &mut [0; 1024];
// The number of bytes we've written to `out`.
let mut nout = 0;
// Create a CSV writer with a default configuration.
let mut wtr = Writer::new();

// Write a single field. Note that we ignore the `WriteResult` and the number
// of input bytes consumed since we're doing this by hand.
let (_, _, n) = wtr.field(&b"foo"[..], &mut out[nout..]);
nout += n;

// Write a delimiter and then another field that requires quotes.
let (_, n) = wtr.delimiter(&mut out[nout..]);
nout += n;
let (_, _, n) = wtr.field(&b"bar,baz"[..], &mut out[nout..]);
nout += n;
let (_, n) = wtr.terminator(&mut out[nout..]);
nout += n;

// Now write another record.
let (_, _, n) = wtr.field(&b"a \"b\" c"[..], &mut out[nout..]);
nout += n;
let (_, n) = wtr.delimiter(&mut out[nout..]);
nout += n;
let (_, _, n) = wtr.field(&b"quux"[..], &mut out[nout..]);
nout += n;

// We must always call finish once done writing.
// This ensures that any closing quotes are written.
let (_, n) = wtr.finish(&mut out[nout..]);
nout += n;

assert_eq!(&out[..nout], &b"\
foo,\"bar,baz\"
\"a \"\"b\"\" c\",quux"[..]);
```
*/

#![deny(missing_docs)]
#![no_std]

#[cfg(test)]
extern crate arrayvec;
extern crate memchr;

pub use reader::{
    Reader, ReaderBuilder, Terminator,
    ReadFieldResult, ReadFieldNoCopyResult,
    ReadRecordResult, ReadRecordNoCopyResult,
};
pub use writer::{Writer, WriterBuilder, WriteResult, QuoteStyle};

mod reader;
mod writer;
