#[crate_id = "csv#0.1.0"];
#[crate_type = "lib"];
#[license = "UNLICENSE"];
#[doc(html_root_url = "http://burntsushi.net/rustdoc/rust-csv")];

//! This crate provides a CSV encoder and decoder that works with Rust's
//! `serialize` crate.

#[feature(macro_rules)];
// Dunno what this is, but apparently it's required for the 'log' crate.
#[feature(phase)];

#[phase(syntax, link)] extern crate log;
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

macro_rules! opttry(
    ($e:expr) => (match $e { Some(e) => return Err(e), None => {}, })
)

macro_rules! enctry(
    ($e:expr) => (
        if self.err.is_ok() {
            match $e {
                Ok(e) => e,
                Err(e) => { self.err = Err(e); return },
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
    /// convenient since they can't (theoretically) fail as a result of an
    /// IO error.
    encoder: Encoder<'a>,
    priv w: MemWriter,
}

impl<'a> StrEncoder<'a> {
    /// Creates a new CSV string encoder. At any time, `to_str` can be called
    /// to retrieve the cumulative CSV data.
    pub fn new() -> StrEncoder<'a> {
        let mut w = MemWriter::new();
        let enc = Encoder::to_writer(&mut w as &mut Writer);
        StrEncoder {
            encoder: enc,
            w: w,
        }
    }

    /// Returns the encoded CSV data as a string.
    pub fn to_str(self) -> ~str {
        str::from_utf8_owned(self.w.unwrap()).unwrap()
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
    pub fn encode_all<E: Encodable<Encoder<'a>>>(&mut self, es: Vec<E>) {
        match self.encoder.encode_all(es) {
            Ok(_) => {},
            Err(err) => fail!("{}", err),
        }
     }
}

/// An encoder can encode Rust values into CSV records or documents.
pub struct Encoder<'a> {
    priv buf: &'a mut Writer,
    priv err: IoResult<()>,
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
    pub fn encode<E: Encodable<Encoder<'a>>>(&mut self, e: E) -> IoResult<()> {
        e.encode(self);
        self.err.clone()
    }

    /// Calls `encode` on each element in the vector given.
    pub fn encode_all<E: Encodable<Encoder<'a>>>
                     (&mut self, es: Vec<E>) -> IoResult<()> {
        for e in es.move_iter() {
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

    fn write_to_str<T: fmt::Show>(&mut self, t: T) {
        enctry!(self.buf.write_str(t.to_str()))
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
        enctry!(self.buf.write_str(::std::f64::to_str_digits(v, 10)))
    }
    fn emit_f32(&mut self, v: f32) {
        enctry!(self.buf.write_str(::std::f32::to_str_digits(v, 10)))
    }
    fn emit_char(&mut self, v: char) { self.write_to_str(v) }
    fn emit_str(&mut self, v: &str) {
        let s = self.quoted(v).to_str();
        enctry!(self.buf.write_str(s))
    }
    fn emit_enum(&mut self, _: &str, f: |&mut Encoder<'a>|) {
        f(self)
    }
    fn emit_enum_variant(&mut self, v_name: &str, _: uint, _: uint,
                         _: |&mut Encoder<'a>|) {
        enctry!(self.buf.write_str(v_name))
    }
    fn emit_enum_variant_arg(&mut self, _: uint, _: |&mut Encoder<'a>|) {
        fail!("Cannot encode enum variants with arguments.")
    }
    fn emit_enum_struct_variant(&mut self, v_name: &str, v_id: uint, len: uint,
                                f: |&mut Encoder<'a>|) {
        self.emit_enum_variant(v_name, v_id, len, f)
    }
    fn emit_enum_struct_variant_field(&mut self, _: &str, _: uint,
                                      _: |&mut Encoder<'a>|) {
        fail!("Cannot encode enum variants with arguments.")
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
        if self.same_len {
            if self.first_len == 0 {
                self.first_len = len
            } else if self.first_len != len {
                fail!("Record has length {} but other records have length {}",
                      len, self.first_len)
            }
        }
        f(self);
        if self.use_crlf {
            enctry!(self.buf.write_str("\r\n"))
        } else {
            enctry!(self.buf.write_str("\n"))
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
                    if err.kind == EndOfFile {
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

    fn parse_record(&mut self) -> Result<Vec<~str>, Error> {
        let mut vals: Vec<~str> = vec!();
        while !self.is_eof() {
            let val = try!(self.parse_value());
            vals.push(val);
            if self.is_lineterm() {
                // If it's a CRLF ending, consume the '\r'
                if self.cur_is('\r') { opttry!(self.next_char()) }
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
                    continue
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
            } else if self.cur_is('\\') && self.peek_is('"') {
                // We also try to support \ escaped quotes even though
                // the spec says "" is used.
                opttry!(self.next_char()); // throw away the "
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
                self.look = try!(self.read_next());
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
    priv err: Option<Error>,
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
            err: None,
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
    ///
    /// If there is an error decoding the CSV data, `fail!` is called. (Sorry,
    /// hopefully this will change at some point.)
    pub fn decode<D: Decodable<Decoder<'a>>>(&mut self) -> D {
        Decodable::decode(self)
    }

    /// Calls `decode` on every record in the CSV data until EOF and returns
    /// them as a vector. (Sorry, hopefully this will change at some point.)
    pub fn decode_all<D: Decodable<Decoder<'a>>>(&mut self) -> Vec<D> {
        let mut records: Vec<D> = vec!();
        while !self.p.is_eof() {
            records.push(self.decode())
        }
        records
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
    /// be called at any time.
    ///
    /// If `has_headers` is `false` (which is the default), then this will
    /// always return an empty vector.
    pub fn headers(&mut self) -> Vec<~str> {
        if !self.p.has_headers {
            return vec!()
        }
        if self.p.headers.len() == 0 {
            self.read_to_stack();
            assert!(self.p.headers.len() > 0);
        }
        self.p.headers.clone()
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
    fn pop(&mut self) -> Value {
        if self.stack.len() == 0 {
            self.read_to_stack()
        }
        self.stack.pop().unwrap()
    }

    fn read_to_stack(&mut self) {
        match self.p.parse_record() {
            Err(err) => fail!("{}", err),
            Ok(r) => self.push_record(r),
        }
    }

    fn pop_record(&mut self) -> Vec<~str> {
        match self.pop() {
            Record(r) => r,
            String(s) =>
                self.fail(format!("Expected record but got value '{}'.", s)),
        }
    }

    fn pop_string(&mut self) -> ~str {
        match self.pop() {
            Record(_) => self.fail(format!("Expected value but got record.")),
            String(s) => s,
        }
    }

    fn pop_from_str<T: FromStr>(&mut self) -> T {
        let s = self.pop_string();
        match FromStr::from_str(s) {
            None => self.fail(format!("Failed converting '{}' from str.", s)),
            Some(t) => t,
        }
    }

    fn push_record(&mut self, r: Vec<~str>) {
        self.stack.push(Record(r))
    }

    fn push_string(&mut self, s: ~str) {
        self.stack.push(String(s))
    }

    fn fail(&self, msg: &str) -> ! {
        fail!("{}", self.p.err(msg));
    }
}

impl<'a> serialize::Decoder for Decoder<'a> {
    fn read_nil(&mut self) { unimplemented!() }
    fn read_uint(&mut self) -> uint { self.pop_from_str() }
    fn read_u64(&mut self) -> u64 { self.pop_from_str() }
    fn read_u32(&mut self) -> u32 { self.pop_from_str() }
    fn read_u16(&mut self) -> u16 { self.pop_from_str() }
    fn read_u8(&mut self) -> u8 { self.pop_from_str() }
    fn read_int(&mut self) -> int { self.pop_from_str() }
    fn read_i64(&mut self) -> i64 { self.pop_from_str() }
    fn read_i32(&mut self) -> i32 { self.pop_from_str() }
    fn read_i16(&mut self) -> i16 { self.pop_from_str() }
    fn read_i8(&mut self) -> i8 { self.pop_from_str() }
    fn read_bool(&mut self) -> bool { self.pop_from_str() }
    fn read_f64(&mut self) -> f64 { self.pop_from_str() }
    fn read_f32(&mut self) -> f32 { self.pop_from_str() }
    fn read_char(&mut self) -> char {
        let s = self.pop_string();
        let chars = s.chars().to_owned_vec();
        if chars.len() != 1 {
            self.fail(format!("Expected single character but got '{}'.", s))
        }
        chars[0]
    }
    fn read_str(&mut self) -> ~str {
        self.pop_string()
    }
    fn read_enum<T>(&mut self, _: &str, f: |&mut Decoder<'a>| -> T) -> T {
        f(self)
    }
    fn read_enum_variant<T>(&mut self, names: &[&str],
                            f: |&mut Decoder<'a>, uint| -> T) -> T {
        let variant = to_lower(self.pop_string());
        match names.iter().position(|&name| to_lower(name) == variant) {
            Some(idx) => f(self, idx),
            None => self.fail(format!("Expected one of {} but found '{}'.",
                                      names, variant)),
        }
    }
    fn read_enum_variant_arg<T>(&mut self, _: uint,
                                _: |&mut Decoder<'a>| -> T) -> T {
        self.fail("Cannot decode into enum variants with arguments.")
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
        let r = self.pop_record();
        if r.len() != len {
            self.fail(format!("Struct '{}' has {} fields but current record \
                               has {} fields.", s_name, len, r.len()))
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
        let r = self.pop_record();
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
    fn enc() {
        // let r = vec!(
            // Record{color: Green, a: 0.14, b: ~"bbb", c: ~"c", }, 
            // Record{color: Red, a: 1f64, b: ~"y", c: ~"z", }, 
        // ); 
        let r = vec!(
            (Green, 0.14, ~"b:bb", ~"c"),
            (Red, 1f64, ~"y", ~"z")
        );
        // let r = vec!(
            // vec!('a', 'b', 'c'), 
            // vec!('y', 'z'), 
        // ); 
        let mut senc = StrEncoder::new();
        senc.encode_all(r);
        debug!("{}", senc.to_str());
    }

    #[test]
    fn wat() {
        let mut dec = Decoder::from_str("green,-0.14, \"\",c\r\nred,1,y,z");
        let all: Vec<(Color, f64, ~str, ~str)> = dec.decode_all();
        debug!("{}", all);

        debug!("-------------------");

        let mut dec = Decoder::from_str(
                         "A:B:C:D\ngreen:-0.14: \"\":c\r\nred:1:y:z");
        dec.separator(':');
        dec.has_headers(true);
        debug!("HEADERS: {}", dec.headers());
        loop {
            match dec.record() {
                Err(err) => if err.eof { break } else { fail!("{}", err) },
                Ok(r) => debug!("RECORD: {}", r),
            }
        }

        debug!("-------------------");

        let mut dec = Decoder::from_str("green:-0.14: \"\":c\r\nred:1:y:z");
        dec.separator(':');
        for r in dec {
            debug!("RECORD: {}", r)
        }
    }
}
