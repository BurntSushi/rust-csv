use std::cmp;
use std::io::{self, BufRead};

use bytecount;
use csv_core::{Reader as CoreReader, ReaderBuilder as CoreReaderBuilder};

use record::ByteRecord;
use {Error, Result, Terminator};

/// Builds a CSV reader with various configuration knobs.
///
/// This builder can be used to tweak the field delimiter, record terminator
/// and more for parsing CSV. Once a CSV `Reader` is built, its configuration
/// cannot be changed.
#[derive(Debug)]
pub struct ReaderBuilder {
    builder: CoreReaderBuilder,
    capacity: usize,
    flexible: bool,
}

impl Default for ReaderBuilder {
    fn default() -> ReaderBuilder {
        ReaderBuilder {
            builder: CoreReaderBuilder::default(),
            capacity: 8 * (1<<10),
            flexible: false,
        }
    }
}

impl ReaderBuilder {
    /// Create a new builder for configuring CSV parsing.
    ///
    /// To convert a builder into a reader, call one of the methods starting
    /// with `from_`.
    pub fn new() -> ReaderBuilder {
        ReaderBuilder::default()
    }

    /// Build a CSV parser from this configuration that reads data from `rdr.
    ///
    /// Note that the CSV reader is buffered automatically, so you should not
    /// wrap `rdr` in a buffered reader like `io::BufReader`.
    pub fn from_reader<R: io::Read>(&self, rdr: R) -> Reader<R> {
        Reader::new(self, rdr)
    }

    /// The field delimiter to use when parsing CSV.
    ///
    /// The default is `b','`.
    pub fn delimiter(&mut self, delimiter: u8) -> &mut ReaderBuilder {
        self.builder.delimiter(delimiter);
        self
    }

    /// Whether the number of fields in records is allowed to change or not.
    ///
    /// When disabled (which is the default), parsing CSV data will return an
    /// error if a record is found with a number of fields different from the
    /// number of fields in a previous record.
    ///
    /// When enabled, this error checking is turned off.
    pub fn flexible(&mut self, yes: bool) -> &mut ReaderBuilder {
        self.flexible = yes;
        self
    }

    /// The record terminator to use when parsing CSV.
    ///
    /// A record terminator can be any single byte. The default is a special
    /// value, `Terminator::CRLF`, which treats any occurrence of `\r`, `\n`
    /// or `\r\n` as a single record terminator.
    pub fn terminator(
        &mut self,
        term: Terminator,
    ) -> &mut ReaderBuilder {
        self.builder.terminator(term);
        self
    }

    /// The quote character to use when parsing CSV.
    ///
    /// The default is `b'"'`.
    pub fn quote(&mut self, quote: u8) -> &mut ReaderBuilder {
        self.builder.quote(quote);
        self
    }

    /// The escape character to use when parsing CSV.
    ///
    /// In some variants of CSV, quotes are escaped using a special escape
    /// character like `\` (instead of escaping quotes by doubling them).
    ///
    /// By default, recognizing these idiosyncratic escapes is disabled.
    pub fn escape(&mut self, escape: Option<u8>) -> &mut ReaderBuilder {
        self.builder.escape(escape);
        self
    }

    /// Enable double quote escapes.
    ///
    /// This is enabled by default, but it may be disabled. When disabled,
    /// doubled quotes are not interpreted as escapes.
    pub fn double_quote(&mut self, yes: bool) -> &mut ReaderBuilder {
        self.builder.double_quote(yes);
        self
    }

    /// A convenience method for specifying a configuration to read ASCII
    /// delimited text.
    ///
    /// This sets the delimiter and record terminator to the ASCII unit
    /// separator (`\x1F`) and record separator (`\x1E`), respectively.
    pub fn ascii(&mut self) -> &mut ReaderBuilder {
        self.builder.ascii();
        self
    }

