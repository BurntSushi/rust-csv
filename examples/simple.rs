extern crate csv;

use std::path::Path;

fn main() {
    let fp = &Path::new("./data/simple.csv");
    let mut rdr = csv::Decoder::from_file(fp);

    for record in rdr.iter_decode::<(String, String, uint)>() {
        let (s1, s2, dist) = record.unwrap();
        println!("({}, {}): {}", s1, s2, dist);
    }
}
