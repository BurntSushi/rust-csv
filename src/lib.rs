#![crate_name = "csv"]
#![crate_type = "rlib"]
#![crate_type = "dylib"]
#![license = "UNLICENSE"]
#![doc(html_root_url = "http://burntsushi.net/rustdoc/csv")]

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
//! let mut rdr = csv::Decoder::from_file(&Path::new("foo.csv")).no_headers();
//! for record in rdr.iter() {
//!     println!("{}", record);
//! }
//! ```
//!
//!
//! ## Headers and delimiters
//!
//! By default, the decoder in this crate assumes that the CSV data contains
//! a header record. The header record will be omitted from standard record
//! traversal, but we can access the header record at any time with the
//! `headers` method:
//!
//! ```
//! let mut rdr = csv::Decoder::from_str("abc,xyz\n1,2");
//!
//! assert_eq!(rdr.headers().unwrap(), vec!("abc".to_string(), "xyz".to_string()));
//! assert_eq!(rdr.iter().next().unwrap().unwrap(),
//!            vec!("1".to_string(), "2".to_string()));
//! assert_eq!(rdr.headers().unwrap(), vec!("abc".to_string(), "xyz".to_string()));
//! ```
//!
//! The decoder also assumes that a comma (`,`) is the delimiter used to
//! separate values in a record. This can be changed with the `separator`
//! method. For example, to read tab separated values:
//!
//! ```
//! let mut rdr = csv::Decoder::from_str("a\tb\ny\tz")
//!                            .no_headers()
//!                            .separator(b'\t');
//!
//! assert_eq!(rdr.iter().next().unwrap().unwrap(),
//!            vec!("a".to_string(), "b".to_string()));
//! assert_eq!(rdr.iter().next().unwrap().unwrap(),
//!            vec!("y".to_string(), "z".to_string()));
//! ```
//!
//! Note that the delimiter is using a byte character literal: `b'\t'`. This is
//! because CSV parsing makes no assumptions about the encoding of the data
//! being read (other than the use of a few standard ASCII characters like the
//! field/record separators and `"`).
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
//! let mut rdr = csv::Decoder::from_str("andrew,1987\nkait,1989").no_headers();
//! let record: (String, uint) = rdr.decode().unwrap();
//! assert_eq!(record, ("andrew".to_string(), 1987));
//! ```
//!
//! An iterator is provided to repeat this for all records in the CSV data:
//!
//! ```rust
//! let mut rdr = csv::Decoder::from_str("andrew,1987\nkait,1989").no_headers();
//! for record in rdr.iter_decode::<(String, uint)>() {
//!     let (name, birth) = record.unwrap();
//!     println!("Name: {}, Born: {}", name, birth);
//! }
//! ```
//!
//! Note that `iter_decode` is *explicitly* instantiated with the type
//! of the record.
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
//! let mut rdr = csv::Decoder::from_str("andrew, \"\"\nkait,").no_headers();
//! let record1: (String, Option<uint>) = rdr.decode().unwrap();
//! let record2: (String, Option<uint>) = rdr.decode().unwrap();
//!
//! assert_eq!(record1, ("andrew".to_string(), None));
//! assert_eq!(record2, ("kait".to_string(), None));
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
//! #[deriving(PartialEq, Show, Decodable)]
//! enum Truthy {
//!     Uint(uint),
//!     Bool(bool),
//! }
//!
//! fn main() {
//!     let mut rdr = csv::Decoder::from_str("andrew,false\nkait,1").no_headers();
//!     let record: (String, Truthy) = rdr.decode().unwrap();
//!     assert_eq!(record, ("andrew".to_string(), Bool(false)));
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
//! let mut enc = csv::Encoder::mem_encoder();
//! enc.encode(("andrew", 1987u)).unwrap();
//! enc.encode(("kait", 1989u)).unwrap();
//! assert_eq!(enc.to_string(), "andrew,1987\nkait,1989\n");
//! ```
//!
//! Note that `Encoder::mem_encoder` creates a convenience encoder for
//! strings. You can encode to any `Writer` (with `to_writer`) or to a file:
//!
//! ```no_run
//! let mut enc = csv::Encoder::to_file(&Path::new("foo.csv"));
//! let records = vec!(("andrew", 1987u), ("kait", 1989u));
//! match enc.encode_all(records.iter()) {
//!     Ok(_) => {},
//!     Err(err) => fail!("Error encoding: {}", err),
//! }
//! ```
//!
//! The encoder in this crate supports all of the same things as the decoder,
//! including writing enum and option types. The encoder will make sure that
//! values containing special characters (like quotes, new lines or the
//! delimiter) are appropriately quoted. Quoting only occurs when it is
//! necessary.
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
//! use std::time::Duration;
//!
//! use csv::{Decoder, Encoder};
//!
//! fn main() {
//!     let (send, recv) = channel();
//!     spawn(proc() {
//!         let mut w = ChanWriter::new(send);
//!         let mut enc = Encoder::to_writer(&mut w as &mut Writer);
//!         for x in range(1u, 6) {
//!             match enc.encode((x, x * x)) {
//!                 Ok(_) => {},
//!                 Err(err) => fail!("Failed encoding: {}", err),
//!             }
//!             sleep(Duration::seconds(1));
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
//!   * The first record is read as a "header" by default, but this can be
//!     disabled by calling `no_headers` before decoding any records.
//!     (N.B. The encoder has no explicit support for headers. Simply encode a
//!     vector of strings instead.)
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
//!   * Currently, this crate biases toward UTF-8 support. However, both
//!     the `Decoder` and the `Encoder` expose methods for dealing with raw
//!     encoding agnostic byte strings. The only restriction is that the field
//!     delimiter must be a single 8-bit encoded character, quotations must
//!     be the ASCII `"` and record separators must be the ASCII `\n` or `\r\n`
//!     characters.
//!
//! Everything else should be supported, including new lines in quoted values.

