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

extern crate rand;
extern crate serialize;
extern crate "test" as stdtest;

#[cfg(test)]
extern crate quickcheck;

use std::fmt;
use std::hash;
use std::io;

pub use bytestr::ByteString;
pub use encoder::{Encoder};
pub use decoder::{Decoder, Records};

mod encoder;
mod decoder;

#[cfg(test)]
mod bench;
#[cfg(test)]
mod test;

static QUOTE: u8 = b'"';
static ESCAPE: u8 = b'\\';

type CsvResult<T> = Result<T, Error>;

#[deriving(Clone)]
enum Error {
    ErrEncode(String),
    ErrDecode(String),
    ErrParse(ParseError),
    ErrIo(io::IoError),
}

impl Error {
    fn is_eof(&self) -> bool {
        match self {
            &ErrIo(io::IoError { kind: io::EndOfFile, .. }) => true,
            _ => false,
        }
    }
}

#[deriving(Clone)]
struct ParseError {
    line: uint,
    column: uint,
    kind: ParseErrorKind,
}

#[deriving(Clone)]
enum ParseErrorKind {
    /// This error occurs when a record has a different number of fields
    /// than the first record parsed.
    UnequalLengths(uint, uint),

    /// This error occurs when parsing CSV data as Unicode.
    InvalidUTF8,

    /// This error occurs when an EOF is reached before a closing quote
    /// in a quoted field.
    UnexpectedEof,

    /// This error occurs when a character other than whitespace appears
    /// between an ending quote character and the next field/record delimiter.
    UnexpectedCharAfterQuote(u8),
}

impl fmt::Show for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ErrEncode(ref msg) => write!(f, "CSV encode error: {}", msg),
            &ErrDecode(ref msg) => write!(f, "CSV decode error: {}", msg),
            &ErrParse(ref err) => write!(f, "{}", err),
            &ErrIo(ref err) => write!(f, "{}", err),
        }
    }
}

impl fmt::Show for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CSV parse error:{:u}:{:u}: {}",
               self.line, self.column, self.kind)
    }
}

impl fmt::Show for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &UnequalLengths(first, cur) =>
                write!(f, "First record has length {:u}, but found record \
                           with length {:u}.", first, cur),
            &InvalidUTF8 =>
                write!(f, "Invalid UTF8 encoding."),
            &UnexpectedEof =>
                write!(f, "Expected end quote but found EOF instead."),
            &UnexpectedCharAfterQuote(b) =>
                write!(f, "Expected EOF, line terminator, separator or \
                           whitespace following quoted value, but found \
                           '{}' (\\x{:x}) instead.", b as char, b),
        }
    }
}

#[deriving(Eq, PartialEq, Show)]
enum ParseState {
    StartRecord,
    EndRecord,
    StartField,
    EatCRLF,
    InField,
    InQuotedField,
    InQuotedFieldEscape,
    InQuotedFieldQuote,
}

enum Parsed<'a> {
    ParsedRecord(uint, u64),
    ParsedField(&'a [u8]),
}

impl<'a> fmt::Show for Parsed<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ParsedRecord(line, byte) => {
                write!(f, "Record({}, {})", line, byte)
            }
            &ParsedField(bytes) => {
                try!(write!(f, "Field("));
                try!(f.write(bytes));
                write!(f, ")")
            }
        }
    }
}

struct Parser<R> {
    delimiter: u8,
    flexible: bool,
    buffer: io::BufferedReader<R>,
    fieldbuf: Vec<u8>,
    state: ParseState,
    err: Option<Error>,

    // Keep a copy of the first record parsed.
    // It's easy to do it at the base parser level, even if the user says
    // to ignore headers.
    first_record: Vec<Vec<u8>>,
    first_record_parsed: bool,
    has_headers: bool,

    // Various book-keeping counts.
    field_count: uint, // number of fields in current record
    column: uint, // current column (by byte, *shrug*)
    line_record: uint, // line at which current record started
    line_current: uint, // current line
    byte_offset: u64, // current byte offset
}

impl<R: Reader> Parser<R> {
    pub fn new(rdr: R, delimiter: u8, flexible: bool, has_headers: bool) -> Parser<R> {
        Parser {
            delimiter: delimiter,
            flexible: flexible,
            buffer: io::BufferedReader::new(rdr),
            fieldbuf: Vec::with_capacity(1024),
            state: StartRecord,
            err: None,
            first_record: vec!(),
            first_record_parsed: false,
            has_headers: has_headers,
            field_count: 0,
            column: 1,
            line_record: 1,
            line_current: 1,
            byte_offset: 0,
        }
    }

    #[inline]
    fn clear_field_buffer(&mut self) {
        unsafe { self.fieldbuf.set_len(0); }
    }

    #[inline]
    fn fieldbuf_as_slice(&self) -> &[u8] {
        self.fieldbuf.as_slice()
    }

    fn done(&self) -> bool {
        self.err.is_some()
    }

