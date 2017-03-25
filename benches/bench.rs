#![feature(test)]

extern crate csv;
extern crate rustc_serialize;
extern crate test;

use std::io;

use rustc_serialize::Decodable;
use test::Bencher;

use csv::{Reader};

static CSV_DATA: &'static str = "./examples/data/bench.csv";
static CSV_DATA_GAME: &'static str = "./examples/data/game.csv";

#[derive(Debug, RustcDecodable, PartialEq)]
struct NFLRowOwned {
    gameid: String,
    qtr: i32,
    min: Option<i32>,
    sec: Option<i32>,
    off: String,
    def: String,
    down: Option<i32>,
    togo: Option<i32>,
    ydline: Option<i32>,
    description: String,
    offscore: i32,
    defscore: i32,
    season: i32,
}

#[derive(Debug, RustcDecodable, PartialEq)]
struct GAMERowOwned(String, String, String, String, i32, String);

#[derive(Debug, RustcDecodable, PartialEq)]
struct POPRowOwned {
    country: String,
    city: String,
    accent_city: String,
    region: String,
    population: Option<i32>,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, RustcDecodable, PartialEq)]
struct MBTARowOwned {
    trip_id: String,
    arrival_time: String,
    departure_time: String,
    stop_id: String,
    stop_sequence: i32,
    stop_headsign: String,
    pickup_type: i32,
    drop_off_type: i32,
    timepoint: i32,
}

#[bench]
fn raw_records_nfl(b: &mut Bencher) {
    let mut data = file_to_mem(CSV_DATA);
    b.bytes = data.get_ref().len() as u64;
    b.iter(|| {
        let mut dec = reader(&mut data);
        while !dec.done() {
            while let Some(r) = dec.next_bytes().into_iter_result() {
                r.unwrap();
            }
        }
    };
}

#[bench]
fn raw_records_game(b: &mut Bencher) {
    let mut data = file_to_mem(CSV_DATA_GAME);
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

bench_decode!(
    count_nfl_deserialize_owned_bytes,
    NFL, count_deserialize_owned, NFLRowOwned, 9999);
bench_decode!(
    count_nfl_deserialize_owned_str,
    NFL, count_deserialize_owned, NFLRowOwned, 9999);
bench!(count_nfl_iter_bytes, NFL, count_iter_bytes, 130000);
bench!(count_nfl_iter_str, NFL, count_iter_str, 130000);
bench!(count_nfl_read_bytes, NFL, count_read_bytes, 130000);
bench!(count_nfl_read_str, NFL, count_read_str, 130000);
bench_decode!(
    no_headers,
    count_game_deserialize_owned_bytes,
    GAME, count_deserialize_owned, GAMERowOwned, 100000);
bench_decode!(
    no_headers,
    count_game_deserialize_owned_str,
    GAME, count_deserialize_owned, GAMERowOwned, 100000);
bench!(count_game_iter_bytes, GAME, count_iter_bytes, 600000);
bench!(count_game_iter_str, GAME, count_iter_str, 600000);
bench!(count_game_read_bytes, GAME, count_read_bytes, 600000);
bench!(count_game_read_str, GAME, count_read_str, 600000);
bench_decode!(
    count_pop_deserialize_owned_bytes,
    POP, count_deserialize_owned, POPRowOwned, 20000);
bench_decode!(
    count_pop_deserialize_owned_str,
    POP, count_deserialize_owned, POPRowOwned, 20000);
bench!(count_pop_iter_bytes, POP, count_iter_bytes, 140007);
bench!(count_pop_iter_str, POP, count_iter_str, 140007);
bench!(count_pop_read_bytes, POP, count_read_bytes, 140007);
bench!(count_pop_read_str, POP, count_read_str, 140007);
bench_decode!(
    count_mbta_deserialize_owned_bytes,
    MBTA, count_deserialize_owned, MBTARowOwned, 9999);
bench_decode!(
    count_mbta_deserialize_owned_str,
    MBTA, count_deserialize_owned, MBTARowOwned, 9999);
bench!(count_mbta_iter_bytes, MBTA, count_iter_bytes, 90000);
bench!(count_mbta_iter_str, MBTA, count_iter_str, 90000);
bench!(count_mbta_read_bytes, MBTA, count_read_bytes, 90000);
bench!(count_mbta_read_str, MBTA, count_read_str, 90000);

fn count_deserialize_owned<R, D>(rdr: &mut Reader<R>) -> u64
    where R: io::Read, D: Decodable
{
    let mut count = 0;
    for result in rdr.decode() {
        let _: D = result.unwrap();
        count += 1;
    }
    count
}

fn count_iter_bytes<R: io::Read>(rdr: &mut Reader<R>) -> u64 {
    let mut count = 0;
    for rec in rdr.byte_records() {
        count += rec.unwrap().len() as u64;
    }
    count
}

fn count_iter_str<R: io::Read>(rdr: &mut Reader<R>) -> u64 {
    let mut count = 0;
    for rec in rdr.records() {
        count += rec.unwrap().len() as u64;
    }
    count
}

fn count_read_bytes<R: io::Read>(rdr: &mut Reader<R>) -> u64 {
    let mut count = 0;
    while !rdr.done() {
        while let Some(r) = rdr.next_bytes().into_iter_result() {
            r.unwrap();
            count += 1;
        }
    }
    count
}

fn count_read_str<R: io::Read>(rdr: &mut Reader<R>) -> u64 {
    let mut count = 0;
    while !rdr.done() {
        while let Some(r) = rdr.next_str().into_iter_result() {
            r.unwrap();
            count += 1;
        }
    }
    count
}