    /// Set the capacity (in bytes) of the buffer used in the CSV reader.
    ///
    /// Note that if a custom buffer is given with the `buffer` method, then
    /// this setting has no effect.
    pub fn buffer_capacity(&mut self, capacity: usize) -> &mut ReaderBuilder {
        self.capacity = capacity;
        self
    }

    /// Enable or disable the NFA for parsing CSV.
    ///
    /// This is intended to be a debug option useful for debugging. The NFA
    /// is always slower than the DFA.
    #[doc(hidden)]
    pub fn nfa(&mut self, yes: bool) -> &mut ReaderBuilder {
        self.builder.nfa(yes);
        self
    }
}

#[derive(Debug)]
pub struct Reader<R> {
    core: CoreReader,
    rdr: io::BufReader<R>,
    state: ReaderState,
}

#[derive(Debug)]
struct ReaderState {
    flexible: bool,
    first_field_count: Option<u64>,
    prev_pos: Position,
    cur_pos: Position,
    eof: bool,
}

impl<R: io::Read> Reader<R> {
    /// Create a new CSV reader given a builder and a source of underlying
    /// bytes.
    fn new(builder: &ReaderBuilder, rdr: R) -> Reader<R> {
        Reader {
            core: builder.builder.build(),
            rdr: io::BufReader::with_capacity(builder.capacity, rdr),
            state: ReaderState {
                flexible: builder.flexible,
                first_field_count: None,
                prev_pos: Position::new(),
                cur_pos: Position::new(),
                eof: false,
            },
        }
    }

    /// Return the current position of this CSV reader.
    ///
    /// The byte offset in the position returned can be used to `seek` this
    /// reader. In particular, seeking to a position returned here on the same
    /// data will result in parsing the same subsequent record.
    pub fn position(&self) -> &Position {
        &self.state.cur_pos
    }

    pub fn read_record_bytes(
        &mut self,
        record: &mut ByteRecord,
    ) -> Result<bool> {
        use csv_core::ReadRecordResult::*;

        record.clear();
        if self.state.eof {
            return Ok(true);
        }
        let (mut outlen, mut endlen) = (0, 0);
        loop {
            let (res, nin, nout, nend) = {
                let input = self.rdr.fill_buf()?;
                let (mut fields, mut ends) = record.as_parts();
                self.core.read_record(
                    input, &mut fields[outlen..], &mut ends[endlen..])
            };
            self.rdr.consume(nin);
            self.state.cur_pos.byte += nin as u64;
            self.state.cur_pos.line = self.core.line();
            outlen += nout;
            endlen += nend;
            match res {
                InputEmpty => continue,
                OutputFull => {
                    record.expand_fields();
                    continue;
                }
                OutputEndsFull => {
                    record.expand_ends();
                    continue;
                }
                Record => {
                    record.set_len(endlen);
                    self.state.add_record(endlen as u64)?;
                    break;
                }
                End => {
                    self.state.eof = true;
                    break;
                }
            }
        }
        Ok(self.state.eof)
    }
}

impl ReaderState {
    #[inline(always)]
    fn add_record(&mut self, num_fields: u64) -> Result<()> {
        self.cur_pos.record = self.cur_pos.record.checked_add(1).unwrap();
        if !self.flexible {
            match self.first_field_count {
                None => self.first_field_count = Some(num_fields),
                Some(expected) => {
                    if num_fields != expected {
                        return Err(Error::UnequalLengths {
                            expected_len: expected,
                            pos: self.prev_pos.clone(),
                            len: num_fields,
                        });
                    }
                }
            }
        }
        self.prev_pos = self.cur_pos.clone();
        Ok(())
    }
}

/// A position in CSV data.
///
/// A position is used to report errors in CSV data. All positions include the
/// byte offset, line number and record index at which the error occurred.
#[derive(Clone, Debug)]
pub struct Position {
    byte: u64,
    line: u64,
    record: u64,
}