#![feature(default_type_params, phase)]

#[phase(plugin, link)] extern crate log;
extern crate rand;
extern crate serialize;
extern crate stdtest = "test";

#[cfg(test)]
extern crate quickcheck;

use std::fmt;
use std::hash;
use std::io;

pub use encoder::{Encoder};
pub use decoder::{Decoder, Records};

mod encoder;
mod decoder;

#[cfg(test)]
mod bench;
#[cfg(test)]
mod test;

/// A type that represents unadulterated byte strings.
///
/// Byte strings represent *any* 8 bit character encoding. There are no
/// restrictions placed on the type of encoding used. (This means that there
/// may be *multiple* encodings in any particular byte string!)
///
/// Many CSV files in the wild aren't just malformed with respect to RFC 4180,
/// but they are commonly *not* UTF-8 encoded. Even worse, some of them are
/// encoded improperly. Therefore, any useful CSV parser must be flexible with
/// respect to encodings.
///
/// Thus, this CSV parser uses byte strings internally. This means that
/// quotes and field and record separators *must* be ASCII. Otherwise,
/// the parser places no other restrictions on the content of data in each
/// cell.
///
/// Note that most of the methods in the encoder/decoder will assume UTF-8
/// encoding, but they also expose some lower level methods that use byte
/// strings when absolutely necessary. This type is exposed in case you need
/// to deal with the raw bytes directly.
#[deriving(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteString(Vec<u8>);

impl ByteString {
    /// Create a new byte string from a vector or slice of bytes.
    pub fn from_bytes<S: CloneableVector<u8>>(bs: S) -> ByteString {
        ByteString(bs.into_vec())
    }

    /// Consumes this byte string into a vector of bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        let ByteString(chars) = self;
        chars
    }

    /// Returns this byte string as a slice of bytes.
    pub fn as_bytes<'a>(&'a self) -> &'a [u8] {
        let &ByteString(ref chars) = self;
        chars.as_slice()
    }

    /// Consumes the byte string and decodes it into a Unicode string. If the
    /// decoding fails, then the original ByteString is returned.
    pub fn to_utf8_string(self) -> Result<String, ByteString> {
        String::from_utf8(self.into_bytes()).map_err(ByteString)
    }
}

impl fmt::Show for ByteString {
    /// Writes the raw bytes to `f`. (There is no conversion to UTF-8
    /// encoding.)
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ByteString(ref chars) = *self;
        // XXX: Ideally, we could just do this:
        //
        //    f.write(chars.as_slice())
        //
        // and let the output device figure out how to render it. But it seems
        // the formatting infrastructure assumes that the data is UTF-8
        // encodable, which obviously doesn't work with raw byte strings.
        //
        // For now, we just show the bytes, e.g., `[255, 50, 48, 49, ...]`.
        write!(f, "{}", chars.as_slice())
    }
}

