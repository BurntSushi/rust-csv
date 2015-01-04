#![feature(old_orphan_check)] // see rustc commit c61a00

extern crate csv;
extern crate "rustc-serialize" as rustc_serialize;

use std::path::Path;

#[derive(RustcDecodable)]
struct Record {
    s1: String,
    s2: String,
    dist: Option<uint>,
}

fn main() {
    let fp = &Path::new("./data/simple_missing.csv");
    let rdr = csv::Reader::from_file(fp);
    for record in rdr.decode() {
        let record: Record = record.unwrap();
        println!("({}, {}): {}", record.s1, record.s2, record.dist);
    }
}
