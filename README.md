This crate provides a streaming CSV (comma separated values) encoder and
decoder that works with the `Encoder` and `Decoder` traits in Rust's
`serialize` crate. It [conforms closely to RFC
4180](http://burntsushi.net/rustdoc/csv/#compliance-with-rfc-4180).

[![Build status](https://api.travis-ci.org/BurntSushi/rust-csv.png)](https://travis-ci.org/BurntSushi/rust-csv)

Licensed under the [UNLICENSE](http://unlicense.org).


### Simple examples

Here is a full working Rust program that decodes records from a CSV file. Each
record consists of two strings and an integer (the edit distance between the
strings):

```rust
extern crate csv;

use std::path::Path;

fn main() {
    let fp = &Path::new("./data/simple.csv");
    let mut rdr = csv::Decoder::from_file(fp);

    for record in rdr.iter_decode::<(String, String, uint)>() {
        let (s1, s2, dist) = record.unwrap();
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
    let mut rdr = csv::Decoder::from_file(fp);

    for record in rdr.iter_decode::<Record>() {
        let record = record.unwrap();
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

You can also read CSV headers, change the separator, use `enum` types or just
get plain access to records as vectors of strings. There are examples with more
details in the documentation.

### Documentation

The API is fully documented with lots of examples:
[http://burntsushi.net/rustdoc/csv/](http://burntsushi.net/rustdoc/csv/).


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

The only other CSV parser I know of that builds is
[Geal/rust-csv](https://github.com/Geal/rust-csv), but it doesn't support the
`Encoder` or `Decoder` API.

Another one popped up at
[arjantop/rust-tabular](https://github.com/arjantop/rust-tabular) just
recently, which also does not support the `Encoder` or `Decoder` API.
However, it does support parsing fixed-width tables.

