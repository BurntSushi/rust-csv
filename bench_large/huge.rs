extern crate csv;

fn main() {
    let huge = ::std::env::args().nth(1).unwrap();
    let mut rdr = csv::Reader::from_file(huge).unwrap();
    let mut count = 0;
    loop {
        match rdr.next_bytes() {
            csv::NextField::Error(err) => panic!("{:?}", err),
            csv::NextField::EndOfCsv => break,
            csv::NextField::EndOfRecord => {}
            csv::NextField::Data(_) => { count += 1; }
        }
    }
    println!("{}", count);
}
