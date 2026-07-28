use std::fmt;
use std::hint::black_box;
use std::io;
use std::sync::OnceLock;

use csv::{
    ByteRecord, Reader, ReaderBuilder, StringRecord, Trim, Writer,
    WriterBuilder,
};
use gungraun::prelude::*;
use serde::{
    de::{DeserializeOwned, Deserializer, Visitor},
    Deserialize, Serialize,
};

static NFL: &str = include_str!("../../examples/data/bench/nfl.csv");
static GAME: &str = include_str!("../../examples/data/bench/game.csv");
static POP: &str =
    include_str!("../../examples/data/bench/worldcitiespop.csv");
static MBTA: &str =
    include_str!("../../examples/data/bench/gtfs-mbta-stop-times.csv");

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct NFLRowBorrowed<'a> {
    gameid: &'a str,
    qtr: i32,
    min: Option<i32>,
    sec: Option<i32>,
    off: &'a str,
    def: &'a str,
    down: Option<i32>,
    togo: Option<i32>,
    ydline: Option<i32>,
    description: &'a str,
    offscore: i32,
    defscore: i32,
    season: i32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct GAMERowOwned(String, String, String, String, i32, String);

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct GAMERowBorrowed<'a>(&'a str, &'a str, &'a str, &'a str, i32, &'a str);

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
struct POPRowOwned {
    country: String,
    city: String,
    accent_city: String,
    region: String,
    population: Option<i32>,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
struct POPRowBorrowed<'a> {
    country: &'a str,
    city: &'a str,
    accent_city: &'a str,
    region: &'a str,
    population: Option<i32>,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MBTARowBorrowed<'a> {
    trip_id: &'a str,
    arrival_time: &'a str,
    departure_time: &'a str,
    stop_id: &'a str,
    stop_sequence: i32,
    stop_headsign: &'a str,
    pickup_type: i32,
    drop_off_type: i32,
    timepoint: i32,
}

#[derive(Default)]
struct ByteCounter {
    count: usize,
}

struct AnyValue(u8);

impl<'de> Deserialize<'de> for AnyValue {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        deserializer.deserialize_any(AnyVisitor)
    }
}

struct AnyVisitor;

impl Visitor<'_> for AnyVisitor {
    type Value = AnyValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any CSV field")
    }

    fn visit_bool<E>(self, _: bool) -> Result<AnyValue, E> {
        Ok(AnyValue(1))
    }

    fn visit_u64<E>(self, _: u64) -> Result<AnyValue, E> {
        Ok(AnyValue(2))
    }

    fn visit_i64<E>(self, _: i64) -> Result<AnyValue, E> {
        Ok(AnyValue(3))
    }

    fn visit_u128<E>(self, _: u128) -> Result<AnyValue, E> {
        Ok(AnyValue(4))
    }

    fn visit_i128<E>(self, _: i128) -> Result<AnyValue, E> {
        Ok(AnyValue(5))
    }

    fn visit_f64<E>(self, _: f64) -> Result<AnyValue, E> {
        Ok(AnyValue(6))
    }

    fn visit_str<E>(self, _: &str) -> Result<AnyValue, E> {
        Ok(AnyValue(7))
    }

    fn visit_bytes<E>(self, _: &[u8]) -> Result<AnyValue, E> {
        Ok(AnyValue(8))
    }
}

impl io::Write for ByteCounter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.count += data.len();
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

macro_rules! bench_read {
    ($name:ident, $data:ident, $counter:ident, $result:expr, $trim:expr) => {
        #[library_benchmark]
        fn $name() -> u64 {
            let data = black_box($data.as_bytes());
            let mut builder = ReaderBuilder::new();
            builder.has_headers(false);
            if $trim {
                builder.trim(Trim::All);
            }
            let mut rdr = builder.from_reader(data);
            let result = $counter(&mut rdr);
            assert_eq!(result, $result);
            black_box(result)
        }
    };
}

macro_rules! bench_deserialize_owned {
    (
        $name:ident,
        $data:ident,
        $counter:ident,
        $type:ty,
        $result:expr,
        $headers:expr
    ) => {
        #[library_benchmark]
        fn $name() -> u64 {
            let data = black_box($data.as_bytes());
            let mut rdr =
                ReaderBuilder::new().has_headers($headers).from_reader(data);
            let result = $counter::<_, $type>(&mut rdr);
            assert_eq!(result, $result);
            black_box(result)
        }
    };
}

