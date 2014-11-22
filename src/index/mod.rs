#![allow(missing_docs)]

use std::io;

use {CsvResult, Error, Reader, NextField};

pub struct Indexed<R, I> {
    rdr: Reader<R>,
    idx: I,
    count: u64,
}

impl<R: io::Reader + io::Seek, I: io::Reader + io::Seek> Indexed<R, I> {
    pub fn new(mut rdr: Reader<R>, mut idx: I) -> CsvResult<Indexed<R, I>> {
        try!(idx.seek(-8, io::SeekEnd).map_err(Error::Io));
        let mut count = try!(idx.read_be_u64().map_err(Error::Io));
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

    pub fn seek(&mut self, mut i: u64) -> CsvResult<()> {
        if i >= self.count {
            return Err(Error::Index(format!(
                "Record index {} is out of bounds. (There are {} records.)",
                i, self.count)));
        }
        if self.rdr.has_headers {
            i += 1;
        }
        try!(self.idx.seek((i * 8) as i64, io::SeekSet).map_err(Error::Io));
        let offset = try!(self.idx.read_be_u64().map_err(Error::Io));
        self.rdr.seek(offset as i64, io::SeekSet)
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn csv<'a>(&'a mut self) -> &'a mut Reader<R> {
        &mut self.rdr
    }
}

pub fn create<R: io::Reader + io::Seek, W: io::Writer>
             (csv_rdr: Reader<R>, mut idx_wtr: W) -> CsvResult<()> {
    let mut rdr = csv_rdr.has_headers(false);
    let mut count = 0u64;
    while !rdr.done() {
        try!(idx_wtr.write_be_u64(rdr.byte_offset()).map_err(Error::Io));
        loop {
            match rdr.next_field() {
                NextField::EndOfCsv => break,
                NextField::EndOfRecord => { count += 1; break; },
                NextField::Error(err) => return Err(err),
                NextField::Data(_) => {}
            }
        }
    }
    idx_wtr.write_be_u64(count).map_err(Error::Io)
}
