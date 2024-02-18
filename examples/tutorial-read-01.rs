use eyre::{eyre, Result};
use std::{env, ffi::OsString, fs::File, process};

fn run() -> Result<()> {
    let file_path = get_first_arg()?;
    let file = File::open(file_path)?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}

/// Returns the first positional argument sent to this process. If there are no
/// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString> {
    match env::args_os().nth(1) {
        None => Err(eyre!("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn main() {
    if let Err(err) = run() {
        println!("{:?}", err);
        process::exit(1);
    }
}
