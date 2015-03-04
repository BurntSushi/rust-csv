extern crate csv;

fn main() {
    let mut rdr = csv::Reader::from_file("./data/simple.csv").unwrap();
    for record in rdr.decode() {
        let (s1, s2, dist): (String, String, usize) = record.unwrap();
        println!("({}, {}): {}", s1, s2, dist);
    }
}
