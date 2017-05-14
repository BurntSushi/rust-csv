// To run this example:
//
//   $ git clone git://github.com/BurntSushi/rust-csv
//   $ cd rust-csv
//   $ cargo run --example cookbook-read-colon examples/data/smallpop-colon.csv
extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

fn example() -> Result<(), Box<Error>> {
    let file_path = get_first_arg()?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b':')
        .from_path(&file_path)?;
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
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
