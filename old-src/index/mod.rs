/*!
This sub-module provides experimental CSV record indexing.

It is simplistic, but once a CSV index is created, you can use it to jump to
any record in the data instantly. In essence, it gives you random access for a
modest upfront cost in time and memory.

## Simple example

This example shows how to create an in-memory index and use it to jump to
any record in the data. (The indexing interface works with seekable readers
and writers, so you can use `std::fs::File` for this too.)

```rust
# extern crate csv; fn main() {
use std::io::{self, Write};
use csv::index::{Indexed, create_index};

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
# }
```
*/
use std::io;
use std::ops;

use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};

use {Result, Error, Reader, NextField};

/// A type for representing CSV data with a basic record index.
///
/// The index is a sequence of 64 bit (8 bytes) integers, where each integer
/// corresponds to the *byte offset* of the start of the corresponding record
/// in the CSV data. This allows one to skip to any record in the CSV data
/// and read it instantly.
///
/// Note that this type derefs to a `&mut csv::Reader<R>`.
pub struct Indexed<R, I> {
    rdr: Reader<R>,
    idx: I,
    count: u64,
}

impl<R, I> ops::Deref for Indexed<R, I> {
    type Target = Reader<R>;
    fn deref(&self) -> &Reader<R> { &self.rdr }
}

impl<R, I> ops::DerefMut for Indexed<R, I> {
    fn deref_mut(&mut self) -> &mut Reader<R> { &mut self.rdr }
}

impl<R, I> Indexed<R, I> where R: io::Read + io::Seek, I: io::Read + io::Seek {
    /// Opens a new index corresponding to the CSV reader given.
    ///
    /// If the CSV reader has headers enabled, they are read first.
    ///
    /// Note that there are no checks in place to make sure the index
    /// accurately represents the CSV reader given.
    pub fn open(mut rdr: Reader<R>, mut idx: I) -> Result<Indexed<R, I>> {
        try!(idx.seek(io::SeekFrom::End(-8)));
        let mut count = try!(idx.read_u64::<BigEndian>());
        if rdr.has_headers && count > 0 {
            count -= 1;
            let _ = try!(rdr.byte_headers());
        }
        Ok(Indexed {
            rdr: rdr,
            idx: idx,
            count: count,
        })
    }

    /// Seeks to `i`th record.
    ///
    /// This uses zero-based indexing, so seeking to the `0`th record will read
    /// the first record. (The first record may not be the first written row
    /// in the CSV data if the underlying reader has headers enabled.)
    ///
    /// An error is returned if the index given is greater than or equal to the
    /// number of records in the index.
    pub fn seek(&mut self, mut i: u64) -> Result<()> {
        if i >= self.count {
            return Err(Error::Index(format!(
                "Record index {} is out of bounds. (There are {} records.)",
                i, self.count)));
        }
        // If the underlying reader has headers enabled, then we should offset
        // the index appropriately.
        if self.rdr.has_headers {
            i += 1;
        }
        // 1. Seek the index.
        // 2. Read the corresponding offset.
        // 3. Seek the CSV reader.
        try!(self.idx.seek(io::SeekFrom::Start(i * 8)));
        let offset = try!(self.idx.read_u64::<BigEndian>());
        self.rdr.seek(offset)
    }

    /// Returns the number of CSV records in the index in `O(1)` time.
    pub fn count(&self) -> u64 {
        self.count
    }
}

