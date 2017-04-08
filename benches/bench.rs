#![feature(test)]

extern crate csv;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate test;

use std::io;

use serde::Deserialize;
use test::Bencher;

use csv::{ByteRecord, Reader, ReaderBuilder, StringRecord};

static NFL: &'static str = include_str!("../examples/data/nfl.csv");
static GAME: &'static str = include_str!("../examples/data/game.csv");
static POP: &'static str = include_str!("../examples/data/worldcitiespop.csv");
static MBTA: &'static str = include_str!("../examples/data/gtfs-mbta-stop-times.csv");

#[derive(Debug, Deserialize, PartialEq)]
struct NFLRow {
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

#[derive(Debug, Deserialize, PartialEq)]
struct GAMERow(String, String, String, String, i32, String);

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
struct POPRow {
    country: String,
    city: String,
    accent_city: String,
    region: String,
    population: Option<i32>,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize, PartialEq)]
struct MBTARow {
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

macro_rules! bench {
    ($name:ident, $data:ident, $counter:ident, $result:expr) => {
        #[bench]
        fn $name(b: &mut Bencher) {
            let data = $data.as_bytes();
            b.bytes = data.len() as u64;
            b.iter(|| {
                let mut rdr = ReaderBuilder::new()
                    .has_headers(false)
                    .from_reader(data);
                assert_eq!($counter(&mut rdr), $result);
            })
        }
    };
}

macro_rules! bench_serde {
    (no_headers,
     $name:ident, $data:ident, $counter:ident, $type:ty, $result:expr) => {
        #[bench]
        fn $name(b: &mut Bencher) {
            let data = $data.as_bytes();
            b.bytes = data.len() as u64;
            b.iter(|| {
                let mut rdr = ReaderBuilder::new()
                    .has_headers(false)
                    .from_reader(data);
                assert_eq!($counter::<_, $type>(&mut rdr), $result);
            })
        }
    };
    ($name:ident, $data:ident, $counter:ident, $type:ty, $result:expr) => {
        #[bench]
        fn $name(b: &mut Bencher) {
            let data = $data.as_bytes();
            b.bytes = data.len() as u64;
            b.iter(|| {
                let mut rdr = ReaderBuilder::new()
                    .has_headers(true)
                    .from_reader(data);
                assert_eq!($counter::<_, $type>(&mut rdr), $result);
            })
        }
    };
}

bench_serde!(
    count_nfl_deserialize_bytes, NFL, count_deserialize_bytes, NFLRow, 9999);
bench_serde!(
    count_nfl_deserialize_str, NFL, count_deserialize_str, NFLRow, 9999);
bench!(count_nfl_iter_bytes, NFL, count_iter_bytes, 10000);
bench!(count_nfl_iter_str, NFL, count_iter_str, 10000);
bench!(count_nfl_read_bytes, NFL, count_read_bytes, 10000);
bench!(count_nfl_read_str, NFL, count_read_str, 10000);
bench_serde!(
    no_headers,
    count_game_deserialize_bytes, GAME, count_deserialize_bytes, GAMERow, 100000);
bench_serde!(
    no_headers,
    count_game_deserialize_str, GAME, count_deserialize_str, GAMERow, 100000);
bench!(count_game_iter_bytes, GAME, count_iter_bytes, 100000);
bench!(count_game_iter_str, GAME, count_iter_str, 100000);
bench!(count_game_read_bytes, GAME, count_read_bytes, 100000);
bench!(count_game_read_str, GAME, count_read_str, 100000);
bench_serde!(
    count_pop_deserialize_bytes, POP, count_deserialize_bytes, POPRow, 20000);
bench_serde!(
    count_pop_deserialize_str, POP, count_deserialize_str, POPRow, 20000);
bench!(count_pop_iter_bytes, POP, count_iter_bytes, 20001);
bench!(count_pop_iter_str, POP, count_iter_str, 20001);
bench!(count_pop_read_bytes, POP, count_read_bytes, 20001);
bench!(count_pop_read_str, POP, count_read_str, 20001);
bench_serde!(
    count_mbta_deserialize_bytes, MBTA, count_deserialize_bytes, MBTARow, 9999);
bench_serde!(
    count_mbta_deserialize_str, MBTA, count_deserialize_str, MBTARow, 9999);
bench!(count_mbta_iter_bytes, MBTA, count_iter_bytes, 10000);
bench!(count_mbta_iter_str, MBTA, count_iter_str, 10000);
bench!(count_mbta_read_bytes, MBTA, count_read_bytes, 10000);
bench!(count_mbta_read_str, MBTA, count_read_str, 10000);

fn count_deserialize_bytes<R, D>(rdr: &mut Reader<R>) -> u64
    where R: io::Read, D: Deserialize
{
    let mut count = 0;
    let mut rec = ByteRecord::new();
    while !rdr.read_byte_record(&mut rec).unwrap() {
        let _: D = rec.deserialize(None).unwrap();
        count += 1;
    }
    count
}

fn count_deserialize_str<R, D>(rdr: &mut Reader<R>) -> u64
    where R: io::Read, D: Deserialize
{
    let mut count = 0;
    for rec in rdr.deserializer::<D>() {
        let _ = rec.unwrap();
        count += 1;
    }
    count
}

fn count_iter_bytes<R: io::Read>(rdr: &mut Reader<R>) -> u64 {
    let mut count = 0;
    for rec in rdr.byte_records() {
        let _ = rec.unwrap();
        count += 1;
    }
    count
}

fn count_iter_str<R: io::Read>(rdr: &mut Reader<R>) -> u64 {
    let mut count = 0;
    for rec in rdr.records() {
        let _ = rec.unwrap();
        count += 1;
    }
    count
}

fn count_read_bytes<R: io::Read>(rdr: &mut Reader<R>) -> u64 {
    let mut count = 0;
    let mut rec = ByteRecord::new();
    while !rdr.read_byte_record(&mut rec).unwrap() {
        count += 1;
    }
    count
}

fn count_read_str<R: io::Read>(rdr: &mut Reader<R>) -> u64 {
    let mut count = 0;
    let mut rec = StringRecord::new();
    while !rdr.read_record(&mut rec).unwrap() {
        count += 1;
    }
    count
}
