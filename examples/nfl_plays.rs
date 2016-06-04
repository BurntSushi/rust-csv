#![cfg_attr(feature = "serde", feature(custom_derive, plugin))]
#![cfg_attr(feature = "serde", plugin(serde_macros))]

extern crate csv;
#[cfg(feature = "rustc-serialize")]
extern crate rustc_serialize;

#[allow(dead_code)]
#[cfg_attr(feature = "rustc-serialize", derive(RustcDecodable))]
#[cfg_attr(feature = "serde", derive(Deserialize))]
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
    let fp = "./data/2012_nfl_pbp_data.csv";
    let mut dec = csv::Reader::from_file(fp).unwrap();

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
