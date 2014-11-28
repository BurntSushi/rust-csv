#![allow(missing_docs)]

use std::error::FromError;
use std::io;

use {CsvResult, Error, Reader, NextField};

pub struct Indexed<R, I> {
    rdr: Reader<R>,
    idx: I,
    count: u64,
}

impl<R: io::Reader + io::Seek, I: io::Reader + io::Seek> Indexed<R, I> {
    pub fn new(mut rdr: Reader<R>, mut idx: I) -> CsvResult<Indexed<R, I>> {
        try!(idx.seek(-8, io::SeekEnd));
        let mut count = try!(idx.read_be_u64());
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
        try!(self.idx.seek((i * 8) as i64, io::SeekSet));
        let offset = try!(self.idx.read_be_u64());
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
             (mut csv_rdr: Reader<R>, mut idx_wtr: W) -> CsvResult<()> {
    // Seek to the beginning so that we get everything.
    try!(csv_rdr.seek(0, ::std::io::SeekSet));
    let mut count = 0u64;
    while !csv_rdr.done() {
        try!(idx_wtr.write_be_u64(csv_rdr.byte_offset()));
        loop {
            match csv_rdr.next_field() {
                NextField::EndOfCsv => break,
                NextField::EndOfRecord => { count += 1; break; },
                NextField::Error(err) => return Err(err),
                NextField::Data(_) => {}
            }
        }
    }
    idx_wtr.write_be_u64(count).map_err(FromError::from_error)
}
