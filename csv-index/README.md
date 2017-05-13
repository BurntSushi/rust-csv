csv-index
=========
A collection of data structures for indexing CSV data, with a focus on data
structures that can be easily serialized to and deserialized from disk.

[![Linux build status](https://api.travis-ci.org/BurntSushi/rust-csv.png)](https://travis-ci.org/BurntSushi/rust-csv)
[![Windows build status](https://ci.appveyor.com/api/projects/status/github/BurntSushi/rust-csv?svg=true)](https://ci.appveyor.com/project/BurntSushi/rust-csv)
[![](http://meritbadge.herokuapp.com/csv-index)](https://crates.io/crates/csv-index)

Dual-licensed under MIT or the [UNLICENSE](http://unlicense.org).

### Documentation

https://docs.rs/csv-index

### Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
csv-index = "0.1"
```

and this to your crate root:

```rust
extern crate csv_index;
```

### Example: build a simple random access index

The `RandomAccessSimple` index is a simple data structure that maps record
indices to the byte offset corresponding to the start of that record in CSV
data. This example shows how to save this index to disk for a particular CSV
file.

```rust
extern crate csv;
extern crate csv_index;

use std::error::Error;
use std::fs::File;
use std::io::{self, Write};
use csv_index::RandomAccessSimple;

fn main() {
  example().unwrap();
}

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
