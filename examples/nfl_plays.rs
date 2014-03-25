extern crate csv;
extern crate serialize;

use std::path::Path;
use csv::Decoder;

#[deriving(Decodable)]
struct Play {
    gameid: ~str,
    qtr: uint,
    min: Option<uint>,
    sec: Option<uint>,
    team_off: ~str,
    team_def: ~str,
    down: Option<uint>,
    togo: Option<uint>,
    ydline: Option<uint>,
    description: ~str,
    offscore: uint,
    defscore: uint,
    season: uint,
}

fn main() {
    let fp = &Path::new("./data/2012_nfl_pbp_data.csv");
    let mut dec = Decoder::from_file(fp);
    dec.has_headers(true);
    match dec.decode_all::<Play>() {
        Err(err) => fail!("{}", err),
        Ok(plays) => {
            println!("Found {} plays.", plays.len());

            let tfb = plays.iter().find(|&p| {
                "NE" == p.team_off
                && p.description.contains("TOUCHDOWN")
                && p.description.contains("T.Brady")
            }).unwrap();
            println!("Tom Brady touchdown: {}", tfb.description);
        }
    }
}
