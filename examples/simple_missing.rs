extern crate csv;
extern crate serialize;

use std::path::Path;

#[deriving(Decodable)]
struct Record {
    s1: StrBuf,
    s2: StrBuf,
    dist: Option<uint>,
}

fn main() {
    let fp = &Path::new("./data/simple_missing.csv");
    let mut rdr = csv::Decoder::from_file(fp);

    for record in rdr.decode_iter::<Record>() {
        println!("({}, {}): {}", record.s1, record.s2, record.dist);
    }
}
