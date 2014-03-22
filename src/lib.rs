#[crate_id = "csv#0.1.0"];
#[crate_type = "lib"];
#[license = "UNLICENSE"];
#[doc(html_root_url = "http://burntsushi.net/rustdoc/rust-csv")];
#[allow(deprecated_owned_vector)];

#[allow(dead_code)];
#[allow(unused_variable)];

//! This crate provides a CSV encoder and decoder that works with Rust's
//! `serialize` crate.

#[feature(macro_rules)];
// Dunno what this is, but apparently it's required for the 'log' crate.
#[feature(phase)];

#[phase(syntax, link)] extern crate log;
extern crate rand;
extern crate serialize;

use std::io;
use std::io::{BufferedReader, Reader, MemReader};
use serialize::Decoder;

macro_rules! opttry(
    ($e:expr) => (match $e { Some(e) => return Err(e), None => {}, })
)

pub struct Decoder<R> {
    priv sep: char,
    priv same_len: bool,
    priv first_len: uint,
    priv buf: BufferedReader<R>,
    priv record: ~[~str],
    priv cur: Option<char>,
    priv look: Option<char>,
    priv line: uint,
    priv col: uint,
}

#[deriving(Show)]
pub struct Error {
    line: uint,
    col: uint,
    msg: ~str,
}

fn ordie<T>(r: Result<T, Error>) -> T {
    match r {
        Ok(t) => t,
        Err(err) => fail!("{}", err),
    }
}

impl<R: Reader> Decoder<R> {
    pub fn from_reader(r: R) -> Decoder<R> {
        let buf = BufferedReader::new(r);
        Decoder {
            sep: ',',
            same_len: true,
            first_len: 0,
            buf: buf,
            record: ~[],
            cur: Some(0u8 as char),
            look: None,
            line: 1,
            col: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.is_eof() && self.record.len() == 0
    }

    fn is_eof(&self) -> bool {
        self.cur.is_none()
    }
}

impl Decoder<MemReader> {
    pub fn from_str(s: &str) -> Decoder<MemReader> {
        let r = MemReader::new(s.as_bytes().to_owned());
        Decoder::from_reader(r)
    }
}

impl<R: Reader> Decoder<R> {
    fn next_record(&mut self) -> Option<Error> {
        if self.record.len() == 0 {
            return match self.parse_record() {
                Ok(r) => { self.record = r; None },
                Err(err) => Some(err),
            }
        }
        None
    }

    fn next_value(&mut self) -> Result<~str, Error> {
        match self.next_record() {
            Some(err) => Err(err),
            None => {
                match self.record.shift() {
                    None => {
                        if self.is_eof() {
                            Err(self.err("EOF"))
                        } else {
                            Err(self.err("BUG: found empty record"))
                        }
                    },
                    Some(v) => Ok(v),
                }
            },
        }
    }