impl Position {
    /// Returns a new position initialized to the start value.
    fn new() -> Position { Position { byte: 0, line: 1, record: 0 } }
    /// The byte offset, starting at `0`, of this position.
    pub fn byte(&self) -> u64 { self.byte }
    /// The line number, starting at `1`, of this position.
    pub fn line(&self) -> u64 { self.line }
    /// The record index, starting at `0`, of this position.
    pub fn record(&self) -> u64 { self.record }
}

#[cfg(test)]
mod tests {
    use {ByteRecord, Error, ReaderBuilder};
    use super::Position;

    fn b(s: &str) -> &[u8] { s.as_bytes() }
    fn s(b: &[u8]) -> &str { ::std::str::from_utf8(b).unwrap() }

    macro_rules! assert_match {
        ($e:expr, $p:pat) => {{
            match $e {
                $p => {}
                e => panic!("match failed, got {:?}", e),
            }
        }}
    }

    #[test]
    fn read_record_bytes() {
        let data = b("foo,\"b,ar\",baz\nabc,mno,xyz");
        let mut rdr = ReaderBuilder::new().from_reader(data);
        // let mut rec = ByteRecord::new();
        let mut rec = ByteRecord::with_capacity(1);

        assert!(!rdr.read_record_bytes(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("foo", s(&rec[0]));
        assert_eq!("b,ar", s(&rec[1]));
        assert_eq!("baz", s(&rec[2]));

        assert!(!rdr.read_record_bytes(&mut rec).unwrap());
        assert_eq!(3, rec.len());
        assert_eq!("abc", s(&rec[0]));
        assert_eq!("mno", s(&rec[1]));
        assert_eq!("xyz", s(&rec[2]));

        assert!(rdr.read_record_bytes(&mut rec).unwrap());
    }

    #[test]
    fn read_record_unequal_fails() {
        let data = b("foo\nbar,baz");
        let mut rdr = ReaderBuilder::new().from_reader(data);
        let mut rec = ByteRecord::new();

        assert!(!rdr.read_record_bytes(&mut rec).unwrap());
        assert_eq!(1, rec.len());
        assert_eq!("foo", s(&rec[0]));

        assert_match!(
            rdr.read_record_bytes(&mut rec),
            Err(Error::UnequalLengths {
                expected_len: 1,
                pos: Position { byte: 4, line: 2, record: 1},
                len: 2,
            }));
    }

    #[test]
    fn read_record_unequal_ok() {
        let data = b("foo\nbar,baz");
        let mut rdr = ReaderBuilder::new().flexible(true).from_reader(data);
        let mut rec = ByteRecord::new();

        assert!(!rdr.read_record_bytes(&mut rec).unwrap());
        assert_eq!(1, rec.len());
        assert_eq!("foo", s(&rec[0]));

        assert!(!rdr.read_record_bytes(&mut rec).unwrap());
        assert_eq!(2, rec.len());
        assert_eq!("bar", s(&rec[0]));
        assert_eq!("baz", s(&rec[1]));

        assert!(rdr.read_record_bytes(&mut rec).unwrap());
    }

    // This tests that even if we get a CSV error, we can continue reading
    // if we want.
    #[test]
    fn read_record_unequal_continue() {
        let data = b("foo\nbar,baz\nquux");
        let mut rdr = ReaderBuilder::new().from_reader(data);
        let mut rec = ByteRecord::new();

        assert!(!rdr.read_record_bytes(&mut rec).unwrap());
        assert_eq!(1, rec.len());
        assert_eq!("foo", s(&rec[0]));

        assert_match!(
            rdr.read_record_bytes(&mut rec),
            Err(Error::UnequalLengths {
                expected_len: 1,
                pos: Position { byte: 4, line: 2, record: 1},
                len: 2,
            }));

        assert!(!rdr.read_record_bytes(&mut rec).unwrap());
        assert_eq!(1, rec.len());
        assert_eq!("quux", s(&rec[0]));

        assert!(rdr.read_record_bytes(&mut rec).unwrap());
    }
}
