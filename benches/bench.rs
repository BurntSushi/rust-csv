#![feature(test)]

extern crate csv;
extern crate test;

use std::io;

use test::Bencher;

use csv::{ByteRecord, Reader, ReaderBuilder, ReadField};

static NFL: &'static str = include_str!("../examples/data/bench.csv");
static GAME: &'static str = include_str!("../examples/data/game.csv");

#[bench]
fn count_nfl_field_bytes(b: &mut Bencher) {
    let data = NFL.as_bytes();
    b.bytes = data.len() as u64;
    b.iter(|| {
        let mut rdr = ReaderBuilder::new().from_reader(data);
        assert_eq!(count_read_field_bytes(&mut rdr), 130000);
    })
}

#[bench]
fn count_game_field_bytes(b: &mut Bencher) {
    let data = GAME.as_bytes();
    b.bytes = data.len() as u64;
    b.iter(|| {
        let mut rdr = ReaderBuilder::new().from_reader(data);
        assert_eq!(count_read_field_bytes(&mut rdr), 600000);
    })
}

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

fn count_read_field_bytes<R: io::Read>(rdr: &mut Reader<R>) -> u64 {
    let mut count = 0;
    let mut field = Vec::with_capacity(1024);
    loop {
        match rdr.read_field_bytes(&mut field).unwrap() {
            ReadField::Field | ReadField::Record => { count += 1 }
            ReadField::End => break,
        }
    }
    count
}

fn count_read_record_bytes<R: io::Read>(rdr: &mut Reader<R>) -> u64 {
    let mut count = 0;
    let mut rec = ByteRecord::new();
    while rdr.read_record_bytes(&mut rec).unwrap() {
        count += 1;
    }
    count
}
