#![feature(test)]

extern crate csv_core;
extern crate test;

use test::Bencher;

use csv_core::{Reader, ReaderBuilder, ReadResult};

static NFL: &'static str = include_str!("../../examples/data/bench.csv");
static GAME: &'static str = include_str!("../../examples/data/game.csv");

#[bench]
fn count_nfl_nocopy(b: &mut Bencher) {
    let data = NFL.as_bytes();
    b.bytes = data.len() as u64;
    let mut builder = ReaderBuilder::new();
    let mut rdr = builder.copy(false).build();
    b.iter(|| {
        rdr.reset();
        assert_eq!(count_fields(&mut rdr, data), 130000);
    })
}

#[bench]
fn count_nfl_copy(b: &mut Bencher) {
    let data = NFL.as_bytes();
    b.bytes = data.len() as u64;
    let mut rdr = Reader::new();
    b.iter(|| {
        rdr.reset();
        assert_eq!(count_fields(&mut rdr, data), 130000);
    })
}

#[bench]
fn count_game_nocopy(b: &mut Bencher) {
    let data = GAME.as_bytes();
    b.bytes = data.len() as u64;
    let mut builder = ReaderBuilder::new();
    let mut rdr = builder.copy(false).build();
    b.iter(|| {
        rdr.reset();
        assert_eq!(count_fields(&mut rdr, data), 600000);
    })
}

#[bench]
fn count_game_copy(b: &mut Bencher) {
    let data = GAME.as_bytes();
    b.bytes = data.len() as u64;
    let mut rdr = Reader::new();
    b.iter(|| {
        rdr.reset();
        assert_eq!(count_fields(&mut rdr, data), 600000);
    })
}

fn count_fields(rdr: &mut Reader, mut data: &[u8]) -> u64 {
    let mut count = 0;
    let mut field = [0u8; 1024];
    loop {
        let (res, nin, _) = rdr.read(data, &mut field);
        data = &data[nin..];
        match res {
            ReadResult::InputEmpty => {}
            ReadResult::OutputFull => panic!("field too large"),
            ReadResult::Field{..} => { count += 1; }
            ReadResult::End => break,
        }
    }
    count
}
