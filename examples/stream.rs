extern crate csv;

use std::comm::channel;
use std::io::{ChanReader, ChanWriter, Reader, Writer};
use std::io::timer::sleep;
use std::task::spawn;

use csv::{Decoder, Encoder};

fn main() {
    let (send, recv) = channel();
    spawn(proc() {
        let mut w = ChanWriter::new(send);
        let mut enc = Encoder::to_writer(&mut w as &mut Writer);
        for x in range(1u, 6) {
            match enc.encode((x, x * x)) {
                Ok(_) => {},
                Err(err) => fail!("Failed encoding: {}", err),
            }
            sleep(500);
        }
    });

    let mut r = ChanReader::new(recv);
    // We create a CSV reader with a small buffer so that we can see streaming
    // in action on small inputs.
    let mut dec = Decoder::from_reader_capacity(&mut r as &mut Reader, 1);
    for r in dec.iter() {
        println!("Record: {}", r);
    }
}
