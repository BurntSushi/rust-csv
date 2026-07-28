use std::hint::black_box;

use csv_core::{Reader, ReaderBuilder, WriteResult, Writer};
use gungraun::prelude::*;

static NFL: &str = include_str!("../../examples/data/bench/nfl.csv");
static GAME: &str = include_str!("../../examples/data/bench/game.csv");
static POP: &str =
    include_str!("../../examples/data/bench/worldcitiespop.csv");
static MBTA: &str =
    include_str!("../../examples/data/bench/gtfs-mbta-stop-times.csv");

macro_rules! bench {
    ($name:ident, $data:ident, $counter:ident, $result:expr, $setup:ident) => {
        #[library_benchmark]
        #[bench::run(setup = $setup)]
        fn $name(mut rdr: Reader) -> u64 {
            let data = black_box($data.as_bytes());
            rdr.reset();
            let result = $counter(&mut rdr, data);
            assert_eq!(result, $result);
            black_box(result)
        }
    };
}

fn setup_dfa() -> Reader {
    ReaderBuilder::new().build()
}

fn setup_nfa() -> Reader {
    ReaderBuilder::new().nfa(true).build()
}

bench!(count_nfl_field_copy_dfa, NFL, count_fields, 130000, setup_dfa);
bench!(count_nfl_field_copy_nfa, NFL, count_fields, 130000, setup_nfa);
bench!(count_nfl_record_copy_dfa, NFL, count_records, 10000, setup_dfa);
bench!(count_nfl_record_copy_nfa, NFL, count_records, 10000, setup_nfa);

bench!(count_game_field_copy_dfa, GAME, count_fields, 600000, setup_dfa);
bench!(count_game_field_copy_nfa, GAME, count_fields, 600000, setup_nfa);
bench!(count_game_record_copy_dfa, GAME, count_records, 100000, setup_dfa);
bench!(count_game_record_copy_nfa, GAME, count_records, 100000, setup_nfa);

bench!(count_pop_field_copy_dfa, POP, count_fields, 140007, setup_dfa);
bench!(count_pop_field_copy_nfa, POP, count_fields, 140007, setup_nfa);
bench!(count_pop_record_copy_dfa, POP, count_records, 20001, setup_dfa);
bench!(count_pop_record_copy_nfa, POP, count_records, 20001, setup_nfa);

bench!(count_mbta_field_copy_dfa, MBTA, count_fields, 90000, setup_dfa);
bench!(count_mbta_field_copy_nfa, MBTA, count_fields, 90000, setup_nfa);
bench!(count_mbta_record_copy_dfa, MBTA, count_records, 10000, setup_dfa);
bench!(count_mbta_record_copy_nfa, MBTA, count_records, 10000, setup_nfa);

fn count_fields(rdr: &mut Reader, mut data: &[u8]) -> u64 {
    use csv_core::ReadFieldResult::*;

    let mut count = 0;
    let mut field = [0u8; 1024];
    loop {
        let (res, nin, _) = rdr.read_field(data, &mut field);
        data = &data[nin..];
        match res {
            InputEmpty => {}
            OutputFull => panic!("field too large"),
            Field { .. } => count += 1,
            End => break,
        }
    }
    count
}

fn count_records(rdr: &mut Reader, mut data: &[u8]) -> u64 {
    use csv_core::ReadRecordResult::*;

    let mut count = 0;
    let mut record = [0; 8192];
    let mut ends = [0; 32];
    loop {
        let (res, nin, _, _) = rdr.read_record(data, &mut record, &mut ends);
        data = &data[nin..];
        match res {
            InputEmpty => {}
            OutputFull | OutputEndsFull => panic!("field too large"),
            Record => count += 1,
            End => break,
        }
    }
    count
}

#[library_benchmark]
fn write_plain_fields() -> usize {
    write_fields(b"plain field value", 10_000)
}

#[library_benchmark]
fn write_quoted_fields() -> usize {
    write_fields(b"field with commas, newlines\nand \"quotes\"", 10_000)
}

fn write_fields(field: &[u8], iterations: usize) -> usize {
    let mut writer = Writer::new();
    let mut output = [0; 128];
    let mut written = 0;
    for _ in 0..iterations {
        let (result, consumed, nout) =
            writer.field(black_box(field), &mut output);
        assert_eq!(result, WriteResult::InputEmpty);
        assert_eq!(consumed, field.len());
        written += nout;

        let (result, nout) = writer.terminator(&mut output);
        assert_eq!(result, WriteResult::InputEmpty);
        written += nout;
    }
    black_box(written)
}

#[library_benchmark]
fn escape_dense_quotes() -> usize {
    let input = [b'"'; 128];
    let mut output = [0; 256];
    let mut written = 0;
    for _ in 0..1_000 {
        let (result, consumed, nout) =
            csv_core::quote(black_box(&input), &mut output, b'"', b'\\', true);
        assert_eq!(result, WriteResult::InputEmpty);
        assert_eq!(consumed, input.len());
        written += nout;
    }
    black_box(written)
}

#[library_benchmark]
fn classify_numeric_fields() -> usize {
    let fields: [&[u8]; 6] = [
        b"123456789",
        b"-42000",
        b"3.14159265",
        b"NaN",
        b"not a number",
        b"\xFF",
    ];
    let mut non_numeric = 0;
    for _ in 0..10_000 {
        for field in black_box(fields) {
            non_numeric += csv_core::is_non_numeric(field) as usize;
        }
    }
    black_box(non_numeric)
}

library_benchmark_group!(
    name = fields,
    benchmarks = [
        count_nfl_field_copy_dfa,
        count_nfl_field_copy_nfa,
        count_game_field_copy_dfa,
        count_game_field_copy_nfa,
        count_pop_field_copy_dfa,
        count_pop_field_copy_nfa,
        count_mbta_field_copy_dfa,
        count_mbta_field_copy_nfa
    ]
);

library_benchmark_group!(
    name = records,
    benchmarks = [
        count_nfl_record_copy_dfa,
        count_nfl_record_copy_nfa,
        count_game_record_copy_dfa,
        count_game_record_copy_nfa,
        count_pop_record_copy_dfa,
        count_pop_record_copy_nfa,
        count_mbta_record_copy_dfa,
        count_mbta_record_copy_nfa
    ]
);

library_benchmark_group!(
    name = writing,
    benchmarks = [
        write_plain_fields,
        write_quoted_fields,
        escape_dense_quotes,
        classify_numeric_fields
    ]
);

main!(library_benchmark_groups = [fields, records, writing]);
