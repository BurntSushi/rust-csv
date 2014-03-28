extern crate csv;

use std::path::Path;

fn main() {
    let huge = "../examples/data/ss10pusa.csv";
    for _ in csv::Decoder::from_file(&Path::new(huge)).iter() {}
}