macro_rules! bench_deserialize_borrowed_bytes {
    ($name:ident, $data:ident, $type:ty, $headers:expr, $result:expr) => {
        #[library_benchmark]
        fn $name() -> u64 {
            let data = black_box($data.as_bytes());
            let mut rdr =
                ReaderBuilder::new().has_headers($headers).from_reader(data);
            let mut count = 0;
            let mut rec = ByteRecord::new();
            while rdr.read_byte_record(&mut rec).unwrap() {
                let _: $type = black_box(&rec).deserialize(None).unwrap();
                count += 1;
            }
            assert_eq!(count, $result);
            black_box(count)
        }
    };
}

macro_rules! bench_deserialize_borrowed_str {
    ($name:ident, $data:ident, $type:ty, $headers:expr, $result:expr) => {
        #[library_benchmark]
        fn $name() -> u64 {
            let data = black_box($data.as_bytes());
            let mut rdr =
                ReaderBuilder::new().has_headers($headers).from_reader(data);
            let mut count = 0;
            let mut rec = StringRecord::new();
            while rdr.read_record(&mut rec).unwrap() {
                let _: $type = black_box(&rec).deserialize(None).unwrap();
                count += 1;
            }
            assert_eq!(count, $result);
            black_box(count)
        }
    };
}

macro_rules! bench_serialize {
    ($name:ident, $setup:ident, $type:ty, $headers:expr) => {
        #[library_benchmark]
        #[bench::run(setup = $setup)]
        fn $name(values: &'static [$type]) -> usize {
            let mut counter = ByteCounter::default();
            {
                let mut wtr = WriterBuilder::new()
                    .has_headers($headers)
                    .from_writer(&mut counter);
                for value in black_box(values) {
                    wtr.serialize(value).unwrap();
                }
            }
            black_box(counter.count)
        }
    };
}

bench_deserialize_owned!(
    count_nfl_deserialize_owned_bytes,
    NFL,
    count_deserialize_owned_bytes,
    NFLRowOwned,
    9999,
    true
);
bench_serialize!(
    count_nfl_serialize_owned_bytes,
    setup_nfl_values,
    NFLRowOwned,
    true
);
bench_deserialize_owned!(
    count_nfl_deserialize_owned_str,
    NFL,
    count_deserialize_owned_str,
    NFLRowOwned,
    9999,
    true
);
bench_serialize!(
    count_nfl_serialize_owned_str,
    setup_nfl_values,
    NFLRowOwned,
    true
);
bench_deserialize_borrowed_bytes!(
    count_nfl_deserialize_borrowed_bytes,
    NFL,
    NFLRowBorrowed<'_>,
    true,
    9999
);
bench_deserialize_borrowed_str!(
    count_nfl_deserialize_borrowed_str,
    NFL,
    NFLRowBorrowed<'_>,
    true,
    9999
);
bench_read!(count_nfl_iter_bytes, NFL, count_iter_bytes, 130000, false);
bench_read!(count_nfl_iter_bytes_trimmed, NFL, count_iter_bytes, 130000, true);
bench_read!(count_nfl_iter_str, NFL, count_iter_str, 130000, false);
bench_read!(count_nfl_iter_str_trimmed, NFL, count_iter_str, 130000, true);
bench_read!(count_nfl_read_bytes, NFL, count_read_bytes, 130000, false);
bench_read!(count_nfl_read_str, NFL, count_read_str, 130000, false);

bench_deserialize_owned!(
    count_game_deserialize_owned_bytes,
    GAME,
    count_deserialize_owned_bytes,
    GAMERowOwned,
    100000,
    false
);
bench_serialize!(
    count_game_serialize_owned_bytes,
    setup_game_values,
    GAMERowOwned,
    false
);
bench_deserialize_owned!(
    count_game_deserialize_owned_str,
    GAME,
    count_deserialize_owned_str,
    GAMERowOwned,
    100000,
    false
);
bench_serialize!(
    count_game_serialize_owned_str,
    setup_game_values,
    GAMERowOwned,
    false
);
bench_deserialize_borrowed_bytes!(
    count_game_deserialize_borrowed_bytes,
    GAME,
    GAMERowBorrowed<'_>,
    true,
    99999
);
bench_deserialize_borrowed_str!(
    count_game_deserialize_borrowed_str,
    GAME,
    GAMERowBorrowed<'_>,
    true,
    99999
);
bench_read!(count_game_iter_bytes, GAME, count_iter_bytes, 600000, false);
bench_read!(count_game_iter_str, GAME, count_iter_str, 600000, false);
bench_read!(count_game_read_bytes, GAME, count_read_bytes, 600000, false);
bench_read!(count_game_read_str, GAME, count_read_str, 600000, false);

