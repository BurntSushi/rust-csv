extern crate csv;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

// This introduces a type alias so that we can conveniently reference our
// record type.
type Record = HashMap<String, String>;

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_path(get_first_arg()?)?;
    for result in rdr.deserialize() {
        let record: Record = result?;
        println!("{:?}", record);
    }
    Ok(())
}

fn get_first_arg() -> Result<OsString, Box<Error>> {
    env::args_os().nth(1).ok_or_else(|| From::from("expected at least 1 arg"))
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
