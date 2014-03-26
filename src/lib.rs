#[crate_id = "csv#0.1.0"];
#[crate_type = "lib"];
#[license = "UNLICENSE"];
#[doc(html_root_url = "http://burntsushi.net/rustdoc/csv")];

//! This crate provides a streaming CSV (comma separated values) encoder and 
//! decoder that works with the `Encoder` and `Decoder` traits in Rust's 
//! `serialize` crate.
//!
//! A CSV file is composed of a list of records where each record starts on
//! a new line. A record is composed of 1 or more values delimited by a comma
//! or some other character. The first record may optionally correspond to a
//! record of labels corresponding to their respective positions.
//!
//! Example data:
//!
//! ```ignore
//! 1997,Ford,,
//! "1997", "Ford", "E350", "Super, luxurious truck"
//! 1997,Ford,E350, "Go get one now
//! they are going fast"
//! ```
//!
//! Note that in the above data, there is a total of 3 records. Each record
//! has length 4.
//!
//! If this data is in a file called `foo.csv`, then its records can be
//! read as vectors of strings using an iterator:
//!
//! ```no_run
//! let mut rdr = csv::Decoder::from_file(&Path::new("foo.csv"));
//! for record in rdr.iter() {
//!     println!("{}", record);
//! }
//! ```
//!
//!
//! ## Headers and delimiters
//!
//! By default, the decoder in this crate assumes that the CSV data contains
//! no header record. Therefore, we must tell the decoder that there is a
//! header record before we start parsing with the `has_headers` method.
//! Then we can access the header record at any time with the `headers` method:
//!
//! ```
//! let mut rdr = csv::Decoder::from_str("abc,xyz\n1,2");
//! rdr.has_headers(true);
//! 
//! assert_eq!(rdr.headers().unwrap(), vec!(~"abc", ~"xyz"));
//! assert_eq!(rdr.iter().next().unwrap(), vec!(~"1", ~"2"));
//! assert_eq!(rdr.headers().unwrap(), vec!(~"abc", ~"xyz"));
//! ```
//!
//! The decoder also assumes that a comma (`,`) is the delimiter used to
//! separate values in a record. This can be changed with the `separator`
//! method. For example, to read tab separated values:
//!
//! ```
//! let mut rdr = csv::Decoder::from_str("a\tb\ny\tz");
//! rdr.separator('\t');
//! 
//! assert_eq!(rdr.iter().next().unwrap(), vec!(~"a", ~"b"));
//! assert_eq!(rdr.iter().next().unwrap(), vec!(~"y", ~"z"));
//! ```
//!
//!
//! ## Decoding
//!
//! Like the `serialize::json` crate, this crate supports encoding and decoding
//! into Rust values with types that satisfy the `Encodable` and/or
//! `Decodable` traits. In this crate, encoding and decoding always works at
//! the level of a CSV record. That is, only types corresponding to CSV
//! records can be given to the encode/decode methods. (Includes, but is not
//! limited to, structs, vectors and tuples.)
//!
//! Given the simple structure of a CSV file, this makes it
//! very simple to retrieve records as tuples. For example:
//!
//! ```
//! let mut rdr = csv::Decoder::from_str("andrew,1987\nkait,1989");
//! let record: (~str, uint) = rdr.decode().unwrap();
//! assert_eq!(record, (~"andrew", 1987));
//! ```
//!
//! An iterator is provided to repeat this for all records in the CSV data:
//!
//! ```
//! let mut rdr = csv::Decoder::from_str("andrew,1987\nkait,1989");
//! for (name, birth) in rdr.decode_iter::<(~str, uint)>() {
//!     println!("Name: {}, Born: {}", name, birth);
//! }
//! ```
//!
//! Note that the `decode_iter` is *explicitly* instantiated with the type
//! of the record. Alternatively, you could create the iterator with a let
//! binding and an explicit type annotation:
//!
//! ```
//! # let mut rdr = csv::Decoder::from_str("andrew,1987\nkait,1989");
//! let mut it: csv::DecodedItems<(~str, uint)> = rdr.decode_iter();
//! ```
//!
//! While this is convenient, CSV data in the real world can often be messy
//! or incomplete. For example, maybe some records don't have a birth year:
//!
//! ```ignore
//! andrew, ""
//! kait,
//! ```
//!
//! Using the above code, this would produce a decoding error since the empty
//! value `""` cannot be decoded into a value with type `uint`. Thankfully this
//! is easily fixed with an `Option` type. We only need to change the type
//! in our previous example:
//!
//! ```
//! let mut rdr = csv::Decoder::from_str("andrew, \"\"\nkait,");
//! let record1: (~str, Option<uint>) = rdr.decode().unwrap();
//! let record2: (~str, Option<uint>) = rdr.decode().unwrap();
//!
//! assert_eq!(record1, (~"andrew", None));
//! assert_eq!(record2, (~"kait", None));
//! ```
//!
//! The `None` value here basically represents the fact that the decoder could
//! not decode the value to a `uint`. In particular, if the value were `"abc"`
//! instead of `""`, then the output would be the same. Therefore, `None`
//! represents a *conversion failure* rather than just an empty or NULL value.
//!
//! We can take this one step further with enumerations. For example, sometimes
//! values are encoded with a variety of different types. As a contrived
//! example, consider values that use any of `1`, `0`, `true` or `false`.
//! None of these values are invalid, so we'd like to decode any of them. This 
//! can be expressed with an `enum` type:
//!
//! ```
//! extern crate csv;
//! extern crate serialize;
//!
//! #[deriving(Eq, Show, Decodable)]
//! enum Truthy {
//!     Uint(uint),
//!     Bool(bool),
//! }
//!
//! fn main() {
//!     let mut rdr = csv::Decoder::from_str("andrew,false\nkait,1");
//!     let record: (~str, Truthy) = rdr.decode().unwrap();
//!     assert_eq!(record, (~"andrew", Bool(false)));
//! }
//! ```
//!
//! When the decoder sees an enum, it first tries to match CSV values with
//! the names of the value constructors (case insensitive). If that fails, then
//! it will try to match the CSV value against the first argument type. The
//! first match with a successful conversion will be used.
//!
//! Currently, the decoder only supports enum types with any mix of value
//! constructors that have 0 or 1 arguments. Using a value constructor with
//! more than one argument will result in a decoding error.
//!
//! Finally, decoding also works with structs by matching values in a record
//! to fields in a struct based on position. If a struct has a different
//! number of fields than a CSV record, an error is returned.
//!
//!
//! ## Encoding
//!
//! Using the encoder in this crate is almost exactly like using the
//! decoder:
//!
//! ```
//! let mut enc = csv::StrEncoder::new();
//! enc.encode(("andrew", 1987));
//! enc.encode(("kait", 1989));
//! assert_eq!(enc.to_str(), "andrew,1987\nkait,1989\n");
//! ```
//!
//! Note that `StrEncoder` is a convenience for encoding to strings. You can
//! encode to any `Writer` (with `to_writer`) or to a file:
//!
//! ```no_run
//! let mut enc = csv::Encoder::to_file(&Path::new("foo.csv"));
//! let records = vec!(("andrew", 1987), ("kait", 1989));
//! match enc.encode_all(records.as_slice()) {
//!     Ok(_) => {},
//!     Err(err) => fail!("Error encoding: {}", err),
//! }
//! ```
//!
//! The encoder in this crate supports all of the same things as the decoder,
//! including writing enum and option types. The encoder will make sure that
//! values containing special characters (like quotes, new lines or the
//! delimiter) are appropriately quoted.
//!
//!
//! ## Streaming
//!
//! All decoding and encoding in this crate is done on demand. That is, you
//! can safely pass a reader to a decoder and expect that it won't be
//! completely consumed immediately.
//!
//! Here's an example that demonstrates streaming with channels:
//!
//! ```no_run
//! extern crate csv;
//! 
//! use std::comm::channel;
//! use std::io::{ChanReader, ChanWriter, Reader, Writer};
//! use std::io::timer::sleep;
//! use std::task::spawn;
//! 
//! use csv::{Decoder, Encoder};
//! 
//! fn main() {
//!     let (send, recv) = channel();
//!     spawn(proc() {
//!         let mut w = ChanWriter::new(send);
//!         let mut enc = Encoder::to_writer(&mut w as &mut Writer);
//!         for x in range(1, 6) {
//!             match enc.encode((x, x * x)) {
//!                 Ok(_) => {},
//!                 Err(err) => fail!("Failed encoding: {}", err),
//!             }
//!             sleep(1000);
//!         }
//!     });
//! 
//!     let mut r = ChanReader::new(recv);
//!     // We create a CSV reader with a small buffer so that we can see streaming
//!     // in action on small inputs.
//!     let mut dec = Decoder::from_reader_capacity(&mut r as &mut Reader, 1);
//!     for r in dec.iter() {
//!         println!("Record: {}", r);
//!     }
//! }
//! ```
//!
//!
//! ## Compliance with RFC 4180
//!
//! RFC 4180 seems to the closest thing to an official specification for CSV.
//! This crate should conform to the specification with these exceptions:
//! (which are mostly used for making the decoder more permissive)
//!
//!   * Both CRLF and LF line endings are supported. This is seamless in the
//!     decoder. By default, the encoder uses LF line endings but can be
//!     instructed to use CRLF with the `crlf` method.
//!   * The first record is read as a "header" if and only if `has_headers`
//!     has been called with `true`. This is off by default.
//!     The encoder has no explicit support for headers. Simply encode a
//!     vector of strings instead.
//!   * By default, the delimiter is a comma, but it can be changed to any
//!     unicode character with the `separator` method (for either encoding
//!     or decoding).
//!   * The decoder interprets `\"` as an escaped quote in addition to the
//!     standard `""`.
//!   * By default, both the encoder and decoder will enforce the invariant
//!     that all records are the same length. (This is what RFC 4180 demands.)
//!     If a record with a different length is found, an error is returned.
//!     This behavior may be turned off by calling `enforce_same_length` with
//!     `false`.
//!   * Empty lines (that do not include other whitespace) are ignored 
//!     by the decoder.
//!   * Only UTF-8 is supported (and therefore ASCII). Bytes that cannot be
//!     decoded into UTF-8 will be ignored by the decoder.
//!
//! Everything else should be supported, including new lines in quoted values.

