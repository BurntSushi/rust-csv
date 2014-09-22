extern crate csv;

use std::path::Path;

fn main() {
    let huge = "../examples/data/ss10pusa.csv";
    let mut rdr = csv::Reader::from_file(&Path::new(huge));
    while !rdr.done() {
        for field in rdr { let _ = field.unwrap(); }
    }
}
