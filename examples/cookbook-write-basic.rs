extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

fn example() -> Result<(), Box<Error>> {
    // Build the CSV writer and write a few records.
    let file_path = get_first_arg()?;
    let mut wtr = csv::Writer::from_path(&file_path)?;

    // When writing records without Serde, the header record is written just
    // like any other record.
    wtr.write_record(&["city", "region", "country", "population"])?;
    wtr.write_record(&["Southborough", "MA", "United States", "9686"])?;
    wtr.write_record(&["Northbridge", "MA", "United States", "14061"])?;
    wtr.flush()?;
    Ok(())
}

fn get_first_arg() -> Result<OsString, Box<Error>> {
    match env::args_os().nth(1) {
        Some(file_path) => Ok(file_path),
        None => Err(From::from("expected 1 argument, but got none")),
    }
}

fn main() {
    if let Err(err) = example() {
        println!("error running example: {}", err);
        process::exit(1);
    }
}
