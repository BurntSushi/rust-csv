use std::fmt::Show;
use std::io;
use stdtest::BenchHarness;
use super::Decoder;

static CSV_SHORT: &'static str = "./examples/data/short.csv";
static CSV_MEDIUM: &'static str = "./examples/data/medium.csv";
static CSV_LARGE: &'static str = "./examples/data/large.csv";

fn ordie<T, E: Show>(r: Result<T, E>) -> T {
    r.or_else(|e: E| -> Result<T, E> fail!(e.to_str())).unwrap()
}

fn file_to_mem(fp: &str) -> io::MemReader {
    use std::path::Path;

    let mut f = ordie(io::File::open(&Path::new(fp)));
    let bs = ordie(f.read_to_end());
    io::MemReader::new(bs)
}

#[bench]
fn short_raw_records(b: &mut BenchHarness) {
    let mut data = file_to_mem(CSV_SHORT);
    b.iter(|| {
        let _ = ordie(data.seek(0, io::SeekSet));
        let mut dec = Decoder::from_reader(&mut data as &mut io::Reader);
        for _ in dec.iter() {}
    })
}

#[bench]
fn medium_raw_records(b: &mut BenchHarness) {
    let mut data = file_to_mem(CSV_MEDIUM);
    b.iter(|| {
        let _ = ordie(data.seek(0, io::SeekSet));
        let mut dec = Decoder::from_reader(&mut data as &mut io::Reader);
        for _ in dec.iter() {}
    })
}

#[bench]
fn large_raw_records(b: &mut BenchHarness) {
    let mut data = file_to_mem(CSV_LARGE);
    b.iter(|| {
        let _ = ordie(data.seek(0, io::SeekSet));
        let mut dec = Decoder::from_reader(&mut data as &mut io::Reader);
        for _ in dec.iter() {}
    })
}

#[deriving(Decodable)]
struct Play {
    gameid: ~str,
    qtr: uint,
    min: Option<uint>,
    sec: Option<uint>,
    team_off: ~str,
    team_def: ~str,
    down: Option<uint>,
    togo: Option<uint>,
    ydline: Option<uint>,
    description: ~str,
    offscore: uint,
    defscore: uint,
    season: uint,
}

#[bench]
fn short_decoded_records(b: &mut BenchHarness) {
    let mut data = file_to_mem(CSV_SHORT);
    b.iter(|| {
        let _ = ordie(data.seek(0, io::SeekSet));
        let mut dec = Decoder::from_reader(&mut data as &mut io::Reader);
        dec.has_headers(true);
        match dec.decode_all::<Play>() {
            Ok(_) => {}
            Err(err) => fail!("{}", err),
        }
    })
}
