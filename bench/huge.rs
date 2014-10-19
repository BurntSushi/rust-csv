extern crate csv;

use std::path::Path;

fn main() {
    let huge = "../examples/data/ss10pusa.csv";
    let mut rdr = csv::Reader::from_file(&Path::new(huge));
    while !rdr.done() {
        loop {
            match rdr.next_field() {
                None => break,
                Some(f) => { f.unwrap(); }
            }
        }
    }
}
