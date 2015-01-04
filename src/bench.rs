use std::fmt::Show;
use std::io::{self, ByRefReader};
use std::io::Reader as IoReader;
use stdtest::Bencher;

use Reader;

static CSV_DATA: &'static str = "./examples/data/bench.csv";

fn ordie<T, E: Show>(r: Result<T, E>) -> T {
    r.or_else(|e: E| -> Result<T, E> panic!(e.to_string())).unwrap()
}

fn file_to_mem(fp: &str) -> io::MemReader {
    use std::path::Path;

    let mut f = ordie(io::File::open(&Path::new(fp)));
    let bs = ordie(f.read_to_end());
    io::MemReader::new(bs)
}

fn reader<'a>(rdr: &'a mut io::MemReader)
             -> Reader<io::RefReader<'a, io::MemReader>> {
    let _ = ordie(rdr.seek(0, io::SeekSet));
    Reader::from_reader(rdr.by_ref())
}

#[bench]
fn raw_records(b: &mut Bencher) {
    let mut data = file_to_mem(CSV_DATA);
    b.bytes = data.get_ref().len() as u64;
    b.iter(|| {
        let mut dec = reader(&mut data);
        while !dec.done() {
            while let Some(r) = dec.next_field().into_iter_result() {
                r.unwrap();
            }
        }
    })
}

#[bench]
fn byte_records(b: &mut Bencher) {
    let mut data = file_to_mem(CSV_DATA);
    b.bytes = data.get_ref().len() as u64;
    b.iter(|| {
        let mut dec = reader(&mut data);
        for r in dec.byte_records() { let _ = r.unwrap(); }
    })
}

#[bench]
fn string_records(b: &mut Bencher) {
    let mut data = file_to_mem(CSV_DATA);
    b.bytes = data.get_ref().len() as u64;
    b.iter(|| {
        let mut dec = reader(&mut data);
        for r in dec.records() { let _ = r.unwrap(); }
    })
}

#[allow(dead_code)]
#[derive(RustcDecodable)]
struct Play {
    gameid: String,
    qtr: int,
    min: Option<int>,
    sec: Option<int>,
    team_off: String,
    team_def: String,
    down: Option<int>,
    togo: Option<int>,
    ydline: Option<int>,
    description: String,
    offscore: int,
    defscore: int,
    season: int,
}

#[bench]
fn decoded_records(b: &mut Bencher) {
    let mut data = file_to_mem(CSV_DATA);
    b.bytes = data.get_ref().len() as u64;
    b.iter(|| {
        let mut dec = reader(&mut data);
        for r in dec.decode::<Play>() { let _ = r.unwrap(); }
    })
}
