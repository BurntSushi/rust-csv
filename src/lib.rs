#[crate_id = "csv#0.1.0"];
#[crate_type = "lib"];
#[license = "UNLICENSE"];
#[doc(html_root_url = "http://burntsushi.net/rustdoc/csv")];

//! This crate provides a CSV encoder and decoder that works with Rust's
//! `serialize` crate.

#[feature(macro_rules)];
// Dunno what this is, but apparently it's required for the 'log' crate.
#[feature(phase)];

#[phase(syntax, link)] extern crate log;
extern crate quickcheck;
extern crate rand;
extern crate serialize;

use std::fmt;
use std::from_str::FromStr;
use std::io::{BufferedReader};
use std::io::{Reader, Writer};
use std::io::{EndOfFile, IoResult, MemReader, MemWriter};
use std::iter::Iterator;
use std::str;
use serialize::{Encodable, Decodable};

macro_rules! enctry(
    ($e:expr) => (
        if self.err.is_err() {
            return
        } else {
            match $e {
                Ok(e) => e,
                Err(e) => { self.err = Err(e.to_str()); return },
            }
        }
    )
)

macro_rules! dectry(
    ($e:expr, $d:expr) => (
        if self.err.is_err() {
            return $d
        } else {
            match $e {
                Ok(e) => e,
                Err(e) => { self.err = Err(e); return $d },
            }
        }
    )
)

/// A convenience encoder for encoding CSV data to strings.
pub struct StrEncoder<'a> {
    /// The underlying Encoder. Options like the separator, line endings and
    /// enforcing consistent record lengths can be set with it.
    ///
    /// It is OK to call `encode` and `encode_all` methods on the underlying
    /// encoder, but the corresponding methods on `StrEncoder` will be more
    /// convenient since they call `fail!` on error. (Encoding to a string
    /// isn't going to cause an IO error, but an error could be caused by
    /// writing records of varying length if same length records are enforced.)
    encoder: Encoder<'a>,
    priv w: ~MemWriter,
}

impl<'a> StrEncoder<'a> {
    /// Creates a new CSV string encoder. At any time, `to_str` can be called
    /// to retrieve the cumulative CSV data.
    pub fn new() -> StrEncoder<'a> {
        let mut w = ~MemWriter::new();
        let enc = Encoder::to_writer(&mut *w as &mut Writer);
        StrEncoder {
            encoder: enc,
            w: w,
        }
    }

    /// Returns the encoded CSV data as a string.
    pub fn to_str<'r>(&'r self) -> &'r str {
        str::from_utf8(self.w.get_ref()).unwrap()
    }

    /// This is just like `Encoder::encode`, except it calls `fail!` if there
    /// was an error.
    pub fn encode<E: Encodable<Encoder<'a>>>(&mut self, e: E) {
        match self.encoder.encode(e) {
            Ok(_) => {},
            Err(err) => fail!("{}", err),
        }
    }

    /// This is just like `Encoder::encode_all`, except it calls `fail!` if 
    /// there was an error.
    pub fn encode_all<E: Encodable<Encoder<'a>>>(&mut self, es: &[E]) {
        match self.encoder.encode_all(es) {
            Ok(_) => {},
            Err(err) => fail!("{}", err),
        }
     }
}

/// An encoder can encode Rust values into CSV records or documents.
pub struct Encoder<'a> {
    priv buf: &'a mut Writer,
    priv err: Result<(), ~str>,
    priv sep: char,
    priv same_len: bool,
    priv first_len: uint,
    priv use_crlf: bool,
}

