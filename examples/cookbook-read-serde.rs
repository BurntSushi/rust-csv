#![allow(dead_code)]
use std::{error::Error, io, process};

#[cfg(feature = "serde")]
// By default, struct field names are deserialized based on the position of
// a corresponding field in the CSV data's header record.
#[derive(Debug, serde::Deserialize)]
struct Record {
    city: String,
    region: String,
    country: String,
    population: Option<u64>,
}

#[cfg(feature = "serde")]
fn example() -> Result<(), Box<dyn Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.deserialize() {
        // Notice that we need to provide a type hint for automatic
        // deserialization.
        let record: Record = result?;
        println!("{:?}", record);
    }
    Ok(())
}

#[cfg(feature = "serde")]
fn main() {
    if let Err(err) = example() {
        println!("error running example: {}", err);
        process::exit(1);
    }
}
#[cfg(not(feature = "serde"))]
fn main() {
    println!("this example requires the 'serde' feature");
}
