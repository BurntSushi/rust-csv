use eyre::Result;
use std::{io, process};

fn run() -> Result<u64> {
    let mut rdr = csv::Reader::from_reader(io::stdin());

    let mut count = 0;
    for result in rdr.byte_records() {
        let record = result?;
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