/// Creates a new index for the given CSV reader.
///
/// The CSV data is read from `rdr` and the index is written to `wtr`.
pub fn create_index<R, W>(mut rdr: Reader<R>, mut wtr: W) -> Result<()>
        where R: io::Read + io::Seek, W: io::Write {
    // Seek to the beginning so that we get everything.
    try!(rdr.seek(0));
    let mut count = 0u64;
    while !rdr.done() {
        try!(wtr.write_u64::<BigEndian>(rdr.byte_offset()));
        loop {
            match rdr.next_bytes() {
                NextField::EndOfCsv => break,
                NextField::EndOfRecord => { count += 1; break; },
                NextField::Error(err) => return Err(err),
                NextField::Data(_) => {}
            }
        }
    }
    wtr.write_u64::<BigEndian>(count).map_err(From::from)
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};
    use Reader;

    type CsvReader = Reader<io::Cursor<Vec<u8>>>;
    type Bytes = io::Cursor<Vec<u8>>;
    type Indexed = super::Indexed<Bytes, Bytes>;

    fn index<S: Into<String>>(s: S) -> Indexed {
        index_with(s, |rdr| rdr, |rdr| rdr)
    }

    fn index_nh<S: Into<String>>(s: S) -> Indexed {
        let then = |rdr: CsvReader| rdr.has_headers(false);
        index_with(s, &then, &then)
    }

    fn index_with<S, F, G>(s: S, create: F, new: G) -> Indexed
            where S: Into<String>,
                  F: FnOnce(CsvReader) -> CsvReader,
                  G: FnOnce(CsvReader) -> CsvReader {
        let data = s.into();
        let mut idx_bytes = io::Cursor::new(vec![]);
        super::create_index(create(Reader::from_string(&*data)),
                            idx_bytes.by_ref()).unwrap();
        super::Indexed::open(new(Reader::from_string(data)),
                             idx_bytes).unwrap()
    }

    fn next(idx: &mut Indexed) -> Vec<String> {
        idx.records().next().unwrap().unwrap()
    }

    fn nth(idx: &mut Indexed, i: u64) -> Vec<String> {
        idx.seek(i).unwrap();
        next(idx)
    }

    #[test]
    fn headers_one_field() {
        let data = "\
h1
a
b
c
";
        let mut idx = index(data);
        assert_eq!(idx.count(), 3);

        assert_eq!(nth(&mut idx, 0), vec!["a"]);
        assert_eq!(nth(&mut idx, 1), vec!["b"]);
        assert_eq!(nth(&mut idx, 2), vec!["c"]);
    }

    #[test]
    fn headers_many_fields() {
        let data = "\
h1,h2,h3
a,b,c
d,e,f
g,h,i
";
        let mut idx = index(data);
        assert_eq!(idx.count(), 3);

        assert_eq!(nth(&mut idx, 0), vec!["a", "b", "c"]);
        assert_eq!(nth(&mut idx, 1), vec!["d", "e", "f"]);
        assert_eq!(nth(&mut idx, 2), vec!["g", "h", "i"]);
    }

    #[test]
    fn no_headers_one_field() {
        let data = "\
h1
a
b
c
";
        let mut idx = index_nh(data);
        assert_eq!(idx.count(), 4);

        assert_eq!(nth(&mut idx, 0), vec!["h1"]);
        assert_eq!(nth(&mut idx, 1), vec!["a"]);
        assert_eq!(nth(&mut idx, 2), vec!["b"]);
        assert_eq!(nth(&mut idx, 3), vec!["c"]);
    }

    #[test]
    fn no_headers_many_fields() {
        let data = "\
h1,h2,h3
a,b,c
d,e,f
g,h,i
";
        let mut idx = index_nh(data);
        assert_eq!(idx.count(), 4);

        assert_eq!(nth(&mut idx, 0), vec!["h1", "h2", "h3"]);
        assert_eq!(nth(&mut idx, 1), vec!["a", "b", "c"]);
        assert_eq!(nth(&mut idx, 2), vec!["d", "e", "f"]);
        assert_eq!(nth(&mut idx, 3), vec!["g", "h", "i"]);
    }

    #[test]
    fn switch_headers_one_field1() {
        let data = "\
h1
a
b
c
";
        let mut idx = index_with(data, |r| r.has_headers(false), |r| r);
        assert_eq!(idx.count(), 3);

        assert_eq!(nth(&mut idx, 0), vec!["a"]);
        assert_eq!(nth(&mut idx, 1), vec!["b"]);
        assert_eq!(nth(&mut idx, 2), vec!["c"]);
    }

    #[test]
    fn switch_headers_one_field2() {
        let data = "\
h1
a
b
c
";
        let mut idx = index_with(data, |r| r, |r| r.has_headers(false));
        assert_eq!(idx.count(), 4);

        assert_eq!(nth(&mut idx, 0), vec!["h1"]);
        assert_eq!(nth(&mut idx, 1), vec!["a"]);
        assert_eq!(nth(&mut idx, 2), vec!["b"]);
        assert_eq!(nth(&mut idx, 3), vec!["c"]);
    }

    #[test]
    fn headers_one_field_newlines() {
        let data = "




h1

a


b






c






";
        let mut idx = index(data);
        assert_eq!(idx.count(), 3);

        assert_eq!(nth(&mut idx, 0), vec!["a"]);
        assert_eq!(nth(&mut idx, 1), vec!["b"]);
        assert_eq!(nth(&mut idx, 2), vec!["c"]);
    }
}
