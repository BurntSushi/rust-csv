//! This example shows how to write your own custom implementation of
//! `Decodable` to parse rational numbers.

extern crate csv;
extern crate regex;
extern crate rustc_serialize;

use std::str;

use regex::Regex;
use rustc_serialize::{Decodable, Decoder};

#[derive(Debug)]
struct Rational {
    numerator: i64,
    denominator: i64,
}

impl Decodable for Rational {
    fn decode<D: Decoder>(d: &mut D) -> Result<Rational, D::Error> {
        let field = try!(d.read_str());
        // This uses the `FromStr` impl below.
        match field.parse() {
            Ok(rat) => Ok(rat),
            Err(_) => Err(d.error(&*format!(
                "Could not parse '{}' as a rational.", field))),
        }
    }
}

impl str::FromStr for Rational {
    type Err = String;

    /// Parse a string into a Rational. Allow for the possibility of whitespace
    /// around `/`.
    fn from_str(s: &str) -> Result<Rational, String> {
        let re = Regex::new(r"^([0-9]+)\s*/\s*([0-9]+)$").unwrap();
        re.captures(s)
          .map(|caps| Rational {
              numerator: caps.at(1).unwrap().parse().unwrap(),
              denominator: caps.at(2).unwrap().parse().unwrap(),
          })
          .ok_or(format!("Could not parse '{}' as a rational.", s))
    }
}

fn main() {
    let data = "
X,Y,Rational
1.1,2.3,5/8
13.21,34.55,144/233
377.610,987.1597,2584/4181";

    let mut rdr = csv::Reader::from_string(data).has_headers(true);
    for row in rdr.decode() {
        let (x, y, r): (f64, f64, Rational) = row.unwrap();
        println!("({}, {}): {:?}", x, y, r);
    }
}
