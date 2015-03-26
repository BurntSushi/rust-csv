extern crate csv;
extern crate rustc_serialize;

#[derive(RustcDecodable)]
struct Record {
    s1: String,
    s2: String,
    dist: u32,
}

fn main() {
    let mut rdr = csv::Reader::from_file("./data/simple.csv").unwrap();
    for record in rdr.decode() {
        let record: Record = record.unwrap();
        println!("({}, {}): {}", record.s1, record.s2, record.dist);
    }
}