impl<'a> Encoder<'a> {
    /// Creates an encoder that encodes CSV data with the `Writer` given.
    pub fn to_writer(w: &mut Writer) -> Encoder<'a> {
        Encoder {
            buf: w,
            err: Ok(()),
            sep: ',',
            same_len: true,
            first_len: 0,
            use_crlf: false,
        }
    }

    /// Encodes a record as CSV data. Only values with types that correspond
    /// to records can be given here (i.e., structs, tuples or vectors).
    pub fn encode<E: Encodable<Encoder<'a>>>
                 (&mut self, e: E) -> Result<(), ~str> {
        e.encode(self);
        self.err.clone()
    }

    /// Calls `encode` on each element in the slice given.
    pub fn encode_all<E: Encodable<Encoder<'a>>>
                     (&mut self, es: &[E]) -> Result<(), ~str> {
        // for e in es.move_iter() { 
        for e in es.iter() {
            try!(self.encode(e))
        }
        Ok(())
    }

    /// Sets the separator character that delimits values in a record.
    pub fn separator(&mut self, c: char) {
        self.sep = c;
    }

    /// When `yes` is `true`, all records written must have the same length.
    /// If a record is written that has a different length than other records
    /// already written, the encoding will fail.
    pub fn enforce_same_length(&mut self, yes: bool) {
        self.same_len = yes;
    }

    /// When `yes` is `true`, CRLF (`\r\n`) line endings will be used.
    pub fn crlf(&mut self, yes: bool) {
        self.use_crlf = yes;
    }

    fn w(&mut self, s: &str) -> IoResult<()> {
        self.buf.write_str(s)
    }

    fn write_to_str<T: fmt::Show>(&mut self, t: T) {
        enctry!(self.w(t.to_str()))
    }

    fn quoted<'a>(&mut self, s: &'a str) -> str::MaybeOwned<'a> {
        if s.find(|c: char| c == self.sep || c == '\n' || c == '"').is_some() {
            str::Owned(self.quote(s))
        } else {
            str::Slice(s)
        }
    }

    fn quote(&mut self, s: &str) -> ~str {
        let mut s = s.replace("\"", "\"\"");
        s.unshift_char('"');
        s.push_char('"');
        s
    }
}

