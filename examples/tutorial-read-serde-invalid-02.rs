#![allow(dead_code)]
use std::{error::Error, io, process};
#[cfg(feature = "serde")]
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    latitude: f64,
    longitude: f64,
    #[serde(deserialize_with = "csv::invalid_option")]
    population: Option<u64>,
    city: String,
    state: String,
}
#[cfg(feature = "serde")]
fn run() -> Result<(), Box<dyn Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.deserialize() {
        let record: Record = result?;
        println!("{:?}", record);
    }
    Ok(())
}
#[cfg(feature = "serde")]
fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
#[cfg(not(feature = "serde"))]
fn main() {
    println!("this example requires the 'serde' feature");
}
