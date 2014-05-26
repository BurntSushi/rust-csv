extern crate csv;

use std::path::Path;

fn main() {
    let fp = &Path::new("./data/simple.csv");
    let mut rdr = csv::Decoder::from_file(fp);

    for (s1, s2, dist) in rdr.decode_iter::<(String, String, uint)>() {
        println!("({}, {}): {}", s1, s2, dist);
    }
}
