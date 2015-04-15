This crate provides a *fast* streaming CSV (comma separated values) writer and
reader that works with the `serialize` crate to do type based encoding
and decoding. There are two primary goals of this project:

1. The default mode of parsing should *just work*. This means the parser
   will bias toward providing *a* parse over a *correct* parse (with
   respect to [RFC 4180](http://tools.ietf.org/html/rfc4180)).
2. Convenient to use by default, but when performance is needed, the
   API will provide an escape hatch.

There is evidence of this parser's performance at the bottom of this README.
You can also see how it compares to other parsers in
[ewanhiggs' CSV game](https://bitbucket.org/ewanhiggs/csv-game).

[![Build status](https://api.travis-ci.org/BurntSushi/rust-csv.png)](https://travis-ci.org/BurntSushi/rust-csv)

Dual-licensed under MIT or the [UNLICENSE](http://unlicense.org).


### Documentation

The API is fully documented with lots of examples:
[http://burntsushi.net/rustdoc/csv/](http://burntsushi.net/rustdoc/csv/).


### Simple examples

Here is a full working Rust program that decodes records from a CSV file. Each
record consists of two strings and an integer (the edit distance between the
strings):

```rust
extern crate csv;

fn main() {
    let mut rdr = csv::Reader::from_file("./data/simple.csv").unwrap();
    for record in rdr.decode() {
        let (s1, s2, dist): (String, String, usize) = record.unwrap();
        println!("({}, {}): {}", s1, s2, dist);
    }
}
```

Don't like tuples? That's fine. Use a struct instead:

```rust
extern crate csv;
extern crate rustc_serialize;

#[derive(RustcDecodable)]
struct Record {
    s1: String,
    s2: String,
    dist: u32,
}

fn main() {
    let mut rdr = csv::Reader::from_file("./data/simple.csv").unwrap();
    for record in rdr.decode() {
        let record: Record = record.unwrap();
        println!("({}, {}): {}", record.s1, record.s2, record.dist);
    }
}
```

Do some records not have a distance for some reason? Use an `Option` type!

```rust
#[derive(RustcDecodable)]
struct Record {
    s1: String,
    s2: String,
    dist: Option<u32>,
}
```

You can also read CSV headers, change the delimiter, use `enum` types or just
get plain access to records as vectors of strings. There are examples with more
details in the documentation.


### Installation

This crate works with Cargo and is on
[crates.io](https://crates.io/crates/csv). The package is regularly updated.
Add it to your `Cargo.toml` like so:

```toml
[dependencies]
csv = "*"
```


### Performance and benchmarks

I claim that this is one of the fastest CSV parsers out there. Its speed should
be comparable or better than
[`libcsv`](http://sourceforge.net/projects/libcsv/)
while providing a more convenient and safer interface. At the lowest level, the
parser can decode CSV at about 200 MB/sec. Here are some rough benchmarks:

```
raw     ... bench:   5627467 ns/iter (+/- 171958) = 241 MB/s
byte    ... bench:   9307428 ns/iter (+/- 473205) = 146 MB/s
string  ... bench:  11043921 ns/iter (+/- 55845)  = 122 MB/s
decoded ... bench:  16150376 ns/iter (+/- 496846) = 83 MB/s
```

`raw` corresponds to the *zero allocation* parser. Namely, no allocations are
made for each field or row. For example, this is the fastest way to compute the
number of records in a CSV file:

```rust
extern crate csv;

fn main() {
    let fpath = ::std::env::args().nth(1).unwrap();
    let mut rdr = csv::Reader::from_file(fpath).unwrap();
    let mut count = 0;
    loop {
        match rdr.next_bytes() {
            NextField::EndOfCsv => break,
            NextField::EndOfRecord => { count += 1; break; }
            NextField::Data(_) => {}
            NextField::Error(err) => fail!(err),
        }
    }
    println!("{}", count);
}
```

`byte` corresponds to allocating a fresh byte string for each field and a fresh
vector for each row. This is more convenient than using the `raw` API:

```rust
extern crate csv;

fn main() {
    let fpath = ::std::env::args().nth(1).unwrap();
    let mut rdr = csv::Reader::from_file(fpath).unwrap();
    let mut count = 0;
    for record in rdr.byte_records().map(|r| r.unwrap()) {
        count += 1;
    }
    println!("{}", count);
}
```

`string` is just like `byte`, except each field is decoded from UTF-8 into a
Unicode string. It's exactly like above, except one uses `records` instead of
`byte_records`.

`decoded` is the slowest approach but also the most convenient if your CSV
contains data other than plain strings, like numbers or booleans.


### Indexing

This library also includes simplistic CSV indexing support. Once a CSV index
is created, you can use it to jump to any record in the data instantly. In
essence, it gives you random access for a modest upfront cost in time and
memory.

This example shows how to create an in-memory index and use it to jump to
any record in the data. (The indexing interface works with seekable readers
and writers, so you can use `std::fs::File` for this too.)

```rust
extern crate csv;

use std::io::{self, Write};
use csv::index::{Indexed, create_index};

fn main() {
  let data = "
  h1,h2,h3
  a,b,c
  d,e,f
  g,h,i
  ";

  let new_csv_rdr = || csv::Reader::from_string(data);

  let mut index_data = io::Cursor::new(Vec::new());
  create_index(new_csv_rdr(), index_data.by_ref()).unwrap();
  let mut index = Indexed::open(new_csv_rdr(), index_data).unwrap();

  // Seek to the second record and read its data. This is done *without*
  // reading the first record.
  index.seek(1).unwrap();

  // Read the first row at this position (which is the second record).
  // Since `Indexed` derefs to a `csv::Reader`, we can call CSV reader methods
  // on it directly.
  let row = index.records().next().unwrap().unwrap();

  assert_eq!(row, vec!["d", "e", "f"]);
}
```
