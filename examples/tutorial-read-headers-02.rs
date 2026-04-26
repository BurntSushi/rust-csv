use std::{error::Error, io, process};

fn run() -> Result<(), Box<dyn Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    let headers = rdr.headers()?;
    println!("{:?}", headers);
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    // We can ask for the headers at any time.
    let headers = rdr.headers()?;
    println!("{:?}", headers);
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