impl<'a> serialize::Encoder for Encoder<'a> {
    fn emit_nil(&mut self) { unimplemented!() }
    fn emit_uint(&mut self, v: uint) { self.write_to_str(v) }
    fn emit_u64(&mut self, v: u64) { self.write_to_str(v) }
    fn emit_u32(&mut self, v: u32) { self.write_to_str(v) }
    fn emit_u16(&mut self, v: u16) { self.write_to_str(v) }
    fn emit_u8(&mut self, v: u8) { self.write_to_str(v) }
    fn emit_int(&mut self, v: int) { self.write_to_str(v) }
    fn emit_i64(&mut self, v: i64) { self.write_to_str(v) }
    fn emit_i32(&mut self, v: i32) { self.write_to_str(v) }
    fn emit_i16(&mut self, v: i16) { self.write_to_str(v) }
    fn emit_i8(&mut self, v: i8) { self.write_to_str(v) }
    fn emit_bool(&mut self, v: bool) { self.write_to_str(v) }
    fn emit_f64(&mut self, v: f64) {
        enctry!(self.w(::std::f64::to_str_digits(v, 10)))
    }
    fn emit_f32(&mut self, v: f32) {
        enctry!(self.w(::std::f32::to_str_digits(v, 10)))
    }
    fn emit_char(&mut self, v: char) { self.write_to_str(v) }
    fn emit_str(&mut self, v: &str) {
        let s = self.quoted(v).to_str();
        enctry!(self.w(s))
    }
    fn emit_enum(&mut self, _: &str, f: |&mut Encoder<'a>|) {
        f(self)
    }
    fn emit_enum_variant(&mut self, v_name: &str, _: uint, _: uint,
                         _: |&mut Encoder<'a>|) {
        enctry!(self.w(v_name))
    }
    fn emit_enum_variant_arg(&mut self, _: uint, _: |&mut Encoder<'a>|) {
        self.err = Err(~"Cannot encode enum variants with arguments.");
    }
    fn emit_enum_struct_variant(&mut self, v_name: &str, v_id: uint, len: uint,
                                f: |&mut Encoder<'a>|) {
        self.emit_enum_variant(v_name, v_id, len, f)
    }
    fn emit_enum_struct_variant_field(&mut self, _: &str, _: uint,
                                      _: |&mut Encoder<'a>|) {
        self.err = Err(~"Cannot encode enum variants with arguments.");
    }
    fn emit_struct(&mut self, _: &str, len: uint, f: |&mut Encoder<'a>|) {
        self.emit_seq(len, f)
    }
    fn emit_struct_field(&mut self, _: &str, f_idx: uint,
                         f: |&mut Encoder<'a>|) {
        self.emit_seq_elt(f_idx, f)
    }
    fn emit_tuple(&mut self, _: uint, _: |&mut Encoder<'a>|) {
        unimplemented!()
    }
    fn emit_tuple_arg(&mut self, _: uint, _: |&mut Encoder<'a>|) {
        unimplemented!()
    }
    fn emit_tuple_struct(&mut self, _: &str, _: uint,
                         _: |&mut Encoder<'a>|) {
        unimplemented!()
    }
    fn emit_tuple_struct_arg(&mut self, _: uint, _: |&mut Encoder<'a>|) {
        unimplemented!()
    }
    fn emit_option(&mut self, _: |&mut Encoder<'a>|) {
        unimplemented!()
    }
    fn emit_option_none(&mut self) { unimplemented!() }
    fn emit_option_some(&mut self, _: |&mut Encoder<'a>|) { unimplemented!() }
    fn emit_seq(&mut self, len: uint, f: |this: &mut Encoder<'a>|) {
        if len == 0 {
            self.err = Err(~"Records must have length bigger than 0.");
            return
        }
        if self.same_len {
            if self.first_len == 0 {
                self.first_len = len
            } else if self.first_len != len {
                self.err = Err(format!(
                    "Record has length {} but other records have length {}",
                    len, self.first_len));
                return
            }
        }
        f(self);
        if self.use_crlf {
            enctry!(self.w("\r\n"))
        } else {
            enctry!(self.w("\n"))
        }
    }
    fn emit_seq_elt(&mut self, idx: uint, f: |this: &mut Encoder<'a>|) {
        if idx > 0 {
            enctry!(self.buf.write_char(self.sep))
        }
        f(self)
    }
    fn emit_map(&mut self, _: uint, _: |&mut Encoder<'a>|) { unimplemented!() }
    fn emit_map_elt_key(&mut self, _: uint, _: |&mut Encoder<'a>|) {
        unimplemented!()
    }
    fn emit_map_elt_val(&mut self, _: uint, _: |&mut Encoder<'a>|) {
        unimplemented!()
    }
}

/// Information for a CSV decoding error.
#[deriving(Clone)]
pub struct Error {
    /// The line on which the error occurred.
    line: uint,

    /// The column where the error occurred.
    col: uint,

    /// A message describing the error.
    msg: ~str,

    /// Whether this error corresponds to EOF or not.
    eof: bool,
}

impl fmt::Show for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f.buf, "Parse error:{}:{}: {}", self.line, self.col, self.msg)
    }
}

/// The parser state used in the decoder.
struct Parser<'a> {
    sep: char, // separator character to use
    same_len: bool, // whether to enforce all rows be of same length
    first_len: uint, // length of first row
    has_headers: bool, // interpret first record as headers when true
    headers: Vec<~str>, // the first record in the CSV data
    buf: BufferedReader<&'a mut Reader>, // buffer to read CSV data from
    cur: Option<char>, // the current character
    look: Option<char>, // one character look-ahead
    line: uint, // current line
    col: uint, // current column
}

impl<'a> Parser<'a> {
    fn err(&self, msg: &str) -> Error {
        Error {
            line: self.line,
            col: self.col,
            msg: msg.to_owned(),
            eof: false,
        }
    }

    fn err_eof(&self) -> Error {
        Error {
            line: self.line,
            col: self.col,
            msg: ~"EOF",
            eof: true,
        }
    }

