use eyre::Result;
use std::{io, process};

fn main() {
    if let Err(err) = run() {
        println!("{:?}", err);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}
