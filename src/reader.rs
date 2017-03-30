use std::cmp;
use std::io::{self, BufRead};

use csv_core::{Reader as CoreReader, ReaderBuilder as CoreReaderBuilder};

use record::ByteRecord;
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
    End,
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

    pub fn read_record_bytes(
        &mut self,
        record: &mut ByteRecord,
    ) -> io::Result<bool> {
        use csv_core::ReadResult::*;

        let (mut fields, mut starts) = record.as_parts();
        starts.clear();
        starts.push(0);
        if self.eof {
            return Ok(false);
        }
        unsafe {
            // SAFETY: Since `fields` is a `Vec<u8>`, we don't need to worry
            // about ownership of elements in the vec. Also, since `len` is
            // always `capacity`, we know that the `len` is always valid.
            // However, it's possible that some space will be unitialized.
            // In the loop below, we never read from `fields` and always
            // truncate the vec to the last location that has been written to.
            //
            // PERFORMANCE: Neglecting this optimization (and using `resize`
            // instead) results in a sizable drop in the `count_*_record_bytes`
            // benchmarks. Seems like something specialization could fix?
            let len = fields.capacity();
            fields.set_len(len);
        }

        let mut inlen = 0;
        let mut outlen = 0;
    'OUTER:
        loop {
            // We use a double loop here to amortize dealing with the buffer.
            // For CSV files with longish records, the common case will be
            // the inner loop churning through the fields in the record with
            // a single call to `fill_buf`/`consume`.
            self.rdr.consume(inlen);
            inlen = 0;
            let input = self.rdr.fill_buf()?;
            loop {
                let (res, nin, nout) =
                    self.core.read(&input[inlen..], &mut fields[outlen..]);
                inlen += nin;
                outlen += nout;
                match res {
                    InputEmpty => break,
                    OutputFull => {
                        let new_len = fields.len().checked_mul(2).unwrap();
                        // This is amortized, so we shouldn't need to do
                        // anything fancy here.
                        fields.resize(cmp::max(4, new_len), 0);
                        continue;
                    }
                    Field { record_end } => {
                        starts.push(outlen);
                        if record_end {
                            break 'OUTER;
                        }
                    }
                    End => { self.eof = true; break 'OUTER; }
                }
                // If our buffer ran out, break and try to refill it.
                // (The core reader interprets an emtpy slice as EOF, and we
                // aren't necessarily at EOF here.)
                if input[inlen..].is_empty() {
                    break;
                }
            }
        }
        self.rdr.consume(inlen);
        fields.truncate(outlen);
        Ok(!self.eof)
    }

    pub fn read_field_bytes(
        &mut self,
        field: &mut Vec<u8>,
    ) -> io::Result<ReadField> {
        use csv_core::ReadResult::*;

        if self.eof {
            return Ok(ReadField::End);
        }
        let len = field.capacity();
        unsafe {
            // SAFETY: Since `field` is a `Vec<u8>`, we don't need to worry
            // about ownership of elements in the vec. Also, since `len` is
            // always `capacity`, we know that the `len` is always valid.
            // However, it's possible that some space will be unitialized.
            // In the loop below, we never read from `field` and always
            // truncate the vec to the last location that has been written to.
            //
            // PERFORMANCE: Neglecting this optimization (and using `resize`
            // instead) results in a 50% drop in the `count_*_field_bytes`
            // benchmarks. Seems like something specialization could fix?
            field.set_len(len);
        }

        let mut outlen = 0;
        loop {
            let (res, nin, nout) = {
                let input = self.rdr.fill_buf()?;
                self.core.read(input, &mut field[outlen..])
            };
            self.rdr.consume(nin);
            outlen += nout;
            let state = match res {
                InputEmpty => continue,
                OutputFull => {
                    let new_len = field.len().checked_mul(2).unwrap();
                    // This is amortized, so we shouldn't need to do anything
                    // fancy here.
                    field.resize(cmp::max(4, new_len), 0);
                    continue;
                }
                Field { record_end: false } => ReadField::Field,
                Field { record_end: true } => ReadField::Record,
                End => { self.eof = true; ReadField::End }
            };
            // This is not only for correctness but for safety as well.
            // Namely, bytes after `outlen` in `field` may be uninitialized.
            field.truncate(outlen);
            return Ok(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use record::ByteRecord;
    use super::{ReaderBuilder, ReadField};

    fn b(s: &str) -> &[u8] { s.as_bytes() }
    fn s(b: &[u8]) -> &str { ::std::str::from_utf8(b).unwrap() }

    macro_rules! assert_read_field_bytes {
        ($rdr:expr, $buf:expr, $res:expr, $out:expr) => {{
            $buf.clear();
            let res = $rdr.read_field_bytes(&mut $buf).unwrap();
            assert_eq!(res, $res);
            assert_eq!($out, s(&$buf));
        }}
    }

    #[test]
    fn read_field_bytes() {
        let data = b("foo,\"b,ar\",baz\nabc,mno,xyz");
        let mut rdr = ReaderBuilder::new().from_reader(data);

        let mut buf = vec![];

        assert_read_field_bytes!(rdr, buf, ReadField::Field, "foo");
        assert_read_field_bytes!(rdr, buf, ReadField::Field, "b,ar");
        assert_read_field_bytes!(rdr, buf, ReadField::Record, "baz");
        assert_read_field_bytes!(rdr, buf, ReadField::Field, "abc");
        assert_read_field_bytes!(rdr, buf, ReadField::Field, "mno");
        assert_read_field_bytes!(rdr, buf, ReadField::Record, "xyz");
        assert_read_field_bytes!(rdr, buf, ReadField::End, "");
    }

    #[test]
    fn read_record_bytes() {
        let data = b("foo,\"b,ar\",baz\nabc,mno,xyz");
        let mut rdr = ReaderBuilder::new().from_reader(data);
        let mut rec = ByteRecord::new();

        assert!(rdr.read_record_bytes(&mut rec).unwrap());
        assert_eq!("foo", s(&rec[0]));
        assert_eq!("b,ar", s(&rec[1]));
        assert_eq!("baz", s(&rec[2]));

        assert!(rdr.read_record_bytes(&mut rec).unwrap());
        assert_eq!("abc", s(&rec[0]));
        assert_eq!("mno", s(&rec[1]));
        assert_eq!("xyz", s(&rec[2]));

        assert!(!rdr.read_record_bytes(&mut rec).unwrap());
    }
}