    fn next_char(&mut self) -> Result<(), Error> {
        self.cur = try!(self.read_next_char());
        if !self.is_eof() {
            if self.cur_is('\n') {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        Ok(())
    }

    fn read_next_char(&mut self) -> Result<Option<char>, Error> {
        match self.look {
            Some(c) => { self.look = None; Ok(Some(c)) },
            None => match self.buf.read_char() {
                Ok(c) => Ok(Some(c)),
                Err(err) => {
                    if err.kind == EndOfFile {
                        Ok(None)
                    } else {
                        Err(self.err(format!("Could not read char: {}", err)))
                    }
                },
            },
        }
    }

    fn parse_record(&mut self) -> Result<Vec<~str>, Error> {
        let mut vals: Vec<~str> = vec!();
        while !self.is_eof() {
            let val = try!(self.parse_value());
            vals.push(val);
            if self.is_lineterm() {
                // If it's a CRLF ending, consume the '\r'
                if self.cur_is('\r') { try!(self.next_char()) }
                break
            }
        }
        if self.is_eof() && vals.len() == 0 {
            return Err(self.err_eof())
        }
        if self.same_len {
            if self.first_len == 0 {
                self.first_len = vals.len()
            } else if self.first_len != vals.len() {
                return Err(self.err(format!(
                    "Record has length {} but other records have length {}",
                    vals.len(), self.first_len)))
            }
        }
        // If this assertion fails, then there is a bug in the above code.
        // Namely, the only way `vals` should be empty is if we've hit EOF,
        // which should be returned as en error.
        //
        // This assertion is important because most of the decoder relies on
        // records having non-zero length. For example, if `headers` has zero
        // length, then that indicates that it hasn't been filled yet.
        assert!(vals.len() > 0);
        if self.has_headers && self.headers.len() == 0 {
            self.headers = vals;
            return self.parse_record()
        }
        Ok(vals)
    }

    fn parse_value(&mut self) -> Result<~str, Error> {
        let mut only_whitespace = true;
        let mut res = ~"";
        loop {
            try!(self.next_char());
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
        if res.len() == 0 && self.is_eof() {
            return Err(self.err_eof())
        }
        Ok(res)
    }

    fn parse_quoted_value(&mut self) -> Result<~str, Error> {
        // Assumes that " has already been read.
        let mut res = ~"";
        loop {
            try!(self.next_char());
            if self.is_eof() {
                return Err(self.err("EOF while parsing quoted value."))
            } else if self.cur_is('"') {
                if self.is_escaped_quote() {
                    try!(self.next_char()); // throw away second "
                    res.push_char('"');
                    continue
                }

                // Eat and spit out everything up to next separator.
                // If we see something that isn't whitespace, it's an error.
                loop {
                    try!(self.next_char());
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
            } else if self.cur_is('\\') && self.peek_is('"') {
                // We also try to support \ escaped quotes even though
                // the spec says "" is used.
                try!(self.next_char()); // throw away the "
                res.push_char('"');
                continue
            }

            res.push_char(self.cur.unwrap());
        }
        Ok(res)
    }

    fn is_eof(&self) -> bool {
        self.cur.is_none()
    }

    fn cur_is(&self, c: char) -> bool {
        self.cur == Some(c)
    }

    fn peek(&mut self) -> Result<Option<char>, Error> {
        match self.look {
            Some(c) => Ok(Some(c)),
            None => {
                self.look = try!(self.read_next_char());
                Ok(self.look)
            },
        }
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
}

/// A decoder can decode CSV values (or entire documents) into values with
/// Rust types automatically.
///
/// Raw records (as strings) can also be accessed with the `record` method
/// or with a standard iterator.
pub struct Decoder<'a> {
    priv stack: Vec<Value>,
    priv err: Result<(), Error>,
    priv p: Parser<'a>,
}

/// A representation of a value found in a CSV document.
/// A CSV document's structure is simple (non-recursive).
enum Value {
    Record(Vec<~str>),
    String(~str),
}

impl<'a> Decoder<'a> {
    /// Creates a new CSV decoder that reads CSV data from the `Reader` given.
    /// Note that the `Reader` given may be a stream. Data is only read as it
    /// is decoded.
    pub fn from_reader(r: &mut Reader) -> Decoder<'a> {
        Decoder {
            stack: vec!(),
            err: Ok(()),
            p: Parser {
                sep: ',',
                same_len: true,
                first_len: 0,
                has_headers: false,
                headers: vec!(),
                buf: BufferedReader::new(r),
                cur: Some(0u8 as char),
                look: None,
                line: 1,
                col: 0,
            },
        }
    }

    /// Creates a new CSV decoder that reads CSV data from the string given.
    pub fn from_str(s: &str) -> Decoder<'a> {
        let r = MemReader::new(s.as_bytes().to_owned());
        Decoder::from_reader(~r as ~Reader)
    }

    /// Decodes the next record for some type. Note that since this decodes
    /// records, only types corresponding to a record (like structs, tuples or
    /// vectors) can be used.
    pub fn decode<D: Decodable<Decoder<'a>>>(&mut self) -> Result<D, Error> {
        let d = Decodable::decode(self);
        match self.err {
            Ok(_) => Ok(d),
            Err(ref err) => Err(err.clone()),
        }
    }

    /// Calls `decode` on every record in the CSV data until EOF and returns
    /// them as a vector.
    pub fn decode_all<D: Decodable<Decoder<'a>>>
                     (&mut self) -> Result<Vec<D>, Error> {
        let mut records: Vec<D> = vec!();
        loop {
            match self.decode() {
                Ok(r) => records.push(r),
                Err(err) => if err.eof { break } else { return Err(err) },
            }
        }
        Ok(records)
    }

    /// Cirumvents the decoding interface and forces the parsing of the next
    /// record and returns it. A record returned by this method will never be
    /// decoded.
    pub fn record(&mut self) -> Result<Vec<~str>, Error> {
        self.p.parse_record()
    }

    /// Sets the separator character that delimits values in a record.
    pub fn separator(&mut self, c: char) {
        self.p.sep = c;
    }

    /// When `yes` is `true`, all records decoded must have the same length.
    /// If a record is decoded that has a different length than other records
    /// already decoded, the decoding will fail.
    pub fn enforce_same_length(&mut self, yes: bool) {
        self.p.same_len = yes;
    }

    /// When `yes` is `true`, the first record decoded will be interpreted as
    /// the headers for the CSV data. Each header is represented as a string.
    /// Headers can be accessed at any time with the `headers` method.
    pub fn has_headers(&mut self, yes: bool) {
        self.p.has_headers = yes;
    }

    /// Returns the header record for the underlying CSV data. This method may
    /// be called repeatedly and at any time.
    ///
    /// If `has_headers` is `false` (which is the default), then this will
    /// call `fail!`.
    pub fn headers(&mut self) -> Result<Vec<~str>, Error> {
        if !self.p.has_headers {
            fail!("To get headers from CSV data, has_headers must be called.")
        }
        if self.p.headers.len() == 0 {
            try!(self.read_to_stack());
            assert!(self.p.headers.len() > 0);
        }
        Ok(self.p.headers.clone())
    }
}

impl<'a> Iterator<Vec<~str>> for Decoder<'a> {
    /// Iterates over each record in the CSV data. The iterator stops when
    /// EOF is reached.
    fn next(&mut self) -> Option<Vec<~str>> {
        match self.record() {
            Ok(r) => Some(r),
            Err(err) => {
                if err.eof {
                    None
                } else {
                    fail!("{}", err)
                }
            }
        }
    }
}

impl<'a> Decoder<'a> {
    fn pop(&mut self) -> Result<Value, Error> {
        if self.stack.len() == 0 {
            try!(self.read_to_stack())
        }
        // On successful return, read_to_stack guarantees a non-empty
        // stack.
        assert!(self.stack.len() > 0);
        Ok(self.stack.pop().unwrap())
    }

