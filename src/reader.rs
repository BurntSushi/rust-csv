use std::cmp;
use std::io::{self, BufRead};

use csv_core::{Reader as CoreReader, ReaderBuilder as CoreReaderBuilder};

use Terminator;

/// Builds a CSV reader with various configuration knobs.
///
/// This builder can be used to tweak the field delimiter, record terminator
/// and more for parsing CSV. Once a CSV `Reader` is built, its configuration
/// cannot be changed.
#[derive(Debug)]
pub struct ReaderBuilder {
    builder: CoreReaderBuilder,
    capacity: usize,
}

impl Default for ReaderBuilder {
    fn default() -> ReaderBuilder {
        ReaderBuilder {
            builder: CoreReaderBuilder::default(),
            capacity: 8 * (1<<10),
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
    eof: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadField {
    Field,
    Record,
    Done,
}

impl<R: io::Read> Reader<R> {
    /// Create a new CSV reader given a builder and a source of underlying
    /// bytes.
    fn new(builder: &ReaderBuilder, rdr: R) -> Reader<R> {
        Reader {
            core: builder.builder.build(),
            rdr: io::BufReader::with_capacity(builder.capacity, rdr),
            eof: false,
        }
    }

    pub fn read_field(
        &mut self,
        field: &mut Vec<u8>,
    ) -> io::Result<ReadField> {
        use csv_core::ReadResult::*;

        if self.eof {
            return Ok(ReadField::Done);
        }
        let mut outlen = 0;
        loop {
            let (res, nin, nout) = {
                let input = self.rdr.fill_buf()?;
                println!("input: {:?}", ::std::str::from_utf8(input));
                self.core.read(input, &mut field[outlen..])
            };
            println!("res: {:?}, nin: {:?}, nout: {:?}", res, nin, nout);
            outlen += nout;
            self.rdr.consume(nin);
            match res {
                InputEmpty => continue,
                OutputFull => {
                    let new_len = field.len().checked_mul(2).unwrap();
                    field.resize(cmp::max(4, new_len), 0);
                    continue;
                }
                Field { record_end: false } => {
                    field.truncate(outlen);
                    return Ok(ReadField::Field);
                }
                Field { record_end: true } => {
                    field.truncate(outlen);
                    return Ok(ReadField::Record);
                }
                End => {
                    self.eof = true;
                    field.truncate(outlen);
                    return Ok(ReadField::Done);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ReaderBuilder, ReadField};

    fn b(s: &str) -> &[u8] { s.as_bytes() }
    fn s(b: &[u8]) -> &str { ::std::str::from_utf8(b).unwrap() }

    #[test]
    fn scratch() {
        let data = b("foo,\"b,ar\",baz\nabc,mno,xyz");
        let mut rdr = ReaderBuilder::new().from_reader(data);

        let mut buf = vec![];

        let res = rdr.read_field(&mut buf).unwrap();
        assert_eq!(res, ReadField::Field);
        assert_eq!(s(&buf), "foo");

        buf.clear();
        let res = rdr.read_field(&mut buf).unwrap();
        assert_eq!(res, ReadField::Field);
        assert_eq!(s(&buf), "b,ar");

        buf.clear();
        let res = rdr.read_field(&mut buf).unwrap();
        assert_eq!(res, ReadField::Record);
        assert_eq!(s(&buf), "baz");

        buf.clear();
        let res = rdr.read_field(&mut buf).unwrap();
        assert_eq!(res, ReadField::Field);
        assert_eq!(s(&buf), "abc");

        buf.clear();
        let res = rdr.read_field(&mut buf).unwrap();
        assert_eq!(res, ReadField::Field);
        assert_eq!(s(&buf), "mno");

        buf.clear();
        let res = rdr.read_field(&mut buf).unwrap();
        assert_eq!(res, ReadField::Record);
        assert_eq!(s(&buf), "xyz");

        buf.clear();
        let res = rdr.read_field(&mut buf).unwrap();
        assert_eq!(res, ReadField::Done);
    }
}
