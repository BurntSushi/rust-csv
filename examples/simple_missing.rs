#![cfg_attr(feature = "serde", feature(custom_derive, plugin))]
#![cfg_attr(feature = "serde", plugin(serde_macros))]

extern crate csv;
#[cfg(feature = "rustc-serialize")]
extern crate rustc_serialize;

#[cfg_attr(feature = "rustc-serialize", derive(RustcDecodable))]
#[cfg_attr(feature = "serde", derive(Deserialize))]
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