    fn read_to_stack(&mut self) -> Result<(), Error> {
        let r = try!(self.p.parse_record());
        self.push_record(r);
        Ok(())
    }

    fn pop_record(&mut self) -> Result<Vec<~str>, Error> {
        match try!(self.pop()) {
            Record(r) => Ok(r),
            String(s) => {
                let m = format!("Expected record but got value '{}'.", s);
                Err(self.err(m))
            },
        }
    }

    fn pop_string(&mut self) -> Result<~str, Error> {
        match try!(self.pop()) {
            Record(_) => {
                let m = format!("Expected value but got record.");
                Err(self.err(m))
            },
            String(s) => Ok(s),
        }
    }

    fn pop_from_str<T: FromStr>(&mut self) -> Result<T, Error> {
        let s = try!(self.pop_string());
        match FromStr::from_str(s) {
            Some(t) => Ok(t),
            None => {
                let m = format!("Failed converting '{}' from str.", s);
                Err(self.err(m))
            },
        }
    }

    fn push_record(&mut self, r: Vec<~str>) {
        self.stack.push(Record(r))
    }

    fn push_string(&mut self, s: ~str) {
        self.stack.push(String(s))
    }

    fn err(&self, msg: &str) -> Error {
        self.p.err(msg)
    }

    fn fail(&self, msg: &str) -> ! {
        fail!("{}", self.p.err(msg));
    }
}

impl<'a> serialize::Decoder for Decoder<'a> {
    fn read_nil(&mut self) { unimplemented!() }
    fn read_uint(&mut self) -> uint { dectry!(self.pop_from_str(), 0) }
    fn read_u64(&mut self) -> u64 { dectry!(self.pop_from_str(), 0) }
    fn read_u32(&mut self) -> u32 { dectry!(self.pop_from_str(), 0) }
    fn read_u16(&mut self) -> u16 { dectry!(self.pop_from_str(), 0) }
    fn read_u8(&mut self) -> u8 { dectry!(self.pop_from_str(), 0) }
    fn read_int(&mut self) -> int { dectry!(self.pop_from_str(), 0) }
    fn read_i64(&mut self) -> i64 { dectry!(self.pop_from_str(), 0) }
    fn read_i32(&mut self) -> i32 { dectry!(self.pop_from_str(), 0) }
    fn read_i16(&mut self) -> i16 { dectry!(self.pop_from_str(), 0) }
    fn read_i8(&mut self) -> i8 { dectry!(self.pop_from_str(), 0) }
    fn read_bool(&mut self) -> bool { dectry!(self.pop_from_str(), false) }
    fn read_f64(&mut self) -> f64 { dectry!(self.pop_from_str(), 0.0) }
    fn read_f32(&mut self) -> f32 { dectry!(self.pop_from_str(), 0.0) }
    fn read_char(&mut self) -> char {
        let s = dectry!(self.pop_string(), '\x00');
        let chars = s.chars().collect::<~[_]>();
        if chars.len() != 1 {
            self.fail(format!("Expected single character but got '{}'.", s))
        }
        chars[0]
    }
    fn read_str(&mut self) -> ~str {
        dectry!(self.pop_string(), ~"")
    }
    fn read_enum<T>(&mut self, _: &str, f: |&mut Decoder<'a>| -> T) -> T {
        f(self)
    }
    fn read_enum_variant<T>(&mut self, names: &[&str],
                            f: |&mut Decoder<'a>, uint| -> T) -> T {
        let variant = to_lower(dectry!(self.pop_string(), f(self, 0)));
        match names.iter().position(|&name| to_lower(name) == variant) {
            Some(idx) => f(self, idx),
            None => {
                let m = format!("Expected one of {} but found '{}'.",
                                names, variant);
                self.err = Err(self.err(m));
                f(self, 0)
            },
        }
    }
    fn read_enum_variant_arg<T>(&mut self, _: uint,
                                f: |&mut Decoder<'a>| -> T) -> T {
        let m = ~"Cannot decode into enum variants with arguments.";
        self.err = Err(self.err(m));
        f(self)
    }
    fn read_enum_struct_variant<T>(&mut self, names: &[&str],
                                   f: |&mut Decoder<'a>, uint| -> T) -> T {
        self.read_enum_variant(names, f)
    }
    fn read_enum_struct_variant_field<T>(&mut self, _: &str, f_idx: uint,
                                         f: |&mut Decoder<'a>| -> T) -> T {
        self.read_enum_variant_arg(f_idx, f)
    }
    fn read_struct<T>(&mut self, s_name: &str, len: uint,
                      f: |&mut Decoder<'a>| -> T) -> T {
        let r = dectry!(self.pop_record(), f(self));
        if r.len() != len {
            let m = format!("Struct '{}' has {} fields but current record \
                             has {} fields.", s_name, len, r.len());
            self.err = Err(self.err(m));
            return f(self)
        }
        for v in r.move_iter().rev() {
            self.push_string(v)
        }
        f(self)
    }
    fn read_struct_field<T>(&mut self, _: &str, _: uint,
                            f: |&mut Decoder<'a>| -> T) -> T {
        f(self)
    }
    fn read_tuple<T>(&mut self, _: |&mut Decoder<'a>, uint| -> T) -> T {
        unimplemented!()
    }
    fn read_tuple_arg<T>(&mut self, _: uint, _: |&mut Decoder<'a>| -> T) -> T {
        unimplemented!()
    }
    fn read_tuple_struct<T>(&mut self, _: &str,
                            _: |&mut Decoder<'a>, uint| -> T) -> T {
        unimplemented!()
    }
    fn read_tuple_struct_arg<T>(&mut self, _: uint,
                                _: |&mut Decoder<'a>| -> T) -> T {
        unimplemented!()
    }
    fn read_option<T>(&mut self, _: |&mut Decoder<'a>, bool| -> T) -> T {
        unimplemented!()
    }
    fn read_seq<T>(&mut self, f: |&mut Decoder<'a>, uint| -> T) -> T {
        let r = dectry!(self.pop_record(), f(self, 0));
        let len = r.len();
        for v in r.move_iter().rev() {
            self.push_string(v)
        }
        f(self, len)
    }
    fn read_seq_elt<T>(&mut self, _: uint, f: |&mut Decoder<'a>| -> T) -> T {
        f(self)
    }
    fn read_map<T>(&mut self, _: |&mut Decoder<'a>, uint| -> T) -> T {
        unimplemented!()
    }
    fn read_map_elt_key<T>(&mut self, _: uint,
                           _: |&mut Decoder<'a>| -> T) -> T {
        unimplemented!()
    }
    fn read_map_elt_val<T>(&mut self, _: uint,
                           _: |&mut Decoder<'a>| -> T) -> T {
        unimplemented!()
    }
}

fn to_lower(s: &str) -> ~str {
    s.chars().map(|c| c.to_lowercase()).collect()
}

#[cfg(test)]
mod test {
    use quickcheck::{TestResult, quickcheck};
    use super::{StrEncoder, Decoder};

