#![feature(test)]

extern crate csv_core;
extern crate test;

use test::Bencher;

use csv_core::{Reader, ReaderBuilder};

static NFL: &'static str = include_str!("../../examples/data/nfl.csv");
static GAME: &'static str = include_str!("../../examples/data/game.csv");
static POP: &'static str = include_str!("../../examples/data/worldcitiespop.csv");

macro_rules! bench {
    ($name:ident, $data:ident, $counter:ident, $result:expr) => {
        bench!($name, $data, $counter, $result, false);
    };
    ($name:ident, $data:ident, $counter:ident, $result:expr, NFA) => {
        bench!($name, $data, $counter, $result, true);
    };
    ($name:ident, $data:ident, $counter:ident, $result:expr, $nfa:expr) => {
        #[bench]
        fn $name(b: &mut Bencher) {
            let data = $data.as_bytes();
            b.bytes = data.len() as u64;
            let mut rdr = ReaderBuilder::new().nfa($nfa).build();
            b.iter(|| {
                rdr.reset();
                assert_eq!($counter(&mut rdr, data), $result);
            })
        }
    };
}

bench!(count_nfl_field_nocopy_dfa, NFL, count_fields_nocopy, 130000);
bench!(count_nfl_field_copy_dfa, NFL, count_fields, 130000);
bench!(count_nfl_field_copy_nfa, NFL, count_fields, 130000, NFA);
bench!(count_nfl_record_nocopy_dfa, NFL, count_records_nocopy, 10000);
bench!(count_nfl_record_copy_dfa, NFL, count_records, 10000);
bench!(count_nfl_record_copy_nfa, NFL, count_records, 10000, NFA);

bench!(count_game_field_nocopy_dfa, GAME, count_fields_nocopy, 600000);
bench!(count_game_field_copy_dfa, GAME, count_fields, 600000);
bench!(count_game_field_copy_nfa, GAME, count_fields, 600000, NFA);
bench!(count_game_record_nocopy_dfa, GAME, count_records_nocopy, 100000);
bench!(count_game_record_copy_dfa, GAME, count_records, 100000);
bench!(count_game_record_copy_nfa, GAME, count_records, 100000, NFA);

bench!(count_pop_field_nocopy_dfa, POP, count_fields_nocopy, 140007);
bench!(count_pop_field_copy_dfa, POP, count_fields, 140007);
bench!(count_pop_field_copy_nfa, POP, count_fields, 140007, NFA);
bench!(count_pop_record_nocopy_dfa, POP, count_records_nocopy, 20001);
bench!(count_pop_record_copy_dfa, POP, count_records, 20001);
bench!(count_pop_record_copy_nfa, POP, count_records, 20001, NFA);

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
            Field{..} => { count += 1; }
            End => break,
        }
    }
    count
}

fn count_fields_nocopy(rdr: &mut Reader, mut data: &[u8]) -> u64 {
    use csv_core::ReadFieldNoCopyResult::*;

    let mut count = 0;
    loop {
        let (res, nin) = rdr.read_field_nocopy(data);
        data = &data[nin..];
        match res {
            InputEmpty => {}
            Field{..} => { count += 1; }
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

fn count_records_nocopy(rdr: &mut Reader, mut data: &[u8]) -> u64 {
    use csv_core::ReadRecordNoCopyResult::*;

    let mut count = 0;
    loop {
        let (res, nin) = rdr.read_record_nocopy(data);
        data = &data[nin..];
        match res {
            InputEmpty => {}
            Record => count += 1,
            End => break,
        }
    }
    count
}
