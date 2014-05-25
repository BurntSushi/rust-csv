extern crate csv;
extern crate serialize;

use std::path::Path;
use csv::Decoder;

#[deriving(Decodable)]
struct Play {
    gameid: StrBuf,
    qtr: uint,
    min: Option<uint>,
    sec: Option<uint>,
    team_off: StrBuf,
    team_def: StrBuf,
    down: Option<uint>,
    togo: Option<uint>,
    ydline: Option<uint>,
    description: StrBuf,
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
                "NE" == p.team_off.as_slice() && "DEN" == p.team_def.as_slice()
                && p.description.as_slice().contains("TOUCHDOWN")
                && p.description.as_slice().contains("T.Brady")
            }).unwrap();
            println!("Tom Brady touchdown: {}", tfb.description);
        }
    }
}
