use std::io;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

/// A simple index for random access to CSV records.
///
/// This index permits seeking to the start of any CSV record with a constant
/// number of operations.
///
/// The format of the index is simplistic and amenable to serializing to disk.
/// It consists of exactly `N+1` 64 bit big-endian integers, where `N` is the
/// number of records in the CSV data that is indexed. Each `i`th integer
/// corresponds to the approximate byte offset where the `i`th record in the
/// CSV data begins. One additional integer is written to the end of the index
/// which indicates the total number of records in the CSV data.
///
/// This indexing format does not store the line numbers of CSV records, so
/// using the positions returned by this index to seek a CSV reader will likely
/// cause any future line numbers reported by that reader to be incorrect.
///
/// This format will never change.
///
/// N.B. The format of this indexing scheme matches the format of the old the
/// `csv::Indexed` type in pre-1.0 versions of the `csv` crate.
pub struct RandomAccessSimple<R> {
    rdr: R,
    len: u64,
}

impl<W: io::Write> RandomAccessSimple<W> {
    /// Write a simple index to the given writer for the given CSV reader.
    ///
    /// If there was a problem reading CSV records or writing to the given
    /// writer, then an error is returned.
    ///
    /// That the given CSV reader is read as given until EOF. The index
    /// produced includes all records, including the first record even if the
    /// CSV reader is configured to interpret the first record as a header
    /// record.
    ///
    /// # Example: in memory index
    ///
    /// This example shows how to create a simple random access index, open it
    /// and query the number of records in the index.
    ///
    /// ```
    /// use std::io;
    /// use csv_index::RandomAccessSimple;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> csv::Result<()> {
    ///     let data = "\
    /// city,country,pop
    /// Boston,United States,4628910
    /// Concord,United States,42695
    /// ";
    ///     let mut rdr = csv::Reader::from_reader(data.as_bytes());
    ///     let mut wtr = io::Cursor::new(vec![]);
    ///     RandomAccessSimple::create(&mut rdr, &mut wtr)?;
    ///
    ///     let idx = RandomAccessSimple::open(wtr)?;
    ///     assert_eq!(idx.len(), 3);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Example: file backed index
    ///
    /// This is like the previous example, but instead of creating the index
    /// in memory with `std::io::Cursor`, we write the index to a file.
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use std::io;
    /// use csv_index::RandomAccessSimple;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> csv::Result<()> {
    ///     let data = "\
    /// city,country,pop
    /// Boston,United States,4628910
    /// Concord,United States,42695
    /// ";
    ///     let mut rdr = csv::Reader::from_reader(data.as_bytes());
    ///     let mut wtr = File::create("data.csv.idx")?;
    ///     RandomAccessSimple::create(&mut rdr, &mut wtr)?;
    ///
    ///     let fileidx = File::open("data.csv.idx")?;
    ///     let idx = RandomAccessSimple::open(fileidx)?;
    ///     assert_eq!(idx.len(), 3);
    ///     Ok(())
    /// }
    /// ```
    pub fn create<R: io::Read>(
        rdr: &mut csv::Reader<R>,
        mut wtr: W,
    ) -> csv::Result<()> {
        // If the reader is configured to read a header, then read that
        // first. (The CSV reader otherwise won't yield the header record
        // when calling `read_byte_record`.)
        let mut len = 0;
        if rdr.has_headers() {
            let header = rdr.byte_headers()?;
            if !header.is_empty() {
                let pos = header.position().expect("position on header row");
                wtr.write_u64::<BigEndian>(pos.byte())?;
                len += 1;
            }
        }
        let mut record = csv::ByteRecord::new();
        while rdr.read_byte_record(&mut record)? {
            let pos = record.position().expect("position on row");
            wtr.write_u64::<BigEndian>(pos.byte())?;
            len += 1;
        }
        wtr.write_u64::<BigEndian>(len)?;
        Ok(())
    }
}

impl<R: io::Read + io::Seek> RandomAccessSimple<R> {
    /// Open an existing simple CSV index.
    ///
    /// The reader given must be seekable and should contain an index written
    /// by `RandomAccessSimple::create`.
    ///
    /// # Example
    ///
    /// This example shows how to create a simple random access index, open it
    /// and query the number of records in the index.
    ///
    /// ```
    /// use std::io;
    /// use csv_index::RandomAccessSimple;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> csv::Result<()> {
    ///     let data = "\
    /// city,country,pop
    /// Boston,United States,4628910
    /// Concord,United States,42695
    /// ";
    ///     let mut rdr = csv::Reader::from_reader(data.as_bytes());
    ///     let mut wtr = io::Cursor::new(vec![]);
    ///     RandomAccessSimple::create(&mut rdr, &mut wtr)?;
    ///
    ///     let idx = RandomAccessSimple::open(wtr)?;
    ///     assert_eq!(idx.len(), 3);
    ///     Ok(())
    /// }
    /// ```
    pub fn open(mut rdr: R) -> csv::Result<RandomAccessSimple<R>> {
        rdr.seek(io::SeekFrom::End(-8))?;
        let len = rdr.read_u64::<BigEndian>()?;
        Ok(RandomAccessSimple { rdr, len })
    }

