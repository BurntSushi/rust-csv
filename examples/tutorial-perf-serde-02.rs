#![allow(dead_code)]
use std::{error::Error, io, process};
#[cfg(feature = "serde")]
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record<'a> {
    country: &'a str,
    city: &'a str,
    accent_city: &'a str,
    region: &'a str,
    population: Option<u64>,
    latitude: f64,
    longitude: f64,
}

#[cfg(feature = "serde")]
fn run() -> Result<u64, Box<dyn Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    let mut raw_record = csv::StringRecord::new();
    let headers = rdr.headers()?.clone();

    let mut count = 0;
    while rdr.read_record(&mut raw_record)? {
        let record: Record = raw_record.deserialize(Some(&headers))?;
        if record.country == "us" && record.region == "MA" {
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(feature = "serde")]
fn main() {
    match run() {
        Ok(count) => {
            println!("{}", count);
        }
        Err(err) => {
            println!("{}", err);
            process::exit(1);
        }
    }
}
#[cfg(not(feature = "serde"))]
fn main() {
    println!("this example requires the 'serde' feature");
}