bench_deserialize_owned!(
    count_pop_deserialize_owned_bytes,
    POP,
    count_deserialize_owned_bytes,
    POPRowOwned,
    20000,
    true
);
bench_serialize!(
    count_pop_serialize_owned_bytes,
    setup_pop_values,
    POPRowOwned,
    true
);
bench_deserialize_owned!(
    count_pop_deserialize_owned_str,
    POP,
    count_deserialize_owned_str,
    POPRowOwned,
    20000,
    true
);
bench_serialize!(
    count_pop_serialize_owned_str,
    setup_pop_values,
    POPRowOwned,
    true
);
bench_deserialize_borrowed_bytes!(
    count_pop_deserialize_borrowed_bytes,
    POP,
    POPRowBorrowed<'_>,
    true,
    20000
);
bench_deserialize_borrowed_str!(
    count_pop_deserialize_borrowed_str,
    POP,
    POPRowBorrowed<'_>,
    true,
    20000
);
bench_read!(count_pop_iter_bytes, POP, count_iter_bytes, 140007, false);
bench_read!(count_pop_iter_str, POP, count_iter_str, 140007, false);
bench_read!(count_pop_read_bytes, POP, count_read_bytes, 140007, false);
bench_read!(count_pop_read_str, POP, count_read_str, 140007, false);

bench_deserialize_owned!(
    count_mbta_deserialize_owned_bytes,
    MBTA,
    count_deserialize_owned_bytes,
    MBTARowOwned,
    9999,
    true
);
bench_serialize!(
    count_mbta_serialize_owned_bytes,
    setup_mbta_values,
    MBTARowOwned,
    true
);
bench_deserialize_owned!(
    count_mbta_deserialize_owned_str,
    MBTA,
    count_deserialize_owned_str,
    MBTARowOwned,
    9999,
    true
);
bench_serialize!(
    count_mbta_serialize_owned_str,
    setup_mbta_values,
    MBTARowOwned,
    true
);
bench_deserialize_borrowed_bytes!(
    count_mbta_deserialize_borrowed_bytes,
    MBTA,
    MBTARowBorrowed<'_>,
    true,
    9999
);
bench_deserialize_borrowed_str!(
    count_mbta_deserialize_borrowed_str,
    MBTA,
    MBTARowBorrowed<'_>,
    true,
    9999
);
bench_read!(count_mbta_iter_bytes, MBTA, count_iter_bytes, 90000, false);
bench_read!(count_mbta_iter_str, MBTA, count_iter_str, 90000, false);
bench_read!(count_mbta_read_bytes, MBTA, count_read_bytes, 90000, false);
bench_read!(count_mbta_read_str, MBTA, count_read_str, 90000, false);

fn setup_nfl_values() -> &'static [NFLRowOwned] {
    static VALUES: OnceLock<Vec<NFLRowOwned>> = OnceLock::new();
    VALUES.get_or_init(|| collect_values(NFL, true))
}

fn setup_game_values() -> &'static [GAMERowOwned] {
    static VALUES: OnceLock<Vec<GAMERowOwned>> = OnceLock::new();
    VALUES.get_or_init(|| collect_values(GAME, false))
}

fn setup_pop_values() -> &'static [POPRowOwned] {
    static VALUES: OnceLock<Vec<POPRowOwned>> = OnceLock::new();
    VALUES.get_or_init(|| collect_values(POP, true))
}

fn setup_mbta_values() -> &'static [MBTARowOwned] {
    static VALUES: OnceLock<Vec<MBTARowOwned>> = OnceLock::new();
    VALUES.get_or_init(|| collect_values(MBTA, true))
}

