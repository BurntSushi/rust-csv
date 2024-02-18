use eyre::Result;
use std::collections::HashMap;
use std::{io, process};

// This introduces a type alias so that we can conveniently reference our
// record type.
type Record = HashMap<String, String>;

fn run() -> Result<()> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.deserialize() {
        let record: Record = result?;
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
