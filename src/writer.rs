use std::fs::File;
use std::io;
use std::path::Path;

use csv_core::{
    Writer as CoreWriter, WriterBuilder as CoreWriterBuilder,
    QuoteStyle, Terminator, WriteResult,
};

use byte_record::Position;
use error::Result;

/// Builds a CSV writer with various configuration knobs.
///
/// This builder can be used to tweak the field delimiter, record terminator
/// and more for writing CSV. Once a CSV `Writer` is built, its configuration
/// cannot be changed.
#[derive(Debug)]
pub struct WriterBuilder {
    builder: CoreWriterBuilder,
    capacity: usize,
    flexible: bool,
    has_headers: bool,
}

impl Default for WriterBuilder {
    fn default() -> WriterBuilder {
        WriterBuilder {
            builder: CoreWriterBuilder::default(),
            capacity: 8 * (1<<10),
            flexible: false,
            has_headers: true,
        }
    }
}

impl WriterBuilder {
    /// Create a new builder for configuring CSV writing.
    ///
    /// To convert a builder into a writer, call one of the methods starting
    /// with `from_`.
    pub fn new() -> WriterBuilder {
        WriterBuilder::default()
    }

    /// Build a CSV writer from this configuration that writer data to the
    /// given file path.
    ///
    /// If there was a problem opening the file at the given path, then this
    /// returns the corresponding error.
    pub fn from_path<P: AsRef<Path>>(&self, path: P) -> Result<Writer<File>> {
        Ok(Writer::new(self, File::create(path)?))
    }

    /// Build a CSV writer from this configuration that writes data to `wtr`.
    ///
    /// Note that the CSV writer is buffered automatically, so you should not
    /// wrap `wtr` in a buffered writer like `io::BufWriter`.
    pub fn from_writer<W: io::Write>(&self, wtr: W) -> Writer<W> {
        Writer::new(self, wtr)
    }

    /// The field delimiter to use when writing CSV.
    ///
    /// The default is `b','`.
    pub fn delimiter(&mut self, delimiter: u8) -> &mut WriterBuilder {
        self.builder.delimiter(delimiter);
        self
    }

    /// Whether to write a header row before writing any other row.
    ///
    /// When this is enabled and the `serialize` method is used to write data
    /// with something that contains field names (like a struct or a map), then
    /// a header row is written containing the field names before any other
    /// row is written.
    ///
    /// This option has no effect when using other methods to write rows. That
    /// is, if you don't use `serialize`, then you must write your header row
    /// explicitly if you want it.
    pub fn has_headers(&mut self, yes: bool) -> &mut WriterBuilder {
        self.has_headers = yes;
        self
    }

    /// Whether the number of fields in records is allowed to change or not.
    ///
    /// When disabled (which is the default), writing CSV data will return an
    /// error if a record is written with a number of fields different from the
    /// number of fields written in a previous record.
    ///
    /// When enabled, this error checking is turned off.
    pub fn flexible(&mut self, yes: bool) -> &mut WriterBuilder {
        self.flexible = yes;
        self
    }

    /// The record terminator to use when writing CSV.
    ///
    /// A record terminator can be any single byte. The default is a special
    /// value, `Terminator::CRLF`, which treats any occurrence of `\r`, `\n`
    /// or `\r\n` as a single record terminator.
    pub fn terminator(
        &mut self,
        term: Terminator,
    ) -> &mut WriterBuilder {
        self.builder.terminator(term);
        self
    }

    /// The quoting style to use when writing CSV.
    ///
    /// By default, this is set to `QuoteStyle::Necessary`, which will only
    /// use quotes when they are necessary to preserve the integrity of data.
    ///
    /// Note that regardless of this setting, an empty field is quoted if it is
    /// the only field in a record.
    pub fn quote_style(&mut self, style: QuoteStyle) -> &mut WriterBuilder {
        self.builder.quote_style(style);
        self
    }

    /// The quote character to use when writing CSV.
    ///
    /// The default is `b'"'`.
    pub fn quote(&mut self, quote: u8) -> &mut WriterBuilder {
        self.builder.quote(quote);
        self
    }

    /// The escape character to use when writing CSV.
    ///
    /// In some variants of CSV, quotes are escaped using a special escape
    /// character like `\` (instead of escaping quotes by doubling them).
    ///
    /// By default, writing these idiosyncratic escapes is disabled, and is
    /// only used when `double_quote` is disabled.
    pub fn escape(&mut self, escape: u8) -> &mut WriterBuilder {
        self.builder.escape(escape);
        self
    }

    /// Enable double quote escapes.
    ///
    /// This is enabled by default, but it may be disabled. When disabled,
    /// quotes in field data are escaped instead of doubled.
    pub fn double_quote(&mut self, yes: bool) -> &mut WriterBuilder {
        self.builder.double_quote(yes);
        self
    }

    /// Set the capacity (in bytes) of the buffer used in the CSV writer.
    pub fn buffer_capacity(&mut self, capacity: usize) -> &mut WriterBuilder {
        self.capacity = capacity;
        self
    }
}

#[derive(Debug)]
pub struct Writer<W: io::Write> {
    core: CoreWriter,
    wtr: Option<W>,
    buf: Buffer,
    state: WriterState,
}