    #[deriving(Show, Encodable, Decodable)]
    enum Color {
        Red, Green, Blue
    }

    #[deriving(Show, Encodable, Decodable)]
    struct Record {
        color: Color,
        a: f64,
        b: ~str,
        c: ~str,
    }

    #[test]
    fn same_record() {
        fn prop(input: Vec<~str>) -> TestResult {
            if input.len() == 0 {
                return TestResult::discard()
            }

            let mut senc = StrEncoder::new();
            senc.encode(input.as_slice());

            let mut dec = Decoder::from_str(senc.to_str());
            let output: Vec<~str> = dec.decode().unwrap();

            TestResult::from_bool(input == output)
        }
        quickcheck(prop);
    }

    #[test]
    fn same_records() {
        fn prop(to_repeat: Vec<~str>, n: uint) -> TestResult {
            if to_repeat.len() == 0 || n == 0 {
                return TestResult::discard()
            }

            let input = Vec::from_fn(n, |_| to_repeat.clone());
            let mut senc = StrEncoder::new();
            senc.encode_all(input.as_slice());

            let mut dec = Decoder::from_str(senc.to_str());
            let output: Vec<Vec<~str>> = dec.decode_all().unwrap();

            TestResult::from_bool(input == output)
        }
        quickcheck(prop);
    }

