#![feature(core, io, std_misc)]

extern crate csv;

use std::sync::mpsc::channel;
use std::old_io as io;
use std::thread::Thread;
use std::time::Duration;

fn main() {
    let (send, recv) = channel();
    let _ = Thread::spawn(move || {
        let w = io::ChanWriter::new(send);
        let mut enc = csv::Writer::from_writer(w);
        for x in range(1, 6) {
            if let Err(err) = enc.encode((x, x * x)) {
                panic!("Failed encoding: {}", err);
            }
            io::timer::sleep(Duration::milliseconds(200));
        }
    });

    let mut dec = csv::Reader::from_reader(io::ChanReader::new(recv))
                              .has_headers(false);
    for r in dec.records() {
        println!("Record: {:?}", r);
    }
}