    fn parse_err(&self, kind: ParseErrorKind) -> Error {
        ErrParse(ParseError {
            line: self.line_record,
            column: self.column,
            kind: kind,
        })
    }

    fn byte_records<'a>(&'a mut self) -> ByteRecords<'a, R> {
        ByteRecords { p: self }
    }

    fn records<'a>(&'a mut self) -> Records<'a, R> {
        Records { p: self, errored: false }
    }
}

struct ByteRecords<'a, R: 'a> {
    p: &'a mut Parser<R>,
}

type ByteString = Vec<u8>;

impl<'a, R: Reader> Iterator<CsvResult<Vec<ByteString>>> for ByteRecords<'a, R> {
    fn next(&mut self) -> Option<CsvResult<Vec<ByteString>>> {
        if self.p.done() {
            return None;
        }
        let skip = self.p.has_headers && !self.p.first_record_parsed;
        let mut record = Vec::with_capacity(self.p.first_record.len());
        for field in self.p {
            match field {
                Err(err) => return Some(Err(err)),
                Ok(bytes) => record.push(Vec::from_slice(bytes)),
            }
        }
        if skip {
            self.next() // O_O
        } else {
            Some(Ok(record))
        }
    }
}

struct Records<'a, R: 'a> {
    p: &'a mut Parser<R>,
    errored: bool,
}

impl<'a, R: Reader> Iterator<CsvResult<Vec<String>>> for Records<'a, R> {
    fn next(&mut self) -> Option<CsvResult<Vec<String>>> {
        if self.errored || self.p.done() {
            return None;
        }
        let skip = self.p.has_headers && !self.p.first_record_parsed;
        let invalid_utf8 = Some(Err(self.p.parse_err(InvalidUTF8)));
        let mut record = Vec::with_capacity(self.p.first_record.len());
        for field in self.p {
            match field {
                Err(err) => return Some(Err(err)),
                Ok(bytes) => {
                    let bvec = bytes.to_vec();
                    let s = match String::from_utf8(bvec) {
                        Ok(s) => s,
                        Err(_) => return invalid_utf8,
                    };
                    record.push(s);
                }
            }
        }
        if skip {
            self.next() // O_O
        } else {
            Some(Ok(record))
        }
    }
}

impl<'a, R: Reader> Iterator<CsvResult<&'a [u8]>> for Parser<R> {
    fn next(&mut self) -> Option<CsvResult<&'a [u8]>> {
        // Every call to this method should produce a Record terminator
        // of a slice of bytes corresponding to a single field. Therefore,
        // we always clear the current field buffer.
        self.clear_field_buffer();

        // The EndRecord state indicates what you'd expect: we emit a
        // Record terminator, check for same-length records and reset a little
        // record-based book keeping.
        //
        // We need to run this check before looking for errors so that we
        // emit a Record terminator before quitting parsing from an EOF.
        if self.state == EndRecord {
            if !self.flexible && self.first_record.len() != self.field_count {
                let err = self.parse_err(UnequalLengths(self.first_record.len(),
                                                        self.field_count));
                self.err = Some(err.clone());
                return Some(Err(err));
            }
            // After processing an EndRecord (and determined there are no
            // errors), we should always start parsing the next record.
            self.state = StartRecord;
            self.line_record = self.line_current;
            self.first_record_parsed = true;
            self.field_count = 0;
            return None;
        }
        
        // Check to see if we've recorded an error and quit parsing if we have.
        // This serves two purposes:
        // 1) When CSV parsing reaches an error, it is unrecoverable. So the
        //    parse function will always return the same error.
        // 2) EOF errors are handled specially and can be returned "lazily".
        //    e.g., EOF in the middle of parsing a field. First we have to
        //    return the field, then return a record terminator and *then*
        //    the EOF error.
        match self.err {
            None => {},
            Some(ref err) => return None,
        }