#[derive(Debug)]
struct WriterState {
    flexible: bool,
    has_headers: bool,
    fields_written: u64,
    /// This is set immediately before flushing the buffer and then unset
    /// immediately after flushing the buffer. This avoids flushing the buffer
    /// twice if the inner writer panics.
    panicked: bool,
}

/// A simple internal buffer for buffering writes.
///
/// We need this because the `csv_core` APIs want to write into a `&mut [u8]`,
/// which is not available with the `std::io::BufWriter` API.
#[derive(Debug)]
struct Buffer {
    /// The contents of the buffer.
    buf: Vec<u8>,
    /// The number of bytes written to the buffer.
    len: usize,
}

impl<W: io::Write> Drop for Writer<W> {
    fn drop(&mut self) {
        if self.wtr.is_some() && !self.state.panicked {
            let _ = self.flush();
        }
    }
}

impl<W: io::Write> Writer<W> {
    fn new(builder: &WriterBuilder, wtr: W) -> Writer<W> {
        Writer {
            core: builder.builder.build(),
            wtr: Some(wtr),
            buf: Buffer {
                buf: vec![0; builder.capacity],
                len: 0,
            },
            state: WriterState {
                flexible: builder.flexible,
                has_headers: builder.has_headers,
                fields_written: 0,
                panicked: false,
            },
        }
    }

    /// Write a single record.
    pub fn write_record<I, T>(&mut self, record: I) -> Result<()>
        where I: IntoIterator<Item=T>, T: AsRef<[u8]>
    {
        for field in record.into_iter() {
            self.write_field(field)?;
        }
        self.write_terminator()
    }

    /// Write a single field.
    ///
    /// One should prefer using `write_record` over this method. It is provided
    /// for cases where writing a field at a time is more convenient than
    /// writing a record at a time.
    ///
    /// Note that if this API is used, `write_record` should be called with an
    /// empty iterator to write a record terminator.
    pub fn write_field<T: AsRef<[u8]>>(&mut self, field: T) -> Result<()> {
        if self.state.fields_written > 0 {
            self.write_delimiter()?;
        }
        let mut field = field.as_ref();
        loop {
            let (res, nin, nout) = self.core.field(field, self.buf.writable());
            field = &field[nin..];
            self.buf.written(nout);
            match res {
                WriteResult::InputEmpty => {
                    self.state.fields_written += 1;
                    return Ok(());
                }
                WriteResult::OutputFull => self.flush()?,
            }
        }
    }

    /// Flush the contents of the internal buffer to the underlying writer.
    ///
    /// If there was a problem writing to the underlying writer, then an error
    /// is returned.
    ///
    /// Note that this also flushes the underlying writer.
    pub fn flush(&mut self) -> Result<()> {
        self.state.panicked = true;
        let result = self.wtr.as_mut().unwrap().write_all(self.buf.readable());
        self.state.panicked = false;
        result?;
        self.buf.clear();
        self.wtr.as_mut().unwrap().flush()?;
        Ok(())
    }

    /// Flush the contents of the internal buffer and return the underlying
    /// writer.
    pub fn into_inner(mut self) -> Result<W> {
        // TODO(burntsushi): This should return an IntoInnerError so that
        // callers can re-capture ownership of the writer.
        self.flush()?;
        Ok(self.wtr.take().unwrap())
    }

    /// Write a CSV delimiter.
    fn write_delimiter(&mut self) -> Result<()> {
        loop {
            let (res, nout) = self.core.delimiter(self.buf.writable());
            self.buf.written(nout);
            match res {
                WriteResult::InputEmpty => return Ok(()),
                WriteResult::OutputFull => self.flush()?,
            }
        }
    }

    /// Write a CSV terminator.
    fn write_terminator(&mut self) -> Result<()> {
        loop {
            let (res, nout) = self.core.terminator(self.buf.writable());
            self.buf.written(nout);
            match res {
                WriteResult::InputEmpty => return Ok(()),
                WriteResult::OutputFull => self.flush()?,
            }
        }
    }
}

impl Buffer {
    /// Returns a slice of the buffer's current contents.
    ///
    /// The slice returned may be empty.
    fn readable(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    /// Returns a mutable slice of the remaining space in this buffer.
    ///
    /// The slice returned may be empty.
    fn writable(&mut self) -> &mut [u8] {
        &mut self.buf[self.len..]
    }

    /// Indicates that `n` bytes have been written to this buffer.
    fn written(&mut self, n: usize) {
        self.len += n;
    }

    /// Clear the buffer.
    fn clear(&mut self) {
        self.len = 0;
    }
}

#[cfg(test)]
mod tests {
    use byte_record::ByteRecord;
    use string_record::StringRecord;

    use super::WriterBuilder;

    fn b(s: &str) -> &[u8] { s.as_bytes() }

    #[test]
    fn one_record() {
        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        wtr.write_record(vec!["a", "b", "c"]).unwrap();

        assert_eq!(wtr.into_inner().unwrap(), b("a,b,c\n"));
    }

    #[test]
    fn one_string_record() {
        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        wtr.write_record(&StringRecord::from(vec!["a", "b", "c"])).unwrap();

        assert_eq!(wtr.into_inner().unwrap(), b("a,b,c\n"));
    }

    #[test]
    fn one_byte_record() {
        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        wtr.write_record(&ByteRecord::from(vec!["a", "b", "c"])).unwrap();

        assert_eq!(wtr.into_inner().unwrap(), b("a,b,c\n"));
    }
}
