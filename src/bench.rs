use std::path::Path;
use stdtest::BenchHarness;
use super::Decoder;

static CSV_SHORT: &'static str = "./examples/data/short.csv";

#[bench]
fn short_raw_records(b: &mut BenchHarness) {
    let fp = &Path::new(CSV_SHORT);
    b.iter(|| {
        let mut dec = Decoder::from_file(fp);
        for _ in dec {}
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
    let fp = &Path::new(CSV_SHORT);
    b.iter(|| {
        let mut dec = Decoder::from_file(fp);
        dec.has_headers(true);
        match dec.decode_all::<Play>() {
            Ok(_) => {}
            Err(err) => fail!("{}", err),
        }
    })
}