    /// Get the position of the record at index `i`.
    ///
    /// The first record has index `0`.
    ///
    /// If the position returned is used to seek the CSV reader that was used
    /// to create this index, then the next record read by the CSV reader will
    /// be the `i`th record.
    ///
    /// Note that since this index does not store the line number of each
    /// record, the position returned will always have a line number equivalent
    /// to `1`. This in turn will cause the CSV reader to report all subsequent
    /// line numbers incorrectly.
    ///
    /// # Example
    ///
    /// This example shows how to create a simple random access index, open it
    /// and use it to seek a CSV reader to read an arbitrary record.
    ///
    /// ```
    /// use std::error::Error;
    /// use std::io;
    /// use csv_index::RandomAccessSimple;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<dyn Error>> {
    ///     let data = "\
    /// city,country,pop
    /// Boston,United States,4628910
    /// Concord,United States,42695
    /// ";
    ///     // Note that we wrap our CSV data in an io::Cursor, which makes it
    ///     // seekable. If you're opening CSV data from a file, then this is
    ///     // not needed since a `File` is already seekable.
    ///     let mut rdr = csv::Reader::from_reader(io::Cursor::new(data));
    ///     let mut wtr = io::Cursor::new(vec![]);
    ///     RandomAccessSimple::create(&mut rdr, &mut wtr)?;
    ///
    ///     // Open the index we just created, get the position of the last
    ///     // record and seek the CSV reader.
    ///     let mut idx = RandomAccessSimple::open(wtr)?;
    ///     let pos = idx.get(2)?;
    ///     rdr.seek(pos)?;
    ///
    ///     // Read the next record.
    ///     if let Some(result) = rdr.records().next() {
    ///         let record = result?;
    ///         assert_eq!(record, vec!["Concord", "United States", "42695"]);
    ///         Ok(())
    ///     } else {
    ///         Err(From::from("expected at least one record but got none"))
    ///     }
    /// }
    /// ```
    pub fn get(&mut self, i: u64) -> csv::Result<csv::Position> {
        if i >= self.len {
            let msg = format!(
                "invalid record index {} (there are {} records)",
                i, self.len
            );
            let err = io::Error::new(io::ErrorKind::Other, msg);
            return Err(csv::Error::from(err));
        }
        self.rdr.seek(io::SeekFrom::Start(i * 8))?;
        let offset = self.rdr.read_u64::<BigEndian>()?;
        let mut pos = csv::Position::new();
        pos.set_byte(offset).set_record(i);
        Ok(pos)
    }

    /// Return the number of records (including the header record) in this
    /// index.
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Return true if and only if this index has zero records.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use csv;

    use super::RandomAccessSimple;

    struct Indexed<'a> {
        csv: csv::Reader<io::Cursor<&'a str>>,
        idx: RandomAccessSimple<io::Cursor<Vec<u8>>>,
    }

    impl<'a> Indexed<'a> {
        fn new(headers: bool, csv_data: &'a str) -> Indexed<'a> {
            let mut rdr = csv::ReaderBuilder::new()
                .has_headers(headers)
                .from_reader(io::Cursor::new(csv_data));
            let mut idxbuf = io::Cursor::new(vec![]);
            RandomAccessSimple::create(&mut rdr, &mut idxbuf).unwrap();
            Indexed {
                csv: rdr,
                idx: RandomAccessSimple::open(idxbuf).unwrap(),
            }
        }

        fn read_at(&mut self, record: u64) -> csv::StringRecord {
            let pos = self.idx.get(record).unwrap();
            self.csv.seek(pos).unwrap();
            self.csv.records().next().unwrap().unwrap()
        }
    }

    #[test]
    fn headers_empty() {
        let idx = Indexed::new(true, "");
        assert_eq!(idx.idx.len(), 0);
    }

    #[test]
    fn headers_one_field() {
        let mut idx = Indexed::new(true, "h1\na\nb\nc\n");
        assert_eq!(idx.idx.len(), 4);
        assert_eq!(idx.read_at(0), vec!["h1"]);
        assert_eq!(idx.read_at(1), vec!["a"]);
        assert_eq!(idx.read_at(2), vec!["b"]);
        assert_eq!(idx.read_at(3), vec!["c"]);
    }

    #[test]
    fn headers_many_fields() {
        let mut idx = Indexed::new(
            true,
            "\
h1,h2,h3
a,b,c
d,e,f
g,h,i
",
        );
        assert_eq!(idx.idx.len(), 4);
        assert_eq!(idx.read_at(0), vec!["h1", "h2", "h3"]);
        assert_eq!(idx.read_at(1), vec!["a", "b", "c"]);
        assert_eq!(idx.read_at(2), vec!["d", "e", "f"]);
        assert_eq!(idx.read_at(3), vec!["g", "h", "i"]);
    }

    #[test]
    fn no_headers_one_field() {
        let mut idx = Indexed::new(false, "h1\na\nb\nc\n");
        assert_eq!(idx.idx.len(), 4);
        assert_eq!(idx.read_at(0), vec!["h1"]);
        assert_eq!(idx.read_at(1), vec!["a"]);
        assert_eq!(idx.read_at(2), vec!["b"]);
        assert_eq!(idx.read_at(3), vec!["c"]);
    }

    #[test]
    fn no_headers_many_fields() {
        let mut idx = Indexed::new(
            false,
            "\
h1,h2,h3
a,b,c
d,e,f
g,h,i
",
        );
        assert_eq!(idx.idx.len(), 4);
        assert_eq!(idx.read_at(0), vec!["h1", "h2", "h3"]);
        assert_eq!(idx.read_at(1), vec!["a", "b", "c"]);
        assert_eq!(idx.read_at(2), vec!["d", "e", "f"]);
        assert_eq!(idx.read_at(3), vec!["g", "h", "i"]);
    }

    #[test]
    fn headers_one_field_newlines() {
        let mut idx = Indexed::new(
            true,
            "




h1

a


b






c






",
        );
        assert_eq!(idx.idx.len(), 4);
        assert_eq!(idx.read_at(0), vec!["h1"]);
        assert_eq!(idx.read_at(1), vec!["a"]);
        assert_eq!(idx.read_at(2), vec!["b"]);
        assert_eq!(idx.read_at(3), vec!["c"]);
    }
}
