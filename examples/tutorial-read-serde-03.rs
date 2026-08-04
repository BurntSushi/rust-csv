#[allow(dead_code)]
use std::collections::HashMap;
use std::{error::Error, io, process};

// This introduces a type alias so that we can conveniently reference our
// record type.
type Record = HashMap<String, String>;

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