        // A parser machine encapsulates the main parsing state transitions.
        // Normally, the state machine would be written as methods on the
        // Parser type, but mutable borrows become troublesome. So we isolate
        // the things we need to mutate during state transitions with
        // the ParserMachine type.
        let mut pmachine = ParserMachine {
            fieldbuf: unsafe { transmute(&mut self.fieldbuf) },
            state: unsafe { transmute(&mut self.state) },
            delimiter: self.delimiter,
        };
        let mut consumed = 0; // tells the buffer how much we consumed
        'TOPLOOP: loop {
            // The following code is basically, "fill a buffer with data from
            // the underlying reader if it's empty, and then run the parser
            // over each byte in the slice returned."
            //
            // This technique is critical for performance, because it lifts
            // a lot of case analysis off of each byte. (i.e., This loop could
            // be more simply written with `buf.read_byte()`, but it is much
            // slower.)
            match self.buffer.fill_buf() {
                Err(err) => {
                    // The error is processed below.
                    // We don't handle it here because we need to do some
                    // book keeping first.
                    self.err = Some(ErrIo(err));
                    break 'TOPLOOP;
                }
                Ok(bs) => {
                    // This "batch" processing of bytes is critical for
                    // performance.
                    for &b in bs.iter() {
                        pmachine.parse_byte(b);
                        let s = *pmachine.state;
                        if s == EndRecord {
                            // Don't consume the byte we just read, because
                            // it is the first byte of the next record.
                            break 'TOPLOOP;
                        } else {
                            consumed += 1;
                            self.column += 1;
                            self.byte_offset += 1;
                            if is_crlf(b) {
                                if b == b'\n' {
                                    self.line_current += 1;
                                }
                                self.column = 1;
                            }
                            if s == StartField {
                                break 'TOPLOOP
                            }
                        }
                    }
                }
            }
            self.buffer.consume(consumed);
            consumed = 0;
        }
        // We get here when we break out of the loop, so make sure the buffer
        // knows how much we read.
        self.buffer.consume(consumed);

        // Handle the error. EOF is a bit tricky, but otherwise, we just stop
        // the parser cold.
        match self.err {
            None => {}
            Some(ref err @ ErrIo(io::IoError { kind: io::EndOfFile, .. })) => {
                // If we get EOF while we're trying to parse a new record
                // but haven't actually seen any fields yet (i.e., trailing
                // new lines in a file), then we should immediately stop the
                // parser.
                if *pmachine.state == StartRecord {
                    return None;
                }
                *pmachine.state = EndRecord;
                // fallthrough to return current field.
                // This happens when we get an EOF in the middle of parsing
                // a field. We set the state to end the record so that the
                // next call to `parse` will produce a Record terminator.
                // *Then* a subsequent call to `parse` will produce an EOF
                // error.
            }
            Some(ref err) => {
                // Reset the state to the beginning so that bad errors
                // are always reported. (i.e., Don't let an EndRecord state
                // slip in here.)
                *pmachine.state = StartRecord;
                return Some(Err(err.clone()));
            }
        }
        if !self.first_record_parsed {
            // This is only copying bytes for the first record.
            self.first_record.push(pmachine.fieldbuf.clone());
        }
        self.field_count += 1;
        Some(Ok(pmachine.fieldbuf.as_slice()))
    }
}

struct ParserMachine<'a> {
    fieldbuf: &'a mut Vec<u8>,
    state: &'a mut ParseState,
    delimiter: u8,
}

impl<'a> ParserMachine<'a> {
    #[inline]
    fn parse_byte(&mut self, b: u8) {
        match *self.state {
            StartRecord => self.parse_start_record(b),
            EndRecord => unreachable!(),
            StartField => self.parse_start_field(b),
            EatCRLF => self.parse_eat_crlf(b),
            InField => self.parse_in_field(b),
            InQuotedField => self.parse_in_quoted_field(b),
            InQuotedFieldEscape => self.parse_in_quoted_field_escape(b),
            InQuotedFieldQuote => self.parse_in_quoted_field_quote(b),
        }
    }

    #[inline]
    fn parse_start_record(&mut self, b: u8) {
        if !is_crlf(b) {
            *self.state = StartField;
            self.parse_start_field(b);
        }
    }

    #[inline]
    fn parse_start_field(&mut self, b: u8) {
        if is_crlf(b) {
            *self.state = EatCRLF;
        } else if b == QUOTE {
            *self.state = InQuotedField;
        } else if b == self.delimiter {
            // empty field, so return in StartField state,
            // which causes a new empty field to be returned
        } else {
            *self.state = InField;
            self.fieldbuf.push(b);
        }
    }

    #[inline]
    fn parse_eat_crlf(&mut self, b: u8) {
        if !is_crlf(b) {
            *self.state = EndRecord;
        }
    }

    #[inline]
    fn parse_in_field(&mut self, b: u8) {
        if is_crlf(b) {
            *self.state = EatCRLF;
        } else if b == self.delimiter {
            *self.state = StartField;
        } else {
            self.fieldbuf.push(b);
        }
    }

    #[inline]
    fn parse_in_quoted_field(&mut self, b: u8) {
        if b == ESCAPE {
            *self.state = InQuotedFieldEscape;
        } else if b == QUOTE {
            *self.state = InQuotedFieldQuote;
        } else {
            self.fieldbuf.push(b);
        }
    }

    #[inline]
    fn parse_in_quoted_field_escape(&mut self, b: u8) {
        *self.state = InQuotedField;
        self.fieldbuf.push(b);
    }

    #[inline]
    fn parse_in_quoted_field_quote(&mut self, b: u8) {
        if b == QUOTE {
            *self.state = InQuotedField;
            self.fieldbuf.push(b);
        } else if b == self.delimiter {
            *self.state = StartField;
        } else if is_crlf(b) {
            *self.state = EatCRLF;
        } else {
            // Should we provide a strict variant that disallows
            // random chars after a quote?
            *self.state = InField;
            self.fieldbuf.push(b);
        }
    }
}

#[inline]
fn is_crlf(b: u8) -> bool { b == b'\n' || b == b'\r' }