    #[test]
    fn encoder_simple() {
        let mut senc = StrEncoder::new();
        senc.encode(("springsteen", 's', 1, 0.14, false));
        assert_eq!("springsteen,s,1,0.14,false\n", senc.to_str());
    }

    #[test]
    fn encoder_simple_crlf() {
        let mut senc = StrEncoder::new();
        senc.encoder.crlf(true);
        senc.encode(("springsteen", 's', 1, 0.14, false));
        assert_eq!("springsteen,s,1,0.14,false\r\n", senc.to_str());
    }

    #[test]
    fn encoder_simple_tabbed() {
        let mut senc = StrEncoder::new();
        senc.encoder.separator('\t');
        senc.encode(("springsteen", 's', 1, 0.14, false));
        assert_eq!("springsteen\ts\t1\t0.14\tfalse\n", senc.to_str());
    }

    #[test]
    fn encoder_same_length_records() {
        let mut senc = StrEncoder::new();
        senc.encoder.enforce_same_length(true);
        senc.encode(vec!('a'));
        match senc.encoder.encode(vec!('a', 'b')) {
            Ok(_) => fail!("Encoder should report an error when records of \
                            varying length are added and records of same \
                            length is enabled."),
            Err(_) => {},
        }
    }

    #[test]
    fn encoder_quoted_quotes() {
        let mut senc = StrEncoder::new();
        senc.encode(vec!("sprin\"g\"steen"));
        assert_eq!("\"sprin\"\"g\"\"steen\"\n", senc.to_str());
    }

