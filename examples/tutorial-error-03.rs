extern crate csv;

use std::error::Error;
use std::io;
use std::process;

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.records() {
        // Examine our Result.
        // If there was no problem, print the record.
        // Otherwise, convert our error to a Box<Error> and return it.
        match result {
            Err(err) => return Err(From::from(err)),
            Ok(record) => {
              println!("{:?}", record);
            }
        }
    }
    Ok(())
}
