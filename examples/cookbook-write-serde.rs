use std::{error::Error, io, process};

#[cfg(feature = "serde")]
#[derive(Debug, serde::Serialize)]
struct Record {
    city: String,
    region: String,
    country: String,
    population: Option<u64>,
}

#[cfg(feature = "serde")]
fn example() -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_writer(io::stdout());

    // When writing records with Serde using structs, the header row is written
    // automatically.
    wtr.serialize(Record {
        city: "Southborough".to_string(),
        region: "MA".to_string(),
        country: "United States".to_string(),
        population: Some(9686),
    })?;
    wtr.serialize(Record {
        city: "Northbridge".to_string(),
        region: "MA".to_string(),
        country: "United States".to_string(),
        population: Some(14061),
    })?;
    wtr.flush()?;
    Ok(())
}

#[cfg(feature = "serde")]
fn main() {
    if let Err(err) = example() {
        println!("error running example: {}", err);
        process::exit(1);
    }
}
#[cfg(not(feature = "serde"))]
fn main() {
    println!("this example requires the 'serde' feature");
}