impl Slice<u8> for ByteString {
    fn as_slice<'a>(&'a self) -> &'a [u8] {
        let ByteString(ref chars) = *self;
        chars.as_slice()
    }
}

impl<H: hash::Writer> hash::Hash<H> for ByteString {
    fn hash(&self, hasher: &mut H) {
        self.as_slice().hash(hasher);
    }
}

impl<S: Str> Equiv<S> for ByteString {
    fn equiv(&self, other: &S) -> bool {
        self.as_bytes() == other.as_slice().as_bytes()
    }
}

/// A type that encapsulates any type of error that can be generated by
/// reading and writing CSV data.
#[deriving(Clone)]
pub enum Error {
    /// An error caused by invalid encoding. (Such as trying to write records
    /// of varying length when `enforce_same_length` is enabled.)
    ErrEncode(String),

    /// An error caused by parsing or decoding a single CSV record.
    ErrRecord(RecordError),

    /// An IO error that occurred when reading or writing CSV data.
    ErrIo(io::IoError),

    /// EOF is reached. This is only exposed through the `record`,
    /// `record_bytes` and `decode` methods in the decoder.
    ErrEOF,
}

/// An error caused by parsing or decoding a single record.
#[deriving(Clone)]
pub struct RecordError {
    /// The line on which the error occurred.
    pub line: uint,

    /// The column where the error occurred.
    pub col: uint,

    /// A message describing the error.
    pub msg: String,
}

impl Error {
    fn record<S: StrAllocating>(line: uint, col: uint, msg: S) -> Error {
        ErrRecord(RecordError {
            line: line,
            col: col,
            msg: msg.into_string(),
        })
    }
    fn eof() -> Error { ErrEOF }
    fn io(e: io::IoError) -> Error { ErrIo(e) }

    /// Returns true if this error is an EOF error.
    pub fn is_eof(&self) -> bool {
        match self { &ErrEOF => true, _ => false }
    }
}

impl fmt::Show for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ErrEncode(ref msg) => write!(f, "Encode error: {}", msg),
            &ErrRecord(ref re) => write!(f, "{}", re),
            &ErrIo(ref ie) => {
                try!(write!(f, "IO error ({}): {}", ie.kind, ie.desc));
                match ie.detail {
                    None => {},
                    Some(ref det) => try!(write!(f, " (detail: {})", det)),
                }
                Ok(())
            }
            &ErrEOF => write!(f, "EOF"),
        }
    }
}

impl fmt::Show for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Parse error:{}:{}: {}", self.line, self.col, self.msg)
    }
}

/// The parser state used in the decoder.
struct Parser<R> {
    buf: io::BufferedReader<R>, // buffer to read CSV data from
    sep: u8, // separator character to use
    same_len: bool, // whether to enforce all rows be of same length
    first_len: uint, // length of first row
    no_headers: bool, // interpret first record as headers when true
    headers: Vec<ByteString>, // the first record in the CSV data
    cur: Option<u8>, // the current character
    look: Option<u8>, // one character look-ahead
    line: uint, // current line
    col: uint, // current column
    byte: u64, // current byte offset
    byte_record_start: u64, // byte offset of previous read record
}

