#![feature(test)]

extern crate csv;
extern crate test;

use std::io;

use test::Bencher;

use csv::{ByteRecord, Reader, ReaderBuilder};

static NFL: &'static str = include_str!("../examples/data/nfl.csv");
static GAME: &'static str = include_str!("../examples/data/game.csv");
static POP: &'static str = include_str!("../examples/data/worldcitiespop.csv");

#[bench]
fn count_nfl_record_bytes(b: &mut Bencher) {
    let data = NFL.as_bytes();
    b.bytes = data.len() as u64;
    b.iter(|| {
        let mut rdr = ReaderBuilder::new().from_reader(data);
        assert_eq!(count_read_record_bytes(&mut rdr), 10000);
    })
}

#[bench]
fn count_game_record_bytes(b: &mut Bencher) {
    let data = GAME.as_bytes();
    b.bytes = data.len() as u64;
    b.iter(|| {
        let mut rdr = ReaderBuilder::new().from_reader(data);
        assert_eq!(count_read_record_bytes(&mut rdr), 100000);
    })
}

#[bench]
fn count_pop_record_bytes(b: &mut Bencher) {
    let data = POP.as_bytes();
    b.bytes = data.len() as u64;
    b.iter(|| {
        let mut rdr = ReaderBuilder::new().from_reader(data);
        assert_eq!(count_read_record_bytes(&mut rdr), 20001);
    })
}

fn count_read_record_bytes<R: io::Read>(rdr: &mut Reader<R>) -> u64 {
    let mut count = 0;
    let mut rec = ByteRecord::new();
    while rdr.read_record_bytes(&mut rec).unwrap() {
        count += 1;
    }
    count
}