    fn next_char(&mut self) -> Option<Error> {
        match self.read_next() {
            Ok(c) => self.cur = c,
            Err(err) => return Some(err),
        }
        if !self.is_eof() {
            if self.cur_is('\n') {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        None
    }

    fn read_next(&mut self) -> Result<Option<char>, Error> {
        match self.look {
            Some(c) => { self.look = None; Ok(Some(c)) },
            None => match self.buf.read_char() {
                Ok(c) => Ok(Some(c)),
                Err(err) => {
                    if err.kind == io::EndOfFile {
                        Ok(None)
                    } else {
                        Err(self.err(format!("Could not read char: {}", err)))
                    }
                },
            },
        }
    }

    fn err(&self, msg: &str) -> Error {
        Error {
            line: self.line,
            col: self.col,
            msg: msg.to_owned(),
        }
    }

    fn peek(&mut self) -> Result<Option<char>, Error> {
        match self.look {
            Some(c) => Ok(Some(c)),
            None => {
                self.look = try!(self.read_next());
                Ok(self.look)
            },
        }
    }

    fn cur_is(&self, c: char) -> bool {
        self.cur == Some(c)
    }

    fn peek_is(&mut self, c: char) -> bool {
        match self.peek() {
            Ok(Some(p)) => p == c,
            _ => false,
        }
    }

    fn is_lineterm(&mut self) -> bool {
        if self.cur_is('\n') {
            return true
        }
        if self.cur_is('\r') {
            return self.peek_is('\n')
        }
        false
    }

    fn is_sep(&mut self) -> bool {
        self.cur_is(self.sep)
    }

    fn is_escaped_quote(&mut self) -> bool {
        // Assumes that self.cur == '"'
        self.peek_is('"')
    }

    fn parse_record(&mut self) -> Result<~[~str], Error> {
        let mut vals: ~[~str] = ~[];
        while !self.is_eof() {
            let val = try!(self.parse_value());
            vals.push(val);
            if self.is_lineterm() {
                // If it's a CRLF ending, consume the '\r'
                if self.cur_is('\r') { opttry!(self.next_char()) }
                break
            }
        }
        Ok(vals)
    }

    fn parse_value(&mut self) -> Result<~str, Error> {
        let mut only_whitespace = true;
        let mut res = ~"";
        loop {
            opttry!(self.next_char());
            if self.is_eof() || self.is_lineterm() || self.is_sep() {
                break
            } else if self.cur.unwrap().is_whitespace() {
                res.push_char(self.cur.unwrap());
                continue
            } else if only_whitespace && self.cur_is('"') {
                // Throw away any leading whitespace.
                return self.parse_quoted_value()
            }
            only_whitespace = false;
            res.push_char(self.cur.unwrap());
        }
        Ok(res)
    }

    fn parse_quoted_value(&mut self) -> Result<~str, Error> {
        // Assumes that " has already been read.
        let mut res = ~"";
        loop {
            opttry!(self.next_char());
            if self.is_eof() {
                return Err(self.err("EOF while parsing quoted value."))
            } else if self.cur_is('"') {
                if self.is_escaped_quote() {
                    opttry!(self.next_char()); // throw away second "
                    res.push_char('"');
                }

                // Eat and spit out everything up to next separator.
                // If we see something that isn't whitespace, it's an error.
                loop {
                    opttry!(self.next_char());
                    if self.is_eof() || self.is_lineterm() || self.is_sep() {
                        break
                    } else if !self.cur.unwrap().is_whitespace() {
                        let msg = format!(
                            "Expected EOF, line terminator, separator or \
                            whitespace following quoted value but found \
                            '{}' instead.", self.cur.unwrap());
                        return Err(self.err(msg));
                    }
                }
                break
            }
            res.push_char(self.cur.unwrap());
        }
        Ok(res)
    }
}

impl<R: Reader> serialize::Decoder for Decoder<R> {
    fn read_nil(&mut self) { unimplemented!() }
    fn read_uint(&mut self) -> uint { unimplemented!() }
    fn read_u64(&mut self) -> u64 { unimplemented!() }
    fn read_u32(&mut self) -> u32 { unimplemented!() }
    fn read_u16(&mut self) -> u16 { unimplemented!() }
    fn read_u8(&mut self) -> u8 { unimplemented!() }
    fn read_int(&mut self) -> int { unimplemented!() }
    fn read_i64(&mut self) -> i64 { unimplemented!() }
    fn read_i32(&mut self) -> i32 { unimplemented!() }
    fn read_i16(&mut self) -> i16 { unimplemented!() }
    fn read_i8(&mut self) -> i8 { unimplemented!() }
    fn read_bool(&mut self) -> bool { unimplemented!() }
    fn read_f64(&mut self) -> f64 { unimplemented!() }
    fn read_f32(&mut self) -> f32 { unimplemented!() }
    fn read_char(&mut self) -> char { unimplemented!() }
    fn read_str(&mut self) -> ~str {
        ordie(self.next_value())
    }
    fn read_enum<T>(&mut self, name: &str, f: |&mut Decoder<R>| -> T) -> T {
        unimplemented!()
    }
    fn read_enum_variant<T>(&mut self, names: &[&str],
                            f: |&mut Decoder<R>, uint| -> T) -> T {
        unimplemented!()
    }
    fn read_enum_variant_arg<T>(&mut self, a_idx: uint,
                                f: |&mut Decoder<R>| -> T) -> T {
        unimplemented!()
    }
    fn read_enum_struct_variant<T>(&mut self, names: &[&str],
                                   f: |&mut Decoder<R>, uint| -> T) -> T {
        unimplemented!()
    }
    fn read_enum_struct_variant_field<T>(&mut self, f_name: &str, f_idx: uint,
                                         f: |&mut Decoder<R>| -> T) -> T {
        unimplemented!()
    }
    fn read_struct<T>(&mut self, s_name: &str, len: uint,
                      f: |&mut Decoder<R>| -> T) -> T {
        unimplemented!()
    }
    fn read_struct_field<T>(&mut self, f_name: &str, f_idx: uint,
                            f: |&mut Decoder<R>| -> T) -> T {
        unimplemented!()
    }
    fn read_tuple<T>(&mut self, f: |&mut Decoder<R>, uint| -> T) -> T {
        unimplemented!()
    }
    fn read_tuple_arg<T>(&mut self, a_idx: uint,
                         f: |&mut Decoder<R>| -> T) -> T {
        unimplemented!()
    }
    fn read_tuple_struct<T>(&mut self, s_name: &str,
                            f: |&mut Decoder<R>, uint| -> T) -> T {
        unimplemented!()
    }
    fn read_tuple_struct_arg<T>(&mut self, a_idx: uint,
                                f: |&mut Decoder<R>| -> T) -> T {
        unimplemented!()
    }
    fn read_option<T>(&mut self, f: |&mut Decoder<R>, bool| -> T) -> T {
        match self.next_value() {
            Ok(v) => { self.record.unshift(v); f(self, true) },
            Err(_) => f(self, false),
        }
    }
    fn read_seq<T>(&mut self, f: |&mut Decoder<R>, uint| -> T) -> T {
        unimplemented!()
    }
    fn read_seq_elt<T>(&mut self, idx: uint, f: |&mut Decoder<R>| -> T) -> T {
        unimplemented!()
    }
    fn read_map<T>(&mut self, f: |&mut Decoder<R>, uint| -> T) -> T {
        unimplemented!()
    }
    fn read_map_elt_key<T>(&mut self, idx: uint,
                           f: |&mut Decoder<R>| -> T) -> T {
        unimplemented!()
    }
    fn read_map_elt_val<T>(&mut self, idx: uint,
                           f: |&mut Decoder<R>| -> T) -> T {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    use serialize::Decodable;
    use super::Decoder;

    #[test]
    fn wat() {
        let mut dec = Decoder::from_str("a,     \"bbbb\"    ,c\r\nx,y,z");
        while !dec.is_empty() {
            let s: Option<~str> = Decodable::decode(&mut dec);
            debug!("{}", s);
        }
        // while !dec.is_eof() { 
            // match dec.parse_record() { 
                // Ok(v) => debug!("RECORD: |{}|", v), 
                // Err(err) => { debug!("ERR: {}", err); return; }, 
            // } 
        // } 
    }
}
