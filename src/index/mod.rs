#![allow(missing_docs)]

use std::io;

use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};

use {Result, Error, Reader, NextField};

pub struct Indexed<R, I> {
    rdr: Reader<R>,
    idx: I,
    count: u64,
}

impl<R: io::Read + io::Seek, I: io::Read + io::Seek> Indexed<R, I> {
    pub fn new(mut rdr: Reader<R>, mut idx: I) -> Result<Indexed<R, I>> {
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

    pub fn seek(&mut self, mut i: u64) -> Result<()> {
        if i >= self.count {
            return Err(Error::Index(format!(
                "Record index {} is out of bounds. (There are {} records.)",
                i, self.count)));
        }
        if self.rdr.has_headers {
            i += 1;
        }
        try!(self.idx.seek(io::SeekFrom::Start(i * 8)));
        let offset = try!(self.idx.read_u64::<BigEndian>());
        self.rdr.seek(offset)
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn csv<'a>(&'a mut self) -> &'a mut Reader<R> {
        &mut self.rdr
    }
}

pub fn create<R: io::Read + io::Seek, W: io::Write>
             (mut csv_rdr: Reader<R>, mut idx_wtr: W) -> Result<()> {
    // Seek to the beginning so that we get everything.
    try!(csv_rdr.seek(0));
    let mut count = 0u64;
    while !csv_rdr.done() {
        try!(idx_wtr.write_u64::<BigEndian>(csv_rdr.byte_offset()));
        loop {
            match csv_rdr.next_field() {
                NextField::EndOfCsv => break,
                NextField::EndOfRecord => { count += 1; break; },
                NextField::Error(err) => return Err(err),
                NextField::Data(_) => {}
            }
        }
    }
    idx_wtr.write_u64::<BigEndian>(count).map_err(From::from)
}
