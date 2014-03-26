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

    for (s1, s2, dist) in rdr.decode_iter::<(~str, ~str, uint)>() {
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
    s1: ~str,
    s2: ~str,
    dist: uint,
}

fn main() {
    let fp = &Path::new("./data/simple.csv");
    let mut rdr = csv::Decoder::from_file(fp);

    for record in rdr.decode_iter::<Record>() {
        println!("({}, {}): {}", record.s1, record.s2, record.dist);
    }
}
```

Do some records not have a distance for some reason? Use an `Option` type!

```rust
#[deriving(Decodable)]
struct Record {
    s1: ~str,
    s2: ~str,
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

This will hopefully get easier when the new `cargo` package manager lands, but 
for right now, you can either clone the repo and build manually or install with
`cargo-lite`.

From source:

```bash
git clone git://github.com/BurntSushi/rust-csv
cd rust-csv
rustc -O --crate-type lib ./src/lib.rs # makes libcsv-{version}.rlib in CWD
cd ./examples
rustc -O -L .. ./nfl_plays.rs
./nfl_plays
```

For `cargo-lite`:

```bash
pip2 install cargo-lite
cargo-lite install git://github.com/BurntSushi/rust-csv # installs to ~/.rust
cd ~/.rust/src/rust-csv/examples
rustc -O ./nfl_plays.rs
./nfl_plays
```


### Related work

The only other CSV parser I know of that builds is
[Geal/rust-csv](https://github.com/Geal/rust-csv), but it doesn't support the
`Encoder` or `Decoder` API.

Another one popped up at
[arjantop/rust-tabular](https://github.com/arjantop/rust-tabular) just 
recently, which also does not support the `Encoder` or `Decoder` API.
However, it does support parsing fixed-width tables.

