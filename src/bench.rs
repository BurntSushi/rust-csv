use std::path::Path;
use stdtest::BenchHarness;
use super::Decoder;

#[bench]
fn small_raw_records(b: &mut BenchHarness) {
    let fp = &Path::new("./data/2012_nfl_pbp_data.csv");
    b.iter(|| {
        let mut dec = Decoder::from_file(fp);
        for _ in dec {}
    })
}
