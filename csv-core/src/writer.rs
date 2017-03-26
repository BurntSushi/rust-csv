use memchr::memchr;

use Terminator;

/// The quoting style to use when writing CSV data.
#[derive(Clone, Copy, Debug)]
pub enum QuoteStyle {
    /// This puts quotes around every field. Always.
    Always,
    /// This puts quotes around fields only when necessary.
    ///
    /// They are necessary when fields are empty or contain a quote, delimiter
    /// or record terminator.
    ///
    /// This is the default.
    Necessary,
    /// This *never* writes quotes.
    ///
    /// If a field requires quotes, then the writer will report an error.
    Never,
}

impl Default for QuoteStyle {
    fn default() -> QuoteStyle {
        QuoteStyle::Necessary
    }
}

/// A builder for configuring a CSV writer.
///
/// This builder permits specifying the CSV delimiter, terminator, quoting
/// style and more.
#[derive(Debug)]
pub struct WriterBuilder {
    wtr: Writer,
}

impl WriterBuilder {
    /// Create a new builder for configuring a CSV writer.
    pub fn new() -> WriterBuilder {
        WriterBuilder { wtr: Writer::default() }
    }

    /// The field delimiter to use when writing CSV.
    ///
    /// The default is `b','`.
    pub fn delimiter(&mut self, delimiter: u8) -> &mut WriterBuilder {
        self.wtr.delimiter = delimiter;
        self
    }

    /// The record terminator to use when writing CSV.
    ///
    /// A record terminator can be any single byte. The default is a special
    /// value, `Terminator::CRLF`, which uses `\r\n` as the record terminator.
    ///
    /// The default is `b'\n'`.
    pub fn terminator(&mut self, term: Terminator) -> &mut WriterBuilder {
        self.wtr.term = term;
        self
    }

    /// The quoting style to use when writing CSV.
    ///
    /// By default, this is set to `QuoteStyle::Necessary`, which will only
    /// use quotes when they are necessary to preserve the integrity of data.
    pub fn quote_style(&mut self, style: QuoteStyle) -> &mut WriterBuilder {
        self.wtr.style = style;
        self
    }

    /// The quote character to use when writing CSV.
    ///
    /// The default value is `b'"'`.
    pub fn quote(&mut self, quote: u8) -> &mut WriterBuilder {
        self.wtr.quote = quote;
        self
    }

    /// The escape character to use when writing CSV.
    ///
    /// This is only used when `double_quote` is set to `false`.
    ///
    /// The default value is `b'\\'`.
    pub fn escape(&mut self, escape: u8) -> &mut WriterBuilder {
        self.wtr.escape = escape;
        self
    }

    /// The quoting escape mechanism to use when writing CSV.
    ///
    /// When enabled (which is the default), quotes are escaped by doubling
    /// them. e.g., `"` escapes to `""`.
    ///
    /// When disabled, quotes are escaped with the escape character (which
    /// is `\\` by default).
    pub fn double_quote(&mut self, yes: bool) -> &mut WriterBuilder {
        self.wtr.double_quote = yes;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WriteResult {
    InputEmpty,
    OutputFull,
}

/// A writer for CSV data.
///
/// # RFC 4180
///
/// This writer conforms to RFC 4180 with one exception: it doesn't guarantee
/// that all records written are of the same length. Instead, the onus is on
/// the caller to ensure that all records written are of the same length.
#[derive(Debug)]
pub struct Writer {
    state: WriterState,
    delimiter: u8,
    term: Terminator,
    style: QuoteStyle,
    quote: u8,
    escape: u8,
    double_quote: bool,
}

#[derive(Debug)]
struct WriterState {
    quoting: bool,
}

impl Writer {
    /// Creates a new CSV writer with the default configuration.
    pub fn new() -> Writer {
        Writer::default()
    }

    pub fn field(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> (WriteResult, usize, usize) {
        (WriteResult::InputEmpty, 0, 0)
    }

    pub fn delimiter(&mut self, output: &mut [u8]) -> (WriteResult, usize) {
        let (res, nout) = self.write(&[self.delimiter], output);
        if nout > 0 {
            self.state.quoting = false;
        }
        (res, nout)
    }

    pub fn terminator(&mut self, output: &mut [u8]) -> (WriteResult, usize) {
        let (res, nout) = match self.term {
            Terminator::CRLF => write_pessimistic(&[b'\r', b'\n'], output),
            Terminator::Any(b) => write_pessimistic(&[b], output),
        };
        if nout > 0 {
            self.state.quoting = false;
        }
        (res, nout)
    }

    /// Returns true if and only if the given input field requires quotes,
    /// taking into account the current configuration of this writer.
    pub fn needs_quotes(&self, input: &[u8]) -> bool {
        input.iter().any(|&b| self.byte_needs_quotes(b))
    }

    fn byte_needs_quotes(&self, b: u8) -> bool {
        self.delimiter == b
        || self.term == b
        || self.quote == b
        // This is a bit hokey. By default, the record terminator is
        // '\n', but we still need to quote '\r' because the reader
        // interprets '\r' as a record terminator by default.
        || b == b'\r' || b == b'\n'
    }

    fn write(&self, data: &[u8], output: &mut [u8]) -> (WriteResult, usize) {
        if data.len() > output.len() {
            (WriteResult::OutputFull, 0)
        } else {
            output[..data.len()].copy_from_slice(data);
            (WriteResult::InputEmpty, data.len())
        }
    }
}

impl Default for Writer {
    fn default() -> Writer {
        Writer {
            state: WriterState::default(),
            delimiter: b',',
            term: Terminator::Any(b'\n'),
            style: QuoteStyle::default(),
            quote: b'"',
            escape: b'\\',
            double_quote: true,
        }
    }
}

impl Default for WriterState {
    fn default() -> WriterState {
        WriterState {
            quoting: false,
        }
    }
}

pub fn quote(
    mut input: &[u8],
    mut output: &mut [u8],
    quote: u8,
    escape: u8,
    doubled: bool,
) -> (WriteResult, usize, usize) {
    let (mut nin, mut nout) = (0, 0);
    loop {
        match memchr(quote, input) {
            None => {
                let (res, i, o) = write_optimistic(input, output);
                nin += i;
                nout += o;
                return (res, nin, nout);
            }
            _ => unimplemented!(),
        }
    }
}

fn write_optimistic(
    input: &[u8],
    output: &mut [u8],
) -> (WriteResult, usize, usize) {
    if input.len() > output.len() {
        let input = &input[..output.len()];
        output.copy_from_slice(input);
        (WriteResult::OutputFull, output.len(), output.len())
    } else {
        output[..input.len()].copy_from_slice(input);
        (WriteResult::InputEmpty, input.len(), input.len())
    }
}

fn write_pessimistic(
    input: &[u8],
    output: &mut [u8],
) -> (WriteResult, usize) {
    if input.len() > output.len() {
        (WriteResult::OutputFull, 0)
    } else {
        output[..input.len()].copy_from_slice(input);
        (WriteResult::InputEmpty, input.len())
    }
}