impl<R: Reader> Parser<R> {
    fn parse_record(&mut self, as_header: bool)
                   -> Result<Vec<ByteString>, Error> {
        self.line += 1;
        try!(self.eat_lineterms());
        if try!(self.peek()).is_none() {
            return Err(Error::eof())
        }

        self.byte_record_start = self.byte;
        let mut vals = Vec::with_capacity(self.first_len);
        while !self.is_eof() {
            let val = try!(self.parse_value());
            vals.push(val);
            if self.is_lineterm() {
                try!(self.eat_lineterm());
                break
            }
        }
        if self.is_eof() && vals.len() == 0 {
            return Err(Error::eof())
        }

        // This is a bit hokey, but if an error is generated at this point
        // (like a decoding error), then the line number will be off by one.
        // We correct this here, but we have to offset this correction the
        // next time `parse_record` is called.
        self.line -= 1;

        if self.same_len {
            if self.first_len == 0 {
                self.first_len = vals.len()
            } else if self.first_len != vals.len() {
                return Err(self.err(format!(
                    "Record has length {} but other records have length {}",
                    vals.len(), self.first_len).as_slice()))
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
        if !self.no_headers && self.headers.len() == 0 {
            self.headers = vals;
            if as_header {
                return Ok(self.headers.clone())
            }
            return self.parse_record(false)
        }
        Ok(vals)
    }

    fn parse_value(&mut self) -> Result<ByteString, Error> {
        let mut only_whitespace = true;
        let mut res = Vec::with_capacity(4);
        loop {
            try!(self.next_byte());
            if self.is_end_of_val() {
                break
            } else if only_whitespace {
                if self.cur_is(b'"') {
                    // Throw away any leading whitespace.
                    return self.parse_quoted_value()
                } else if self.is_blank() {
                    res.push(self.cur.unwrap());
                    continue
                }
            }
            only_whitespace = false;
            res.push(self.cur.unwrap());
        }
        Ok(ByteString(res))
    }

    fn parse_quoted_value(&mut self) -> Result<ByteString, Error> {
        // Assumes that " has already been read.
        let mut res = Vec::with_capacity(4);
        loop {
            try!(self.next_byte());
            if self.is_eof() {
                return Err(self.err("EOF while parsing quoted value."))
            } else if self.cur_is(b'"') {
                if self.peek_is(b'"') {
                    try!(self.next_byte()); // throw away second "
                    res.push(b'"');
                    continue
                }

                // Eat and spit out everything up to next separator.
                // If we see something that isn't whitespace, it's an error.
                try!(self.next_byte());
                loop {
                    if self.is_end_of_val() {
                        break
                    } else if !self.is_blank() {
                        let msg = format!(
                            "Expected EOF, line terminator, separator or \
                            whitespace following quoted value but found \
                            '{}' instead.", self.cur.unwrap());
                        return Err(self.err(msg.as_slice()));
                    }
                    try!(self.next_byte());
                }
                break
            } else if self.cur_is(b'\\') && self.peek_is(b'"') {
                // We also try to support \ escaped quotes even though
                // the spec says "" is used.
                try!(self.next_byte()); // throw away the "
                res.push(b'"');
                continue
            }
            res.push(self.cur.unwrap());
        }
        Ok(ByteString(res))
    }

    fn next_byte(&mut self) -> Result<(), Error> {
        self.cur = try!(self.read_next_byte());
        if !self.is_eof() {
            self.byte += 1;
            if self.cur_is(b'\n') {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        Ok(())
    }

    fn read_next_byte(&mut self) -> Result<Option<u8>, Error> {
        match self.look {
            Some(c) => { self.look = None; Ok(Some(c)) }
            None => match self.buf.read_byte() {
                Ok(c) => Ok(Some(c)),
                Err(io::IoError { kind: io::EndOfFile, .. }) => Ok(None),
                Err(err) => Err(Error::io(err)),
            }
        }
    }

    fn peek(&mut self) -> Result<Option<u8>, Error> {
        match self.look {
            Some(c) => Ok(Some(c)),
            None => {
                self.look = try!(self.read_next_byte());
                Ok(self.look)
            }
        }
    }

    fn cur_is(&self, c: u8) -> bool {
        self.cur == Some(c)
    }

    fn peek_is(&mut self, c: u8) -> bool {
        match self.peek() {
            Ok(Some(p)) => p == c,
            _ => false,
        }
    }

    fn is_end_of_val(&mut self) -> bool {
        self.cur_is(self.sep) || self.is_lineterm() || self.is_eof()
    }

    fn is_eof(&self) -> bool {
        self.cur.is_none()
    }

    fn is_blank(&self) -> bool {
        return self.cur == Some(b' ') || self.cur == Some(b'\t')
    }

    fn is_lineterm(&mut self) -> bool {
        if self.cur_is(b'\n') {
            return true
        }
        if self.cur_is(b'\r') {
            return self.peek_is(b'\n')
        }
        false
    }

    fn eat_lineterms(&mut self) -> Result<(), Error> {
        while self.peek_is(b'\n') || self.peek_is(b'\r') {
            try!(self.next_byte()); // read a '\r' or a '\n'
            try!(self.eat_lineterm()); // read a '\n' if read '\r'
        }
        Ok(())
    }

    fn eat_lineterm(&mut self) -> Result<(), Error> {
        if self.cur_is(b'\r') {
            try!(self.next_byte());
        }
        Ok(())
    }

    fn err<S: StrAllocating>(&self, msg: S) -> Error {
        Error::record(self.line, self.col, msg)
    }
}