    #[test]
    fn encoder_quoted_sep() {
        let mut senc = StrEncoder::new();
        senc.encoder.separator(',');
        senc.encode(vec!("spring,steen"));
        assert_eq!("\"spring,steen\"\n", senc.to_str());
    }

    #[test]
    fn encoder_quoted_newlines() {
        let mut senc = StrEncoder::new();
        senc.encode(vec!("spring\nsteen"));
        assert_eq!("\"spring\nsteen\"\n", senc.to_str());
    }

    #[test]
    fn encoder_zero() {
        let mut senc = StrEncoder::new();
        match senc.encoder.encode::<Vec<int>>(vec!()) {
            Ok(_) => fail!("Encoder should report an error when trying to \
                            encode records of length 0."),
            Err(_) => {},
        }
    }

    #[test]
    fn decoder_simple_nonl() {
        let mut d = Decoder::from_str("springsteen,s,1,0.14,false");
        let r: (~str, char, int, f64, bool) = d.decode().unwrap();
        assert_eq!(r, (~"springsteen", 's', 1, 0.14, false));
    }

    #[test]
    fn decoder_simple() {
        let mut d = Decoder::from_str("springsteen,s,1,0.14,false\n");
        let r: (~str, char, int, f64, bool) = d.decode().unwrap();
        assert_eq!(r, (~"springsteen", 's', 1, 0.14, false));
    }

    #[test]
    fn decoder_simple_crlf() {
        let mut d = Decoder::from_str("springsteen,s,1,0.14,false\r\n");
        let r: (~str, char, int, f64, bool) = d.decode().unwrap();
        assert_eq!(r, (~"springsteen", 's', 1, 0.14, false));
    }

    #[test]
    fn decoder_simple_tabbed() {
        let mut d = Decoder::from_str("springsteen\ts\t1\t0.14\tfalse\r\n");
        d.separator('\t');
        let r: (~str, char, int, f64, bool) = d.decode().unwrap();
        assert_eq!(r, (~"springsteen", 's', 1, 0.14, false));
    }

    #[test]
    fn decoder_same_length_records() {
        let mut d = Decoder::from_str("a\na,b");
        d.enforce_same_length(true);
        match d.decode_all::<Vec<~str>>() {
            Ok(_) => fail!("Decoder should report an error when records of \
                            varying length are decoded and records of same \
                            length if enabled."),
            Err(_) => {},
        }
    }

    #[test]
    #[should_fail]
    fn decoder_bad_header_access() {
        let mut d = Decoder::from_str("");
        d.has_headers(false);
        let _ = d.headers();
    }

    // #[test] 
    // fn wat() { 
        // let mut dec = Decoder::from_str("0\n"); 
        // match dec.decode_all::<Vec<int>>() { 
            // Ok(all) => debug!("====== WAT: {}", all), 
            // Err(err) => fail!("eof? {} ======== {}", err.eof, err), 
        // } 
    // } 
}
