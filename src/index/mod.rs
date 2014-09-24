#![allow(missing_doc)]

use std::io::{mod, File};
use std::path::BytesContainer;

use {CsvResult, ErrIo, Reader};

pub struct Indexed<R, I> {
    rdr: Reader<R>,
    idx: I,
}

impl Indexed<File, File> {
    pub fn from_file(csv_path: &Path) -> CsvResult<Indexed<File, File>> {
        let rdr = try!(File::open(csv_path).map_err(ErrIo));
        let idx = try!(File::open(&idx_path(csv_path)).map_err(ErrIo));
        Ok(Indexed::new(rdr, idx))
    }
}

impl<R: io::Reader + io::Seek, I: io::Reader + io::Seek> Indexed<R, I> {
    pub fn new(rdr: R, idx: I) -> Indexed<R, I> {
        Indexed {
            rdr: Reader::from_reader(rdr).no_headers(),
            idx: idx,
        }
    }

    pub fn seek(&mut self, i: u64) -> CsvResult<()> {
        // Why does `seek` want an `i64`?
        try!(self.idx.seek((i * 8) as i64, io::SeekSet).map_err(ErrIo));
        let offset = try!(self.idx.read_be_u64().map_err(ErrIo));
        println!("OFFSET: {}", offset);
        self.rdr.seek(offset as i64, io::SeekSet)
    }

    pub fn csv<'a>(&'a mut self) -> &'a mut Reader<R> {
        &mut self.rdr
    }
}

pub fn create_file(csv_path: &Path) -> CsvResult<()> {
    create(File::open(csv_path), File::create(&idx_path(csv_path)))
}

pub fn create<R: io::Reader + io::Seek, W: io::Writer>
             (csv_rdr: R, mut idx_wtr: W) -> CsvResult<()> {
    let mut rdr = Reader::from_reader(csv_rdr);
    while !rdr.done() {
        println!("WRITING OFFSET: {}", rdr.byte_offset());
        try!(idx_wtr.write_be_u64(rdr.byte_offset()).map_err(ErrIo));
        for field in rdr { let _ = try!(field); }
    }
    Ok(())
}

fn idx_path(csv_path: &Path) -> Path {
    let mut p = csv_path.container_into_owned_bytes();
    p.push_all(".idx".as_bytes());
    Path::new(p)
}
