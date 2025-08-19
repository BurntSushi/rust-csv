use eyre::Result;
use std::{io, process};

fn run() -> Result<u64> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    let mut record = csv::ByteRecord::new();

    let mut count = 0;
    while rdr.read_byte_record(&mut record)? {
        if &record[0] == b"us" && &record[3] == b"MA" {
            count += 1;
        }
    }
    Ok(count)
}

fn main() {
    match run() {
        Ok(count) => {
            println!("{}", count);
        }
        Err(err) => {
            println!("{:?}", err);
            process::exit(1);
        }
    }
}
