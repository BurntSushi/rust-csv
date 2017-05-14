csv
===
A fast and flexible CSV reader and writer for Rust, with support for Serde.

[![Linux build status](https://api.travis-ci.org/BurntSushi/rust-csv.png)](https://travis-ci.org/BurntSushi/rust-csv)
[![Windows build status](https://ci.appveyor.com/api/projects/status/github/BurntSushi/rust-csv?svg=true)](https://ci.appveyor.com/project/BurntSushi/rust-csv)
[![](http://meritbadge.herokuapp.com/csv)](https://crates.io/crates/csv)

Dual-licensed under MIT or the [UNLICENSE](http://unlicense.org).


### Documentation

https://docs.rs/csv-index


### Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
csv = "1.0.0-beta.1"
```

and this to your crate root:

```rust
extern crate csv;
```

### Simple example

This example shows how to read CSV data from a file and print each record to
stdout.

There are more examples in the
[cookbook](https://docs.rs/csv/1.0.0-beta.1/csv/examples/index.html).

```rust
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
