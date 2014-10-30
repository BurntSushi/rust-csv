extern crate csv;

use std::comm::channel;
use std::io;
use std::task::spawn;
use std::time::Duration;

fn main() {
    let (send, recv) = channel();
    spawn(proc() {
        let w = io::ChanWriter::new(send);
        let mut enc = csv::Writer::from_writer(w);
        for x in range(1u, 6) {
            match enc.encode((x, x * x)) {
                Ok(_) => {},
                Err(err) => panic!("Failed encoding: {}", err),
            }
            io::timer::sleep(Duration::milliseconds(500));
        }
    });

    let r = io::ChanReader::new(recv);
    // We create a CSV reader with a small buffer so that we can see streaming
    // in action on small inputs.
    let buf = io::BufferedReader::with_capacity(1, r);
    let mut dec = csv::Reader::from_reader(buf);
    for r in dec.records() {
        println!("Record: {}", r);
    }
}
