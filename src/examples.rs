/*!
A cookbook of examples for CSV reading and writing.

# List of examples

This is a list of examples that follow. Each of them can be found in the
`examples` directory of the
[`rust-csv`](https://github.com/BurntSushi/rust-csv)
repository.

1. [Simple example.](#simple-example)
2. [Simple example with Serde.](#simple-example-with-serde)
3. [Example with a different delimiter.](#example-with-a-different-delimiter)
4. [Example without headers.](#example-without-headers)
5. [Simple example writing CSV data.](#simple-example-writing-csv-data)
6. [Simple example writing CSV data with Serde.](#simple-example-writing-csv-data-with-serde)

# Simple example

This example shows how to read CSV data from a file and print each record to
stdout.

```no_run
extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

fn example() -> Result<(), Box<Error>> {
    // Build the CSV reader and iterate over each record.
    let file_path = get_first_arg()?;
    let mut rdr = csv::Reader::from_path(&file_path)?;
    for result in rdr.records() {
        // The iterator yields Result<StringRecord, Error>, so we check the
        // error here..
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
```

The above example can be run like so:

```ignore
$ git clone git://github.com/BurntSushi/rust-csv
$ cd rust-csv
$ cargo run --example simple examples/data/simplepop.csv
```

# Simple example with Serde

This is like the previous example, except it shows how to deserialize each
record into a struct type that you define.

For more examples and details on how Serde deserialization works, see the
[`Reader::deserialize`](struct.Reader.html#method.deserialize)
method.

```no_run
extern crate csv;
#[macro_use]
extern crate serde_derive;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

// By default, struct field names are deserialized based on the position of
// a corresponding field in the CSV data's header record.
#[derive(Debug,Deserialize)]
struct Record {
    city: String,
    region: String,
    country: String,
    population: Option<u64>,
}

fn example() -> Result<(), Box<Error>> {
    // Build the CSV reader and iterate over each record.
    let file_path = get_first_arg()?;
    let mut rdr = csv::Reader::from_path(&file_path)?;
    for result in rdr.deserialize() {
        // Notice that we need to provide a type hint for automatic
        // deserialization.
        let record: Record = result?;
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
```

The above example can be run like so:

```ignore
$ git clone git://github.com/BurntSushi/rust-csv
$ cd rust-csv
$ cargo run --example simple-serde examples/data/simplepop.csv
```

# Example with a different delimiter

This example shows how to read CSV data from a file where fields are separated
by `;` instead of `,`.

```no_run
extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

fn example() -> Result<(), Box<Error>> {
    let file_path = match env::args_os().nth(1) {
        None => return Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => file_path,
    };

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
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
```

The above example can be run like so:

```ignore
$ git clone git://github.com/BurntSushi/rust-csv
$ cd rust-csv
$ cargo run --example simple-delim examples/data/simplepop-delim.csv
```

# Example without headers

The CSV reader in this crate assumes that CSV data has a header record by
default, but the setting can be toggled. When enabled, the first record in
CSV data in interpreted as the header record and is skipped. When disabled, the
first record is not skipped. This example shows how to disable that setting.

```no_run
extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

fn example() -> Result<(), Box<Error>> {
    let file_path = get_first_arg()?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
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
```

The above example can be run like so:

```ignore
$ git clone git://github.com/BurntSushi/rust-csv
$ cd rust-csv
$ cargo run --example simple-no-headers examples/data/simplepop-no-headers.csv
```

# Simple example writing CSV data

This example shows how to write CSV data to a file.

```no_run
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
    wtr.write_record(&["city", "region", "country", "population"][..])?;
    wtr.write_record(&["Southborough", "MA", "United States", "9686"][..])?;
    wtr.write_record(&["Northbridge", "MA", "United States", "14061"][..])?;
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
```

The above example can be run like so:

```ignore
$ git clone git://github.com/BurntSushi/rust-csv
$ cd rust-csv
$ cargo run --example simple-write /tmp/simplepop.csv
```

# Simple example writing CSV data with Serde

This example shows how to write CSV data to a file with Serde. Namely, we
represent each record using a custom struct that we define. In this example,
headers are written automatically.

```no_run
extern crate csv;
#[macro_use]
extern crate serde_derive;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

#[derive(Debug, Serialize)]
struct Record {
    city: String,
    region: String,
    country: String,
    population: Option<u64>,
}

fn example() -> Result<(), Box<Error>> {
    // Build the CSV writer and write a few records.
    let file_path = get_first_arg()?;
    let mut wtr = csv::Writer::from_path(&file_path)?;

    // When writing records without Serde, the header row must be written
    // explicitly.
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
```

The above example can be run like so:

```ignore
$ git clone git://github.com/BurntSushi/rust-csv
$ cd rust-csv
$ cargo run --example simple-write-serde /tmp/simplepop.csv
```
*/
