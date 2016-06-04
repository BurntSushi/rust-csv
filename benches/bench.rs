#![feature(test)]

#![cfg_attr(feature = "serde-test", feature(custom_derive, plugin))]
#![cfg_attr(feature = "serde-test", plugin(serde_macros))]

extern crate csv;

#[cfg(feature = "rustc-serialize")]
extern crate rustc_serialize;

extern crate test;

use std::fmt::{Debug, Display};
use std::fs;
use std::io::{self, Read, Seek};
use test::Bencher;

use csv::Reader;

static CSV_DATA: &'static str = "./examples/data/bench.csv";

fn ordie<T, E: Debug+Display>(r: Result<T, E>) -> T {
    r.or_else(|e: E| -> Result<T, E> { panic!(format!("{:?}", e)) }).unwrap()
}

fn file_to_mem(fp: &str) -> io::Cursor<Vec<u8>> {
    let mut f = ordie(fs::File::open(fp));
    let mut bs = vec![];
    ordie(f.read_to_end(&mut bs));
    io::Cursor::new(bs)
}

fn reader<'a>(rdr: &'a mut io::Cursor<Vec<u8>>)
             -> Reader<&'a mut io::Cursor<Vec<u8>>> {
    let _ = ordie(rdr.seek(io::SeekFrom::Start(0)));
    Reader::from_reader(rdr.by_ref())
}

#[bench]
fn raw_records(b: &mut Bencher) {
    let mut data = file_to_mem(CSV_DATA);
    b.bytes = data.get_ref().len() as u64;
    b.iter(|| {
        let mut dec = reader(&mut data);
        while !dec.done() {
            while let Some(r) = dec.next_bytes().into_iter_result() {
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
#[cfg_attr(feature = "rustc-serialize", derive(RustcDecodable))]
#[cfg_attr(feature = "serde", derive(Deserialize))]
struct Play {
    gameid: String,
    qtr: i32,
    min: Option<i32>,
    sec: Option<i32>,
    team_off: String,
    team_def: String,
    down: Option<i32>,
    togo: Option<i32>,
    ydline: Option<i32>,
    description: String,
    offscore: i32,
    defscore: i32,
    season: i32,
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