fn collect_values<D: DeserializeOwned>(
    data: &'static str,
    headers: bool,
) -> Vec<D> {
    ReaderBuilder::new()
        .has_headers(headers)
        .from_reader(data.as_bytes())
        .deserialize()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn count_deserialize_owned_bytes<R, D>(rdr: &mut Reader<R>) -> u64
where
    R: io::Read,
    D: DeserializeOwned,
{
    let mut count = 0;
    let mut rec = ByteRecord::new();
    while rdr.read_byte_record(&mut rec).unwrap() {
        let _: D = black_box(&rec).deserialize(None).unwrap();
        count += 1;
    }
    count
}

fn count_deserialize_owned_str<R, D>(rdr: &mut Reader<R>) -> u64
where
    R: io::Read,
    D: DeserializeOwned,
{
    let mut count = 0;
    for rec in rdr.deserialize::<D>() {
        black_box(rec.unwrap());
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
    let mut rec = ByteRecord::new();
    while rdr.read_byte_record(&mut rec).unwrap() {
        count += rec.len() as u64;
    }
    count
}

fn count_read_str<R: io::Read>(rdr: &mut Reader<R>) -> u64 {
    let mut count = 0;
    let mut rec = StringRecord::new();
    while rdr.read_record(&mut rec).unwrap() {
        count += rec.len() as u64;
    }
    count
}

fn setup_any_record() -> &'static ByteRecord {
    static RECORD: OnceLock<ByteRecord> = OnceLock::new();
    RECORD.get_or_init(|| {
        ByteRecord::from(vec![
            &b"true"[..],
            &b"123456789"[..],
            &b"-42"[..],
            &b"340282366920938463463374607431768211455"[..],
            &b"-170141183460469231731687303715884105728"[..],
            &b"3.14159265"[..],
            &b"plain text"[..],
            &b"\xFF"[..],
        ])
    })
}

#[library_benchmark]
#[bench::run(setup = setup_any_record)]
fn deserialize_any_fields(record: &'static ByteRecord) -> usize {
    let mut sum = 0;
    for _ in 0..10_000 {
        let values: [AnyValue; 8] =
            black_box(record).deserialize(None).unwrap();
        sum += values.iter().map(|value| value.0 as usize).sum::<usize>();
    }
    black_box(sum)
}

fn setup_nfl_records() -> &'static [ByteRecord] {
    static RECORDS: OnceLock<Vec<ByteRecord>> = OnceLock::new();
    RECORDS.get_or_init(|| {
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(NFL.as_bytes());
        rdr.byte_records().collect::<Result<_, _>>().unwrap()
    })
}

#[library_benchmark]
#[bench::run(setup = setup_nfl_records)]
fn write_nfl_record(records: &'static [ByteRecord]) -> usize {
    let mut wtr = Writer::from_writer(Vec::new());
    for record in black_box(records) {
        wtr.write_record(record).unwrap();
    }
    wtr.flush().unwrap();
    black_box(wtr.get_ref().len())
}

#[library_benchmark]
#[bench::run(setup = setup_nfl_records)]
fn write_nfl_bytes(records: &'static [ByteRecord]) -> usize {
    let mut wtr = Writer::from_writer(Vec::new());
    for record in black_box(records) {
        wtr.write_byte_record(record).unwrap();
    }
    wtr.flush().unwrap();
    black_box(wtr.get_ref().len())
}

library_benchmark_group!(
    name = reading,
    benchmarks = [
        count_nfl_iter_bytes,
        count_nfl_iter_bytes_trimmed,
        count_nfl_iter_str,
        count_nfl_iter_str_trimmed,
        count_nfl_read_bytes,
        count_nfl_read_str,
        count_game_iter_bytes,
        count_game_iter_str,
        count_game_read_bytes,
        count_game_read_str,
        count_pop_iter_bytes,
        count_pop_iter_str,
        count_pop_read_bytes,
        count_pop_read_str,
        count_mbta_iter_bytes,
        count_mbta_iter_str,
        count_mbta_read_bytes,
        count_mbta_read_str
    ]
);

library_benchmark_group!(
    name = deserializing,
    benchmarks = [
        count_nfl_deserialize_owned_bytes,
        count_nfl_deserialize_owned_str,
        count_nfl_deserialize_borrowed_bytes,
        count_nfl_deserialize_borrowed_str,
        count_game_deserialize_owned_bytes,
        count_game_deserialize_owned_str,
        count_game_deserialize_borrowed_bytes,
        count_game_deserialize_borrowed_str,
        count_pop_deserialize_owned_bytes,
        count_pop_deserialize_owned_str,
        count_pop_deserialize_borrowed_bytes,
        count_pop_deserialize_borrowed_str,
        count_mbta_deserialize_owned_bytes,
        count_mbta_deserialize_owned_str,
        count_mbta_deserialize_borrowed_bytes,
        count_mbta_deserialize_borrowed_str,
        deserialize_any_fields
    ]
);

library_benchmark_group!(
    name = serializing,
    benchmarks = [
        count_nfl_serialize_owned_bytes,
        count_nfl_serialize_owned_str,
        count_game_serialize_owned_bytes,
        count_game_serialize_owned_str,
        count_pop_serialize_owned_bytes,
        count_pop_serialize_owned_str,
        count_mbta_serialize_owned_bytes,
        count_mbta_serialize_owned_str
    ]
);

library_benchmark_group!(
    name = writing,
    benchmarks = [write_nfl_record, write_nfl_bytes]
);

main!(
    library_benchmark_groups = [reading, deserializing, serializing, writing]
);
