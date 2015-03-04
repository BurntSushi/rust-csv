extern crate csv;
extern crate "rustc-serialize" as rustc_serialize;

#[derive(RustcDecodable)]
struct Record {
    s1: String,
    s2: String,
    dist: Option<u32>,
}

fn main() {
    let fp = "./data/simple_missing.csv";
    let mut rdr = csv::Reader::from_file(fp).unwrap();

    for record in rdr.decode() {
        let record: Record = record.unwrap();
        println!("({}, {}): {:?}", record.s1, record.s2, record.dist);
    }
}
