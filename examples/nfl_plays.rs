#![allow(unstable)]

extern crate csv;
extern crate "rustc-serialize" as rustc_serialize;

use std::path::Path;

#[allow(dead_code)]
#[derive(RustcDecodable)]
struct Play {
    gameid: String,
    qtr: u32,
    min: Option<u32>,
    sec: Option<u32>,
    team_off: String,
    team_def: String,
    down: Option<u32>,
    togo: Option<u32>,
    ydline: Option<u32>,
    description: String,
    offscore: u32,
    defscore: u32,
    season: u32,
}

fn main() {
    let fp = &Path::new("./data/2012_nfl_pbp_data.csv");

    let mut dec = csv::Reader::from_file(fp);
    match dec.decode::<Play>().collect::<Result<Vec<_>, _>>() {
        Err(err) => panic!("{}", err),
        Ok(plays) => {
            println!("Found {} plays.", plays.len());

            let tfb = plays.iter().find(|&p| {
                "NE" == p.team_off && "DEN" == p.team_def
                && p.description.contains("TOUCHDOWN")
                && p.description.contains("T.Brady")
            }).unwrap();
            println!("Tom Brady touchdown: {}", tfb.description);
        }
    }
}
