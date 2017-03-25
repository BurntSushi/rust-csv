extern crate csv;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::error::Error;
use std::io;
use std::process;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record<'a> {
    country: &'a [u8],
    city: &'a [u8],
    accent_city: &'a [u8],
    region: &'a [u8],
    population: Option<u64>,
    latitude: f64,
    longitude: f64,
}

fn run() -> Result<u64, Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    let mut raw_record = csv::ByteRecord::new();
    let headers = rdr.byte_headers()?.clone();

    let mut count = 0;
    while rdr.read_byte_record(&mut raw_record)? {
        let record: Record = raw_record.deserialize(Some(&headers))?;
        if record.country == b"us" && record.region == b"MA" {
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
println!("{}", err);
process::exit(1);
}
}
}
