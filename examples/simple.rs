extern crate csv;

use std::path::Path;

fn main() {
    let fp = &Path::new("./data/simple.csv");
    let rdr = csv::Reader::from_file(fp);
    for record in rdr.decode() {
        let (s1, s2, dist): (String, String, uint) = record.unwrap();
        println!("({}, {}): {}", s1, s2, dist);
    }
}
