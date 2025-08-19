#![no_main]
use libfuzzer_sys::fuzz_target;
use csv::Reader;

fuzz_target!(|data: &[u8]| {
    let mut rdr = Reader::from_reader(data);
});