#[feature(macro_rules)];
// Dunno what this is, but apparently it's required for the 'log' crate.
#[feature(phase)];

#[phase(syntax, link)] extern crate log;
// extern crate quickcheck; 
extern crate rand;
extern crate serialize;
extern crate stdtest = "test";

use std::default::Default;
use std::fmt;
use std::from_str::FromStr;
use std::io::{BufferedReader};
use std::io::{Reader, Writer};
use std::io::{File, IoResult, MemReader, MemWriter};
use std::io::{EndOfFile, InvalidInput};
use std::iter::Iterator;
use std::path::Path;
use std::str;
use serialize::{Encodable, Decodable};

macro_rules! enctry(
    ($e:expr) => (
        if self.err.is_err() {
            return
        } else {
            match $e {
                Ok(e) => e,
                Err(e) => { self.err = Err(e.to_str()); return }
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
                Err(e) => { self.err = Err(e); return $d }
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
            Ok(_) => {}
            Err(err) => fail!("{}", err),
        }
    }

    /// This is just like `Encoder::encode_all`, except it calls `fail!` if 
    /// there was an error.
    pub fn encode_all<E: Encodable<Encoder<'a>>>(&mut self, es: &[E]) {
        match self.encoder.encode_all(es) {
            Ok(_) => {}
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
    /// Creates an encoder that writes the file given. If the file doesn't
    /// exist, then it is created. If it already exists, then it is truncated
    /// before writing.
    pub fn to_file(path: &Path) -> Encoder<'a> {
        let file = File::create(path);
        Encoder::to_writer(~file as ~Writer)
    }

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
        let sep = self.sep;
        let quotable = |c: char| c == sep || c == '\n' || c == '"';
        if s.len() == 0 || s.find(quotable).is_some() {
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
    fn emit_enum_variant(&mut self, v_name: &str, _: uint, len: uint,
                         f: |&mut Encoder<'a>|) {
        match len {
            0 => enctry!(self.w(v_name)),
            1 => f(self),
            _ => self.err = Err(~"Cannot encode enum variants with more \
                                  than one argument."),
        }
    }
    fn emit_enum_variant_arg(&mut self, _: uint, f: |&mut Encoder<'a>|) {
        f(self)
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
    fn emit_tuple(&mut self, len: uint, f: |&mut Encoder<'a>|) {
        self.emit_seq(len, f)
    }
    fn emit_tuple_arg(&mut self, idx: uint, f: |&mut Encoder<'a>|) {
        self.emit_seq_elt(idx, f)
    }
    fn emit_tuple_struct(&mut self, _: &str, _: uint,
                         _: |&mut Encoder<'a>|) {
        unimplemented!()
    }
    fn emit_tuple_struct_arg(&mut self, _: uint, _: |&mut Encoder<'a>|) {
        unimplemented!()
    }
    fn emit_option(&mut self, f: |&mut Encoder<'a>|) {
        f(self)
    }
    fn emit_option_none(&mut self) { }
    fn emit_option_some(&mut self, f: |&mut Encoder<'a>|) { f(self) }
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
            Some(c) => { self.look = None; Ok(Some(c)) }
            None => match self.buf.read_char() {
                Ok(c) => Ok(Some(c)),
                Err(err) => {
                    match err.kind {
                        EndOfFile => Ok(None),
                        InvalidInput => {
                            // Ignore invalid input.
                            self.read_next_char()
                        }
                        _ => Err(self.err(format!(
                                 "Could not read char [{}]: {} (detail: {})",
                                 err.kind, err, err.detail))),
                    }
                }
            }
        }
    }

    fn parse_record(&mut self, as_header: bool) -> Result<Vec<~str>, Error> {
        try!(self.eat_lineterms());
        if self.peek_is_eof() {
            return Err(self.err_eof())
        }

        let mut vals = Vec::with_capacity(self.first_len);
        while !self.is_eof() {
            let val = try!(self.parse_value());
            vals.push(val);
            if self.is_lineterm() {
                try!(self.eat_lineterm());
                try!(self.eat_lineterms());
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
        } else if self.first_len == 0 {
            // This isn't used to enforce same length records, but as a hint
            // for how big to make a vector holding the record.
            self.first_len = vals.len()
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
            if as_header {
                return Ok(self.headers.clone())
            }
            return self.parse_record(false)
        }
        Ok(vals)
    }

    fn parse_value(&mut self) -> Result<~str, Error> {
        let mut only_whitespace = true;
        let mut res = str::with_capacity(4);
        loop {
            try!(self.next_char());
            if self.is_lineterm() || self.is_eof() || self.is_sep() {
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
        let mut res = str::with_capacity(4);
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
                try!(self.next_char());
                loop {
                    if self.is_lineterm() || self.is_eof() || self.is_sep() {
                        break
                    } else if !self.cur.unwrap().is_whitespace() {
                        let msg = format!(
                            "Expected EOF, line terminator, separator or \
                            whitespace following quoted value but found \
                            '{}' instead.", self.cur.unwrap());
                        return Err(self.err(msg));
                    }
                    try!(self.next_char());
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
            }
        }
    }

    fn peek_is(&mut self, c: char) -> bool {
        match self.peek() {
            Ok(Some(p)) => p == c,
            _ => false,
        }
    }

    fn peek_is_eof(&mut self) -> bool {
        match self.peek() {
            Ok(None) => true,
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

    fn eat_lineterms(&mut self) -> Result<(), Error> {
        while self.peek_is('\n') || self.peek_is('\r') {
            try!(self.next_char()); // read a '\r' or a '\n'
            try!(self.eat_lineterm()); // read a '\n' if read '\r' ^^
        }
        Ok(())
    }

    fn eat_lineterm(&mut self) -> Result<(), Error> {
        if self.cur_is('\r') {
            try!(self.next_char());
        }
        Ok(())
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
    /// Creates a new CSV decoder from a file using the file path given.
    pub fn from_file(path: &Path) -> Decoder<'a> {
        let file = File::open(path);
        Decoder::from_reader(~file as ~Reader)
    }

    /// Creates a new CSV decoder that reads CSV data from the `Reader` given.
    /// Note that the `Reader` given may be a stream. Data is only read as it
    /// is decoded.
    ///
    /// The reader given is wrapped in a `BufferedReader` for you.
    pub fn from_reader(r: &mut Reader) -> Decoder<'a> {
        Decoder::from_buffer(BufferedReader::new(r))
    }

    /// This is just like `from_reader`, except it allows you to specify
    /// the capacity used in the underlying buffer.
    pub fn from_reader_capacity(r: &mut Reader, cap: uint) -> Decoder<'a> {
        Decoder::from_buffer(BufferedReader::with_capacity(cap, r))
    }

    /// Creates a new CSV decoder that reads CSV data from the string given.
    pub fn from_str(s: &str) -> Decoder<'a> {
        let r = MemReader::new(s.as_bytes().to_owned());
        Decoder::from_reader(~r as ~Reader)
    }

    fn from_buffer(buf: BufferedReader<&'a mut Reader>) -> Decoder<'a> {
        Decoder {
            stack: vec!(),
            err: Ok(()),
            p: Parser {
                sep: ',',
                same_len: true,
                first_len: 0,
                has_headers: false,
                headers: vec!(),
                buf: buf,
                cur: Some(0u8 as char),
                look: None,
                line: 1,
                col: 0,
            },
        }
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

    /// Provides an iterator to decode one record at a time. Note that this
    /// usually needs to have its type parameter `D` instantiated explicitly
    /// or with a type hint. For example, both of the following are equivalent:
    ///
    /// ```no_run
    /// let mut dec = csv::Decoder::from_str("abc,1");
    /// let mut iter = dec.decode_iter::<(~str, uint)>();
    /// ```
    ///
    /// ```no_run
    /// let mut dec = csv::Decoder::from_str("abc,1");
    /// let mut iter: csv::DecodedItems<(~str, uint)> = dec.decode_iter();
    /// ```
    ///
    /// If there is an error decoding the data then `fail!` is called.
    pub fn decode_iter<D: Decodable<Decoder<'a>>>
                      (&'a mut self) -> DecodedItems<'a, D> {
        DecodedItems { dec: self }
    }

    /// Calls `decode` on every record in the CSV data until EOF and returns
    /// them as a vector. If there was an error decoding a vector, parsing is
    /// stopped and the error is returned.
    pub fn decode_all<D: Decodable<Decoder<'a>>>
                     (&mut self) -> Result<Vec<D>, Error> {
        let mut records: Vec<D> = vec!();
        loop {
            match self.decode() {
                Ok(r) => records.push(r),
                Err(err) => if err.eof { break } else { return Err(err) }
            }
        }
        Ok(records)
    }

    /// Circumvents the decoding interface and iterates over the records as
    /// vectors of strings. A record returned by this method will never be
    /// decoded.
    pub fn iter(&'a mut self) -> Records<'a> {
        Records { dec: self }
    }

    /// Circumvents the decoding interface and forces the parsing of the next
    /// record and returns it. A record returned by this method will never be
    /// decoded.
    pub fn record(&mut self) -> Result<Vec<~str>, Error> {
        self.p.parse_record(false)
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
            // Don't return an EOF error here.
            match self.p.parse_record(true) {
                Ok(_) => {}
                Err(err) => if !err.eof { return Err(err) }
            }
            assert!(self.p.headers.len() > 0);
        }
        Ok(self.p.headers.clone())
    }
}

/// An iterator that yields records as plain vectors of strings. This
/// completely avoids the decoding machinery.
pub struct Records<'a> {
    priv dec: &'a mut Decoder<'a>
}

impl<'a> Iterator<Vec<~str>> for Records<'a> {
    /// Iterates over each record in the CSV data. The iterator stops when
    /// EOF is reached.
    fn next(&mut self) -> Option<Vec<~str>> {
        match self.dec.record() {
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

/// An iterator that yields decoded items of type `D`.
pub struct DecodedItems<'a, D> {
    priv dec: &'a mut Decoder<'a>
}

impl<'a, D: Decodable<Decoder<'a>>> Iterator<D> for DecodedItems<'a, D> {
    fn next(&mut self) -> Option<D> {
        match self.dec.decode() {
            Ok(r) => Some(r),
            Err(err) => {
                if err.eof {
                    None
                } else {
                    fail!("Error decoding CSV data: {}", err)
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
        let r = try!(self.p.parse_record(false));
        self.push_record(r);
        Ok(())
    }

    fn pop_record(&mut self) -> Result<Vec<~str>, Error> {
        match try!(self.pop()) {
            Record(r) => Ok(r),
            String(s) => {
                let m = format!("Expected record but got value '{}'.", s);
                Err(self.err(m))
            }
        }
    }

    fn pop_string(&mut self) -> Result<~str, Error> {
        match try!(self.pop()) {
            Record(_) => {
                let m = format!("Expected value but got record.");
                Err(self.err(m))
            }
            String(s) => Ok(s),
        }
    }

    fn pop_from_str<T: FromStr + Default>(&mut self) -> Result<T, Error> {
        let s = try!(self.pop_string());
        let s = s.trim();
        match FromStr::from_str(s) {
            Some(t) => Ok(t),
            None => {
                let m = format!("Failed converting '{}' from str.", s);
                Err(self.err(m))
            }
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
            Some(idx) => return f(self, idx),
            None => {}
        }

        // At this point, we couldn't find a verbatim Enum variant, so let's
        // assume we're trying to load enum variants of one argument.
        // We don't know which one to pick, so we try each of them until we
        // get a hit.
        //
        // If we fail, it's tough to know what error to report. Probably the
        // right way to do this is to maintain a stack of errors. Ug.
        self.push_string(variant); // push what we popped earlier
        for i in range(0, names.len()) {
            // Copy the top of the stack now. We'll push it back on if
            // decoding into this variant fails.
            let cur = dectry!(self.pop_string(), f(self, 0));
            let copy = cur.clone();
            self.push_string(cur);

            let val = f(self, i);
            match self.err {
                Ok(_) => return val, // loaded a value successfully; bail!
                Err(_) => {
                    self.push_string(copy);
                    self.err = Ok(()); // march on...
                }
            }
        }
        self.err = Err(self.err(format!(
                       "Could not load value into any variant in {}", names)));
        f(self, 0)
    }
    fn read_enum_variant_arg<T>(&mut self, _: uint,
                                f: |&mut Decoder<'a>| -> T) -> T {
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
    fn read_tuple<T>(&mut self, f: |&mut Decoder<'a>, uint| -> T) -> T {
        // Using `f(self, self.p.first_len)` is a terrible hack that will
        // only work reliably on CSV files with records that all have the
        // same length.
        // The issue here is that we need to create some tuple, even if there
        // is an error preventing us from getting at data needed to build such
        // a tuple. Therefore, we fake it by assuming this record's length is
        // the same as every other record's length. Ug.
        let r = dectry!(self.pop_record(), f(self, self.p.first_len));
        let len = r.len();
        for v in r.move_iter().rev() {
            self.push_string(v)
        }
        f(self, len)
    }
    fn read_tuple_arg<T>(&mut self, _: uint, f: |&mut Decoder<'a>| -> T) -> T {
        f(self)
    }
    fn read_tuple_struct<T>(&mut self, _: &str,
                            _: |&mut Decoder<'a>, uint| -> T) -> T {
        unimplemented!()
    }
    fn read_tuple_struct_arg<T>(&mut self, _: uint,
                                _: |&mut Decoder<'a>| -> T) -> T {
        unimplemented!()
    }
    fn read_option<T>(&mut self, f: |&mut Decoder<'a>, bool| -> T) -> T {
        let s = dectry!(self.pop_string(), f(self, false));
        if s.len() == 0 {
            f(self, false)
        } else {
            self.push_string(s);
            let val = f(self, true);
            if self.err.is_err() {
                self.err = Ok(());
                return f(self, false)
            }
            val
        }
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
mod bench;

#[cfg(test)]
mod test;
