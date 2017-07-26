/*!
The `csv-index` crate provides data structures for indexing CSV data.

# Usage

This crate is
[on crates.io](https://crates.io/crates/csv-index)
and can be used by adding `csv-index` to your dependencies in your project's
`Cargo.toml`

```toml
[dependencies]
csv-index = "0.1"
```

and this to your crate root:

```ignore
extern crate csv_index;
```

# Example: build a simple random access index

The `RandomAccessSimple` index is a simple data structure that maps record
indices to the byte offset corresponding to the start of that record in CSV
data. This example shows how to save this index to disk for a particular CSV
file.

Note that this indexing data structure cannot be updated. That means that if
your CSV data has changed since the index was created, then the index will need
to be regenerated.

```no_run
extern crate csv;
extern crate csv_index;

use std::error::Error;
use std::fs::File;
use std::io::{self, Write};
use csv_index::RandomAccessSimple;

# fn main() { example().unwrap(); }
fn example() -> Result<(), Box<Error>> {
    // Open a normal CSV reader.
    let mut rdr = csv::Reader::from_path("data.csv")?;

    // Create an index for the CSV data in `data.csv` and write it
    // to `data.csv.idx`.
    let mut wtr = io::BufWriter::new(File::create("data.csv.idx")?);
    RandomAccessSimple::create(&mut rdr, &mut wtr)?;
    wtr.flush()?;

    // Open the index we just created, get the position of the last
    // record and seek the CSV reader to the last record.
    let mut idx = RandomAccessSimple::open(File::open("data.csv.idx")?)?;
    if idx.is_empty() {
        return Err(From::from("expected a non-empty CSV index"));
    }
    let last = idx.len() - 1;
    let pos = idx.get(last)?;
    rdr.seek(pos)?;

    // Read the next record.
    if let Some(result) = rdr.records().next() {
        let record = result?;
        println!("{:?}", record);
        Ok(())
    } else {
        Err(From::from("expected at least one record but got none"))
    }
}
```

# Future work

The full scope of this crate hasn't been determined yet. For example, it's not
clear whether this crate should support data structures more amenable to
in-memory indexing. (Where the current set of indexing data structures are all
amenable to serializing to disk.)
*/

#![deny(missing_docs)]

extern crate byteorder;
extern crate csv;

pub use simple::RandomAccessSimple;

mod simple;
