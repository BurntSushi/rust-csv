use std::{env, error::Error, io, process};

use serde::{Deserialize, Serialize};

// Unlike previous examples, we derive both Deserialize and Serialize. This
// means we'll be able to automatically deserialize and serialize this type.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    city: String,
    state: String,
    population: Option<u64>,
    latitude: f64,
    longitude: f64,
}

fn run() -> Result<(), Box<dyn Error>> {
    // Get the query from the positional arguments.
    // If one doesn't exist or isn't an integer, return an error.
    let minimum_pop: u64 = match env::args().nth(1) {
        None => return Err(From::from("expected 1 argument, but got none")),
        Some(arg) => arg.parse()?,
    };

    // Build CSV readers and writers to stdin and stdout, respectively.
    // Note that we don't need to write headers explicitly. Since we're
    // serializing a custom struct, that's done for us automatically.
    let mut rdr = csv::Reader::from_reader(io::stdin());
    let mut wtr = csv::Writer::from_writer(io::stdout());

    // Iterate over all the records in `rdr`, and write only records containing
    // a population that is greater than or equal to `minimum_pop`.
    for result in rdr.deserialize() {
        // Remember that when deserializing, we must use a type hint to
        // indicate which type we want to deserialize our record into.
        let record: Record = result?;

        // `is_some_and` is a combinator on `Option`. It takes a closure that
        // returns `bool` when the `Option` is `Some`. When the `Option` is
        // `None`, `false` is always returned. In this case, we test it against
        // our minimum population count that we got from the command line.
        if record.population.is_some_and(|pop| pop >= minimum_pop) {
            wtr.serialize(record)?;
        }
    }

    // CSV writers use an internal buffer, so we should always flush when done.
    wtr.flush()?;
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
