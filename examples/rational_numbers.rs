//! This example shows how to write your own custom implementation of
//! `Decodable` to parse rational numbers.

extern crate csv;
extern crate regex;
extern crate "rustc-serialize" as rustc_serialize;

use std::str;

use regex::Regex;
use rustc_serialize::{Decodable, Decoder};

#[deriving(Show)]
struct Rational {
    numerator: i64,
    denominator: i64,
}

impl<E, D: Decoder<E>> Decodable<D, E> for Rational {
    fn decode(d: &mut D) -> Result<Rational, E> {
        let field = try!(d.read_str());
        // This uses the `FromStr` impl below.
        match field.parse() {
            Some(rat) => Ok(rat),
            None => Err(d.error(&*format!(
                "Could not parse '{}' as a rational.", field))),
        }
    }
}

impl str::FromStr for Rational {
    /// Parse a string into a Rational. Allow for the possibility of whitespace
    /// around `/`.
    fn from_str(s: &str) -> Option<Rational> {
        let re = Regex::new(r"^([0-9]+)\s*/\s*([0-9]+)$").unwrap();
        re.captures(s).map(|caps| Rational {
            numerator: caps.at(1).unwrap().parse().unwrap(),
            denominator: caps.at(2).unwrap().parse().unwrap(),
        })
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
        println!("({}, {}): {}", x, y, r);
    }
}
