#![feature(path)]

extern crate csv;
extern crate "rustc-serialize" as rustc_serialize;

use std::old_path::Path;

#[derive(RustcDecodable)]
struct Record {
    s1: String,
    s2: String,
    dist: u32,
}

fn main() {
    let fp = &Path::new("./data/simple.csv");
    let mut rdr = csv::Reader::from_file(fp);

    for record in rdr.decode() {
        let record: Record = record.unwrap();
        println!("({}, {}): {}", record.s1, record.s2, record.dist);
    }
}
