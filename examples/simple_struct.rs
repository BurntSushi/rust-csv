extern crate csv;
extern crate serialize;

use std::path::Path;

#[deriving(Decodable)]
struct Record {
    s1: String,
    s2: String,
    dist: uint,
}

fn main() {
    let fp = &Path::new("./data/simple.csv");
    let mut rdr = csv::Reader::from_file(fp);

    for record in rdr.decode() {
        let record: Record = record.unwrap();
        println!("({}, {}): {}", record.s1, record.s2, record.dist);
    }
}
