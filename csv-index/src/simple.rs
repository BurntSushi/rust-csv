use std::io;

use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use csv;

/// A simple index for random access to CSV records.
///
/// This index permits seeking to the start of any CSV record with a constant
/// number of operations.
///
/// The format of the index is simplistic and amenable to serializing to disk.
/// It consists of exactly `N+1` 64 bit big-endian integers, where `N` is the
/// number of records is the CSV data that is indexed. Each `i`th integer corresponds
/// to the approximate byte offset where the `i`th record in the CSV data
/// begins. One additional integer is written to the end of the index which
/// indicates the total number of records in the CSV data.
///
/// This format will never change.
///
/// N.B. The format of this indexing scheme matches the format of the old the
/// `csv::Indexed` type in pre-1.0 versions of the `csv` crate.
pub struct RandomAccessSimple<R> {
    rdr: R,
    len: u64,
}

impl<R: io::Read + io::Seek> RandomAccessSimple<R> {
    /// Write a simple index to the given writer.
    ///
    /// If there was a problem reading CSV records or writing to the given
    /// writer, then an error is returned.
    ///
    /// That the given CSV reader is read as given until EOF. The index
    /// produced includes all records, including the first record even if the
    /// CSV reader is configured to interpret the first record as a header
    /// record.
    pub fn create<C: io::Read, W: io::Write>(
        rdr: &mut csv::Reader<C>,
        mut wtr: W,
    ) -> csv::Result<()>
    {
        // If the reader is configured to read a header, then read that
        // first. (The CSV reader otherwise won't yield the header record
        // when calling `read_byte_record`.)
        let mut len = 0;
        if rdr.has_headers() {
            let header = rdr.byte_headers()?;
            let pos = header.position().expect("position on header row");
            wtr.write_u64::<BigEndian>(pos.byte())?;
            len += 1;
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

    /// Open an existing simple CSV index.
    ///
    /// The reader given must be seekable and should contain an index written
    /// by `RandomAccessSimple::create`.
    pub fn open(mut rdr: R) -> csv::Result<RandomAccessSimple<R>> {
        rdr.seek(io::SeekFrom::End(-8))?;
        let len = rdr.read_u64::<BigEndian>()?;
        Ok(RandomAccessSimple {
            rdr: rdr,
            len: len,
        })
    }

    /// Get the byte position of the record at index `i`.
    ///
    /// The first record has index `0`.
    ///
    /// If the byte position returned is used to seek the CSV reader that was
    /// used to create this index, then the next record read by the CSV reader
    /// will be the `i`th record.
    pub fn get(&mut self, i: u64) -> csv::Result<u64> {
        if i >= self.len {
            let msg = format!(
                "invalid record index {} (there are {} records)", i, self.len);
            let err = io::Error::new(io::ErrorKind::Other, msg);
            return Err(csv::Error::from(err));
        }
        self.rdr.seek(io::SeekFrom::Start(i * 8))?;
        let offset = self.rdr.read_u64::<BigEndian>()?;
        Ok(offset)
    }

    /// Return the number of records (including the header record) in this
    /// index.
    pub fn len(&self) -> u64 {
        self.len
    }
}
