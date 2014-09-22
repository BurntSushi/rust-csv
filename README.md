This crate provides a streaming CSV (comma separated values) writer and
reader that works with the `serialize` crate to do type based encoding
and decoding. There are two primary goals of this project:

1. The default mode of parsing should *just work*. This means the parser
   will bias toward providing *a* parse over a *correct* parse (with
   respect to [RFC 4180](http://tools.ietf.org/html/rfc4180)).
2. Convenient to use by default, but when performance is needed, the
   API will provide an escape hatch.

[![Build status](https://api.travis-ci.org/BurntSushi/rust-csv.png)](https://travis-ci.org/BurntSushi/rust-csv)

Licensed under the [UNLICENSE](http://unlicense.org).


### Documentation

The API is fully documented with lots of examples:
[http://burntsushi.net/rustdoc/csv/](http://burntsushi.net/rustdoc/csv/).


### Simple examples

Here is a full working Rust program that decodes records from a CSV file. Each
record consists of two strings and an integer (the edit distance between the
strings):

```rust
extern crate csv;

use std::path::Path;

fn main() {
    let fp = &Path::new("./data/simple.csv");
    let mut rdr = csv::Reader::from_file(fp);

    for record in rdr.decode() {
        let (s1, s2, dist): (String, String, uint) = record.unwrap();
        println!("({}, {}): {}", s1, s2, dist);
    }
}
```

Don't like tuples? That's fine. Use a struct instead:

```rust
extern crate csv;
extern crate serialize;

use std::path::Path;

#[deriving(Decodable)]
struct Record {
    s1: String,
    s2: String,
    dist: uint,
}

fn main() {
    let fp = &Path::new("./data/simple.csv");
    let mut rdr = csv::Reader::from_file(fp);

    for record in rdr.decode() {
        let record: Record = record.unwrap();
        println!("({}, {}): {}", record.s1, record.s2, record.dist);
    }
}
```

Do some records not have a distance for some reason? Use an `Option` type!

```rust
#[deriving(Decodable)]
struct Record {
    s1: String,
    s2: String,
    dist: Option<uint>,
}
```

You can also read CSV headers, change the delimiter, use `enum` types or just
get plain access to records as vectors of strings. There are examples with more
details in the documentation.


### Installation

This crate works with Cargo. Assuming you have Rust and
[Cargo](http://crates.io/) installed, simply check out the source and run 
tests:

```bash
git checkout git://github.com/BurntSushi/rust-csv
cd rust-csv
cargo test
```

You can also add `rust-csv` as a dependency to your project's `Cargo.toml`:

```toml
[dependencies.rust-csv]
git = "git://github.com/BurntSushi/rust-csv"
```


### Related work

The only other one I know is
[arjantop/rust-tabular](https://github.com/arjantop/rust-tabular),
which does not support the `Encoder` or `Decoder` API. However, it does support 
parsing fixed-width tables.

