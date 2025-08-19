use eyre::Result;
use std::{io, process};

fn run() -> Result<()> {
    let mut rdr =
        csv::ReaderBuilder::new().has_headers(false).from_reader(io::stdin());
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        println!("{:?}", err);
        process::exit(1);
    }
}
