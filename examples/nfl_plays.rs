extern crate csv;
extern crate serialize;

use std::path::Path;

#[allow(dead_code)]
#[deriving(Decodable)]
struct Play {
    gameid: String,
    qtr: uint,
    min: Option<uint>,
    sec: Option<uint>,
    team_off: String,
    team_def: String,
    down: Option<uint>,
    togo: Option<uint>,
    ydline: Option<uint>,
    description: String,
    offscore: uint,
    defscore: uint,
    season: uint,
}

fn main() {
    let fp = &Path::new("./data/2012_nfl_pbp_data.csv");

    let mut dec = csv::Reader::from_file(fp);
    match dec.decode::<Play>().collect::<Result<Vec<_>, _>>() {
        Err(err) => panic!("{}", err),
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
