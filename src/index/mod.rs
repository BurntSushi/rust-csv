#![allow(missing_doc)]

use std::io;

use {CsvResult, ErrIo, Reader};

pub struct Indexed<R, I> {
    rdr: Reader<R>,
    idx: I,
}

impl<R: io::Reader + io::Seek, I: io::Reader + io::Seek> Indexed<R, I> {
    pub fn new(rdr: Reader<R>, idx: I) -> Indexed<R, I> {
        Indexed {
            rdr: rdr,
            idx: idx,
        }
    }

    pub fn seek(&mut self, mut i: u64) -> CsvResult<()> {
        if self.rdr.has_headers {
            i += 1;
        }
        // Why does `seek` want an `i64`?
        try!(self.idx.seek((i * 8) as i64, io::SeekSet).map_err(ErrIo));
        let offset = try!(self.idx.read_be_u64().map_err(ErrIo));
        self.rdr.seek(offset as i64, io::SeekSet)
    }

    pub fn csv<'a>(&'a mut self) -> &'a mut Reader<R> {
        &mut self.rdr
    }
}

pub fn create<R: io::Reader + io::Seek, W: io::Writer>
             (csv_rdr: Reader<R>, mut idx_wtr: W) -> CsvResult<()> {
    let mut rdr = csv_rdr.has_headers(false);
    while !rdr.done() {
        try!(idx_wtr.write_be_u64(rdr.byte_offset()).map_err(ErrIo));
        loop {
            match rdr.next_field() {
                None => break,
                Some(r) => { try!(r); }
            }
        }
    }
    Ok(())
}
