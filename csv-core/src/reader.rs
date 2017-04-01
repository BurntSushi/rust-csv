use core::fmt;

/// A record terminator.
///
/// Use this specify the record terminator while parsing CSV. The default is
/// CRLF, which treats `\r`, `\n` or `\r\n` as a single record terminator.
#[derive(Clone, Copy, Debug)]
pub enum Terminator {
    /// Parses `\r`, `\n` or `\r\n` as a single record terminator.
    CRLF,
    /// Parses the byte given as a record terminator.
    Any(u8),
}

impl Terminator {
    fn is_crlf(&self) -> bool {
        match *self {
            Terminator::CRLF => true,
            Terminator::Any(_) => false,
        }
    }
}

impl Default for Terminator {
    fn default() -> Terminator {
        Terminator::CRLF
    }
}

impl PartialEq<u8> for Terminator {
    #[inline]
    fn eq(&self, &other: &u8) -> bool {
        match *self {
            Terminator::CRLF => other == b'\r' || other == b'\n',
            Terminator::Any(b) => other == b,
        }
    }
}

/// A pull based CSV reader.
///
/// This reader parses CSV data using a finite state machine. Callers can
/// extract parsed data incrementally using the `read` method.
///
/// Note that this CSV reader is somewhat encoding agnostic. The source data
/// needs to be at least ASCII compatible. There is no support for specifying
/// the full gamut of Unicode delimiters/terminators/quotes/escapes. Instead,
/// any byte can be used, although callers probably want to stick to the ASCII
/// subset (`<= 0x7F`).
///
/// # RFC 4180
///
/// [RFC 4180](https://tools.ietf.org/html/rfc4180)
/// is the closest thing to a specification for CSV data. Unfortunately,
/// CSV data that is seen in the wild can vary significantly. Often times, the
/// CSV data is outright invalid. Instead of fixing the producers of bad CSV
/// data, we have seen fit to make consumers much more flexible in what they
/// accept. This reader continues that tradition, and therefore, isn't
/// technically compliant with RFC 4180. In particular, this reader will
/// never return an error and will always find *a* parse.
///
/// Here are some detailed differences from RFC 4180:
///
/// * CRLF, LF and CR are each treated as a single record terminator by
///   default.
/// * Records are permitted to be of varying length.
/// * Empty lines (that do not include other whitespace) are ignored.
#[derive(Clone, Debug)]
pub struct Reader {
    /// A table-based DFA for parsing CSV.
    dfa: Dfa,
    /// The current DFA state, if the DFA is used.
    dfa_state: DfaState,
    /// The current NFA state, if the NFA is used.
    nfa_state: NfaState,
    /// Whether to copy fields into a caller provided output buffer.
    copy: bool,
    /// The delimiter that separates fields.
    delimiter: u8,
    /// The terminator that separates records.
    term: Terminator,
    /// The quotation byte.
    quote: u8,
    /// Whether to recognize escaped quotes.
    escape: Option<u8>,
    /// Whether to recognized doubled quotes.
    double_quote: bool,
    /// Whether to use the NFA for parsing.
    ///
    /// Generally this is for debugging. There's otherwise no good reason
    /// to avoid the DFA.
    use_nfa: bool,
    /// The current line number.
    line: u64,
    /// The current position in the output buffer when reading a record.
    output_pos: usize,
}

impl Default for Reader {
    fn default() -> Reader {
        Reader {
            dfa: Dfa::new(),
            dfa_state: DfaState::start(),
            nfa_state: NfaState::StartRecord,
            copy: true,
            delimiter: b',',
            term: Terminator::default(),
            quote: b'"',
            escape: None,
            double_quote: true,
            use_nfa: false,
            line: 1,
            output_pos: 0,
        }
    }
}

/// Builds a CSV reader with various configuration knobs.
///
/// This builder can be used to tweak the field delimiter, record terminator
/// and more for parsing CSV. Once a CSV `Reader` is built, its configuration
/// cannot be changed.
#[derive(Debug, Default)]
pub struct ReaderBuilder {
    rdr: Reader,
}

impl ReaderBuilder {
    /// Create a new builder.
    pub fn new() -> ReaderBuilder {
        ReaderBuilder::default()
    }

    /// Build a CSV parser from this configuration.
    pub fn build(&self) -> Reader {
        let mut rdr = self.rdr.clone();
        rdr.build_dfa();
        rdr
    }

    /// The field delimiter to use when parsing CSV.
    ///
    /// The default is `b','`.
    pub fn delimiter(&mut self, delimiter: u8) -> &mut ReaderBuilder {
        self.rdr.delimiter = delimiter;
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
        self.rdr.term = term;
        self
    }

    /// The quote character to use when parsing CSV.
    ///
    /// The default is `b'"'`.
    pub fn quote(&mut self, quote: u8) -> &mut ReaderBuilder {
        self.rdr.quote = quote;
        self
    }

    /// The escape character to use when parsing CSV.
    ///
    /// In some variants of CSV, quotes are escaped using a special escape
    /// character like `\` (instead of escaping quotes by doubling them).
    ///
    /// By default, recognizing these idiosyncratic escapes is disabled.
    pub fn escape(&mut self, escape: Option<u8>) -> &mut ReaderBuilder {
        self.rdr.escape = escape;
        self
    }

    /// Enable double quote escapes.
    ///
    /// This is enabled by default, but it may be disabled. When disabled,
    /// doubled quotes are not interpreted as escapes.
    pub fn double_quote(&mut self, yes: bool) -> &mut ReaderBuilder {
        self.rdr.double_quote = yes;
        self
    }

    /// A convenience method for specifying a configuration to read ASCII
    /// delimited text.
    ///
    /// This sets the delimiter and record terminator to the ASCII unit
    /// separator (`\x1F`) and record separator (`\x1E`), respectively.
    pub fn ascii(&mut self) -> &mut ReaderBuilder {
        self.delimiter(b'\x1F').terminator(Terminator::Any(b'\x1E'))
    }

    /// Enable or disable support for copying field data into a caller
    /// provided buffer when using `read`.
    ///
    /// This is enabled by default. It *may* be useful to disable this if
    /// all you care about is counting CSV fields or records.
    pub fn copy(&mut self, yes: bool) -> &mut ReaderBuilder {
        self.rdr.copy = yes;
        self
    }

    /// Enable or disable the NFA for parsing CSV.
    ///
    /// This is intended to be a debug option useful for debugging. The NFA
    /// is always slower than the DFA.
    #[doc(hidden)]
    pub fn nfa(&mut self, yes: bool) -> &mut ReaderBuilder {
        self.rdr.use_nfa = yes;
        self
    }
}

/// The result of parsing at most one field from CSV data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadFieldResult {
    /// The caller provided input was exhausted before the end of a field or
    /// record was found.
    InputEmpty,
    /// The caller provided output buffer was filled before an entire field
    /// could be written to it.
    OutputFull,
    /// The end of a field was found.
    ///
    /// Note that when `record_end` is true, then the end of this field also
    /// corresponds to the end of a record.
    Field {
        /// Whether this was the last field in a record or not.
        record_end: bool,
    },
    /// All CSV data has been read.
    ///
    /// This state can only be returned when an empty input buffer is provided
    /// by the caller.
    End,
}

impl ReadFieldResult {
    fn from_nfa(
        state: NfaState,
        inpdone: bool,
        outdone: bool,
    ) -> ReadFieldResult {
        match state {
            NfaState::End => ReadFieldResult::End,
            NfaState::EndRecord | NfaState::CRLF => {
                ReadFieldResult::Field { record_end: true }
            }
            NfaState::EndFieldDelim => {
                ReadFieldResult::Field { record_end: false }
            }
            _ => {
                assert!(!state.is_field_final());
                if !inpdone && outdone {
                    ReadFieldResult::OutputFull
                } else {
                    ReadFieldResult::InputEmpty
                }
            }
        }
    }

    /// Convert this to a NoCopy result for use inside the NFA. This panics
    /// if `self` is `ReadFieldResult::OutputFull`.
    fn as_read_field_nocopy_result(&self) -> ReadFieldNoCopyResult {
        match *self {
            ReadFieldResult::InputEmpty => ReadFieldNoCopyResult::InputEmpty,
            ReadFieldResult::OutputFull => {
                panic!("cannot convert OutputFull result into nocopy result");
            }
            ReadFieldResult::Field { record_end } => {
                ReadFieldNoCopyResult::Field { record_end: record_end }
            }
            ReadFieldResult::End => ReadFieldNoCopyResult::End,
        }
    }
}

/// The result of parsing at most one field from CSV data while ignoring the
/// output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadFieldNoCopyResult {
    /// The caller provided input was exhausted before the end of a field or
    /// record was found.
    InputEmpty,
    /// The end of a field was found.
    ///
    /// Note that when `record_end` is true, then the end of this field also
    /// corresponds to the end of a record.
    Field {
        /// Whether this was the last field in a record or not.
        record_end: bool,
    },
    /// All CSV data has been read.
    ///
    /// This state can only be returned when an empty input buffer is provided
    /// by the caller.
    End,
}

/// The result of parsing at most one record from CSV data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadRecordResult {
    /// The caller provided input was exhausted before the end of a record was
    /// found.
    InputEmpty,
    /// The caller provided output buffer was filled before an entire field
    /// could be written to it.
    OutputFull,
    /// The caller provided output buffer of field end poisitions was filled
    /// before the next field could be parsed.
    OutputEndsFull,
    /// The end of a record was found.
    Record,
    /// All CSV data has been read.
    ///
    /// This state can only be returned when an empty input buffer is provided
    /// by the caller.
    End,
}

impl ReadRecordResult {
    fn is_record(&self) -> bool {
        *self == ReadRecordResult::Record
    }

    fn from_nfa(
        state: NfaState,
        inpdone: bool,
        outdone: bool,
        endsdone: bool,
    ) -> ReadRecordResult {
        match state {
            NfaState::End => ReadRecordResult::End,
            NfaState::EndRecord | NfaState::CRLF => ReadRecordResult::Record,
            _ => {
                assert!(!state.is_record_final());
                if !inpdone && outdone {
                    ReadRecordResult::OutputFull
                } else if !inpdone && endsdone {
                    ReadRecordResult::OutputEndsFull
                } else {
                    ReadRecordResult::InputEmpty
                }
            }
        }
    }

    /// Convert this to a NoCopy result for use inside the NFA. This panics
    /// if `self` is `ReadRecordResult::OutputFull` or
    /// `ReadRecordResult::OutputEndsFull`.
    fn as_read_record_nocopy_result(&self) -> ReadRecordNoCopyResult {
        match *self {
            ReadRecordResult::InputEmpty => ReadRecordNoCopyResult::InputEmpty,
            ReadRecordResult::OutputFull => {
                panic!("cannot convert OutputFull result into nocopy result");
            }
            ReadRecordResult::OutputEndsFull => {
                panic!("cannot convert OutputEndsFull result \
                        into nocopy result");
            }
            ReadRecordResult::Record => ReadRecordNoCopyResult::Record,
            ReadRecordResult::End => ReadRecordNoCopyResult::End,
        }
    }
}

/// The result of parsing at most one record from CSV data while ignoring
/// output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadRecordNoCopyResult {
    /// The caller provided input was exhausted before the end of a record was
    /// found.
    InputEmpty,
    /// The end of a record was found.
    Record,
    /// All CSV data has been read.
    ///
    /// This state can only be returned when an empty input buffer is provided
    /// by the caller.
    End,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum NfaState {
    StartRecord = 0,
    StartField = 1,
    InField = 2,
    InQuotedField = 3,
    InEscapedQuote = 4,
    InDoubleEscapedQuote = 5,
    EndFieldDelim = 6,
    EndRecord = 7,
    CRLF = 8,

    // These states aren't used in the DFA, so we
    // assign them meaningless numbers.
    EndFieldTerm = 200,
    InRecordTerm = 201,
    End = 202,
}

const NFA_STATES: &'static [NfaState] = &[
    NfaState::StartRecord,
    NfaState::StartField,
    NfaState::EndFieldDelim,
    NfaState::InField,
    NfaState::InQuotedField,
    NfaState::InEscapedQuote,
    NfaState::InDoubleEscapedQuote,
    NfaState::EndRecord,
    NfaState::CRLF,
];

impl NfaState {
    fn is_field_final(&self) -> bool {
        match *self {
            NfaState::End
            | NfaState::EndRecord
            | NfaState::CRLF
            | NfaState::EndFieldDelim => true,
            _ => false,
        }
    }

    fn is_record_final(&self) -> bool {
        match *self {
            NfaState::End
            | NfaState::EndRecord
            | NfaState::CRLF => true,
            _ => false,
        }
    }
}

impl Reader {
    /// Create a new CSV reader with a default parser configuration.
    pub fn new() -> Reader {
        ReaderBuilder::new().build()
    }

    /// Reset the parser such that it behaves as if it had never been used.
    ///
    /// This may be useful when reading CSV data in a random access pattern.
    pub fn reset(&mut self) {
        self.dfa_state = self.dfa.new_state(NfaState::StartRecord);
        self.nfa_state = NfaState::StartRecord;
        self.line = 1;
    }

    /// Return the current line number as measured by the number of occurrences
    /// of `\n`.
    ///
    /// Line numbers starts at `1` and are reset when `reset` is called.
    pub fn line(&self) -> u64 {
        self.line
    }

    /// Parse CSV data in `input` and copy field data to `output`.
    ///
    /// This routine requires a caller provided buffer of CSV data as the
    /// `input` and a caller provided buffer, `output`, in which to store field
    /// data extracted from `input`. The field data copied to `output` will
    /// have its quotes unescaped.
    ///
    /// Calling this routine parses at most a single field and returns
    /// three values indicating the state of the parser. The first value,
    /// a `ReadFieldResult`, tells the caller what to do next. For example,
    /// if the entire input was read or if the output buffer was filled
    /// before a full field had been read, then `ReadFieldResult::InputEmpty` or
    /// `ReadFieldResult::OutputFull` is returned, respectively. See the
    /// documentation for `ReadFieldResult` for more details.
    ///
    /// The second two values returned correspond to the number of bytes
    /// read from `input` and `output`, respectively.
    ///
    /// # Termination
    ///
    /// This reader interprets an empty `input` buffer as an indication that
    /// there is no CSV data left to read. Namely, when the caller has
    /// exhausted all CSV data, the caller should continue to call `read` with
    /// an empty input buffer until `ReadFieldResult::End` is returned.
    ///
    /// # Errors
    ///
    /// This CSV reader can never return an error. Instead, it prefers *a*
    /// parse over *no* parse.
    pub fn read_field(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> (ReadFieldResult, usize, usize) {
        if self.use_nfa {
            self.read_field_nfa(input, output)
        } else {
            self.read_field_dfa(input, output)
        }
    }

    /// TODO
    pub fn read_field_nocopy(
        &mut self,
        input: &[u8],
    ) -> (ReadFieldNoCopyResult, usize) {
        if self.use_nfa {
            let (res, nin, _) = self.read_field_nfa(input, &mut []);
            (res.as_read_field_nocopy_result(), nin)
        } else {
            self.read_field_nocopy_dfa(input)
        }
    }

    /// TODO
    pub fn read_record(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        ends: &mut [usize],
    ) -> (ReadRecordResult, usize, usize, usize) {
        if self.use_nfa {
            self.read_record_nfa(input, output, ends)
        } else {
            self.read_record_dfa(input, output, ends)
        }
    }

    /// TODO
    pub fn read_record_nocopy(
        &mut self,
        input: &[u8],
    ) -> (ReadRecordNoCopyResult, usize) {
        if self.use_nfa {
            let (res, nin, _, _) = self.read_record_nfa(
                input, &mut [], &mut []);
            (res.as_read_record_nocopy_result(), nin)
        } else {
            self.read_record_nocopy_dfa(input)
        }
    }

    #[inline(always)]
    fn read_record_dfa(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        ends: &mut [usize],
    ) -> (ReadRecordResult, usize, usize, usize) {
        if input.is_empty() {
            let s = self.transition_final_dfa(self.dfa_state);
            let res = self.dfa.new_read_record_result(
                s, true, false, false, false);
            // This part is a little tricky. When reading the final record,
            // the last result the caller will get is an InputEmpty, and while
            // they'll have everything they need in `output`, they'll be
            // missing the final end position of the final field in `ends`.
            // We insert that here, but we must take care to handle the case
            // where `ends` doesn't have enough space. If it doesn't have
            // enough space, then we also can't transition to the next state.
            return match res {
                ReadRecordResult::Record => {
                    if ends.is_empty() {
                        return (ReadRecordResult::OutputEndsFull, 0, 0, 0);
                    }
                    self.dfa_state = s;
                    ends[0] = self.output_pos;
                    self.output_pos = 0;
                    (res, 0, 0, 1)
                }
                _ => {
                    self.dfa_state = s;
                    (res, 0, 0, 0)
                }
            };
        }
        if output.is_empty() {
            return (ReadRecordResult::OutputFull, 0, 0, 0);
        }
        if ends.is_empty() {
            return (ReadRecordResult::OutputEndsFull, 0, 0, 0);
        }
        let (mut nin, mut nout, mut nend) = (0, 0, 0);
        let mut state = self.dfa_state;
        while nin < input.len() && nout < output.len() && nend < ends.len() {
            let (s, has_out) = self.dfa.get_output(state, input[nin]);
            self.line += (input[nin] == b'\n') as u64;
            state = s;
            if has_out {
                output[nout] = input[nin];
                nout += 1;
            }
            nin += 1;
            if state >= self.dfa.final_field {
                ends[nend] = self.output_pos + nout;
                nend += 1;
                if state > self.dfa.final_field {
                    break;
                }
            }
        }
        let res = self.dfa.new_read_record_result(
            state, false,
            nin >= input.len(),
            nout >= output.len(),
            nend >= ends.len());
        self.dfa_state = state;
        if res.is_record() {
            self.output_pos = 0;
        } else {
            self.output_pos += nout;
        }
        (res, nin, nout, nend)
    }

    #[inline(always)]
    fn read_record_nocopy_dfa(
        &mut self,
        input: &[u8],
    ) -> (ReadRecordNoCopyResult, usize) {
        if input.is_empty() {
            self.dfa_state = self.transition_final_dfa(self.dfa_state);
            let res = self.dfa.new_read_record_nocopy_result(
                self.dfa_state, true);
            return (res, 0);
        }
        let mut nin = 0;
        let mut state = self.dfa_state;
        while nin < input.len() {
            state = self.dfa.get(state, input[nin]);
            self.line += (input[nin] == b'\n') as u64;
            nin += 1;
            if state >= self.dfa.final_record {
                break;
            }
            if nin + 4 < input.len() {
                state = self.dfa.get(state, input[nin]);
                self.line += (input[nin] == b'\n') as u64;
                nin += 1;
                if state >= self.dfa.final_record {
                    break;
                }
                state = self.dfa.get(state, input[nin]);
                self.line += (input[nin] == b'\n') as u64;
                nin += 1;
                if state >= self.dfa.final_record {
                    break;
                }
                state = self.dfa.get(state, input[nin]);
                self.line += (input[nin] == b'\n') as u64;
                nin += 1;
                if state >= self.dfa.final_record {
                    break;
                }
                state = self.dfa.get(state, input[nin]);
                self.line += (input[nin] == b'\n') as u64;
                nin += 1;
                if state >= self.dfa.final_record {
                    break;
                }
                state = self.dfa.get(state, input[nin]);
                self.line += (input[nin] == b'\n') as u64;
                nin += 1;
                if state >= self.dfa.final_record {
                    break;
                }
            }
        }
        let res = self.dfa.new_read_record_nocopy_result(state, false);
        self.dfa_state = state;
        (res, nin)
    }

    #[inline(always)]
    fn read_field_dfa(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> (ReadFieldResult, usize, usize) {
        if input.is_empty() {
            self.dfa_state = self.transition_final_dfa(self.dfa_state);
            let res = self.dfa.new_read_field_result(
                self.dfa_state, true, false, false);
            return (res, 0, 0);
        }
        if output.is_empty() {
            return (ReadFieldResult::OutputFull, 0, 0);
        }
        let (mut nin, mut nout) = (0, 0);
        let mut state = self.dfa_state;
        while nin < input.len() && nout < output.len() {
            let b = input[nin];
            self.line += (b == b'\n') as u64;
            let (s, has_out) = self.dfa.get_output(state, b);
            state = s;
            if has_out {
                output[nout] = b;
                nout += 1;
            }
            nin += 1;
            if state >= self.dfa.final_field {
                break;
            }
        }
        let res = self.dfa.new_read_field_result(
            state, false, nin >= input.len(), nout >= output.len());
        self.dfa_state = state;
        (res, nin, nout)
    }

    #[inline(always)]
    fn read_field_nocopy_dfa(
        &mut self,
        input: &[u8],
    ) -> (ReadFieldNoCopyResult, usize) {
        if input.is_empty() {
            self.dfa_state = self.transition_final_dfa(self.dfa_state);
            let res = self.dfa.new_read_field_nocopy_result(
                self.dfa_state, true);
            return (res, 0);
        }
        let mut nin = 0;
        let mut state = self.dfa_state;
        while nin < input.len() {
            state = self.dfa.get(state, input[nin]);
            self.line += (input[nin] == b'\n') as u64;
            nin += 1;
            if state >= self.dfa.final_field {
                break;
            }
            if nin + 4 < input.len() {
                state = self.dfa.get(state, input[nin]);
                self.line += (input[nin] == b'\n') as u64;
                nin += 1;
                if state >= self.dfa.final_field {
                    break;
                }
                state = self.dfa.get(state, input[nin]);
                self.line += (input[nin] == b'\n') as u64;
                nin += 1;
                if state >= self.dfa.final_field {
                    break;
                }
                state = self.dfa.get(state, input[nin]);
                self.line += (input[nin] == b'\n') as u64;
                nin += 1;
                if state >= self.dfa.final_field {
                    break;
                }
                state = self.dfa.get(state, input[nin]);
                self.line += (input[nin] == b'\n') as u64;
                nin += 1;
                if state >= self.dfa.final_field {
                    break;
                }
                state = self.dfa.get(state, input[nin]);
                self.line += (input[nin] == b'\n') as u64;
                nin += 1;
                if state >= self.dfa.final_field {
                    break;
                }
            }
        }
        let res = self.dfa.new_read_field_nocopy_result(state, false);
        self.dfa_state = state;
        (res, nin)
    }

    fn transition_final_dfa(&self, state: DfaState) -> DfaState {
        if state >= self.dfa.final_record || state.is_start() {
            self.dfa.new_state_final_end()
        } else {
            self.dfa.new_state_final_record()
        }
    }

    fn build_dfa(&mut self) {
        self.dfa.classes.add(self.delimiter);
        self.dfa.classes.add(self.quote);
        if let Some(escape) = self.escape {
            self.dfa.classes.add(escape);
        }
        match self.term {
            Terminator::Any(b) => self.dfa.classes.add(b),
            Terminator::CRLF => {
                self.dfa.classes.add(b'\r');
                self.dfa.classes.add(b'\n');
            }
        }
        for &state in NFA_STATES {
            for c in (0..256).map(|c| c as u8) {
                let (mut nextstate, mut inp, mut out) =
                    (state, false, false);
                while !inp && nextstate != NfaState::End {
                    let (s, i, o) = self.transition_nfa(nextstate, c);
                    nextstate = s;
                    inp = i;
                    out = out || o;
                }
                let from = self.dfa.new_state(state);
                let to = self.dfa.new_state(nextstate);
                self.dfa.set(from, c, to, out);
            }
        }
        self.dfa_state = self.dfa.new_state(NfaState::StartRecord);
        self.dfa.finish();
    }

    // The NFA implementation follows. The transition_final_nfa and
    // transition_nfa methods are required for the DFA to operate. The
    // rest are included for completeness (and debugging). Note that this
    // NFA implementation is included in most of the CSV parser tests below.

    #[inline(always)]
    fn read_record_nfa(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        ends: &mut [usize],
    ) -> (ReadRecordResult, usize, usize, usize) {
        if input.is_empty() {
            let s = self.transition_final_nfa(self.nfa_state);
            let res = ReadRecordResult::from_nfa(s, false, false, false);
            return match res {
                ReadRecordResult::Record => {
                    if ends.is_empty() {
                        return (ReadRecordResult::OutputEndsFull, 0, 0, 0);
                    }
                    self.nfa_state = s;
                    ends[0] = self.output_pos;
                    self.output_pos = 0;
                    (res, 0, 0, 1)
                }
                _ => {
                    self.nfa_state = s;
                    (res, 0, 0, 0)
                }
            };
        }
        if output.is_empty() {
            return (ReadRecordResult::OutputFull, 0, 0, 0);
        }
        if ends.is_empty() {
            return (ReadRecordResult::OutputEndsFull, 0, 0, 0);
        }
        let (mut nin, mut nout, mut nend) = (0, self.output_pos, 0);
        let mut state = self.nfa_state;
        while nin < input.len() && nout < output.len() && nend < ends.len() {
            let (s, i, o) = self.transition_nfa(state, input[nin]);
            if o {
                output[nout] = input[nin];
                nout += 1;
            }
            if i {
                nin += 1;
            }
            state = s;
            if state.is_field_final() {
                ends[nend] = nout;
                nend += 1;
                if state != NfaState::EndFieldDelim {
                    break;
                }
            }
        }
        let res = ReadRecordResult::from_nfa(
            state,
            nin >= input.len(),
            nout >= output.len(),
            nend >= ends.len());
        self.nfa_state = state;
        self.output_pos = if res.is_record() { 0 } else { nout };
        (res, nin, nout, nend)
    }

    #[inline(always)]
    fn read_field_nfa(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> (ReadFieldResult, usize, usize) {
        if input.is_empty() {
            self.nfa_state = self.transition_final_nfa(self.nfa_state);
            let res = ReadFieldResult::from_nfa(self.nfa_state, false, false);
            return (res, 0, 0);
        }
        if output.is_empty() {
            // If the output buffer is empty, then we can never make progress,
            // so just quit now.
            return (ReadFieldResult::OutputFull, 0, 0);
        }
        let (mut nin, mut nout) = (0, 0);
        let mut state = self.nfa_state;
        while nin < input.len() && nout < output.len() {
            let (s, i, o) = self.transition_nfa(state, input[nin]);
            if o {
                output[nout] = input[nin];
                nout += 1;
            }
            if i {
                nin += 1;
            }
            state = s;
            if state.is_field_final() {
                break;
            }
        }
        let res = ReadFieldResult::from_nfa(
            state, nin >= input.len(), nout >= output.len());
        self.nfa_state = state;
        (res, nin, nout)
    }

    #[inline(always)]
    fn transition_final_nfa(&self, state: NfaState) -> NfaState {
        use self::NfaState::*;
        match state {
            End
            | StartRecord
            | EndRecord
            | CRLF => End,
            StartField
            | EndFieldDelim
            | EndFieldTerm
            | InField
            | InQuotedField
            | InEscapedQuote
            | InDoubleEscapedQuote
            | InRecordTerm => EndRecord,
        }
    }

    #[inline(always)]
    fn transition_nfa(
        &self,
        state: NfaState,
        c: u8,
    ) -> (NfaState, bool, bool) {
        use self::NfaState::*;
        match state {
            End => (End, false, false),
            StartRecord => {
                if self.term == c {
                    (StartRecord, true, false)
                } else {
                    (StartField, false, false)
                }
            }
            EndRecord => {
                (StartRecord, false, false)
            }
            StartField => {
                if self.quote == c {
                    (InQuotedField, true, false)
                } else if self.delimiter == c {
                    (EndFieldDelim, true, false)
                } else if self.term == c {
                    (EndFieldTerm, false, false)
                } else {
                    (InField, true, true)
                }
            }
            EndFieldDelim => {
                (StartField, false, false)
            }
            EndFieldTerm => {
                (InRecordTerm, false, false)
            }
            InField => {
                if self.delimiter == c {
                    (EndFieldDelim, true, false)
                } else if self.term == c {
                    (EndFieldTerm, false, false)
                } else {
                    (InField, true, true)
                }
            }
            InQuotedField => {
                if self.quote == c {
                    (InDoubleEscapedQuote, true, false)
                } else if self.escape == Some(c) {
                    (InEscapedQuote, true, false)
                } else {
                    (InQuotedField, true, true)
                }
            }
            InEscapedQuote => {
                (InQuotedField, true, true)
            }
            InDoubleEscapedQuote => {
                if self.double_quote && self.quote == c {
                    (InQuotedField, true, true)
                } else if self.delimiter == c {
                    (EndFieldDelim, true, false)
                } else if self.term == c {
                    (EndFieldTerm, false, false)
                } else {
                    (InField, true, true)
                }
            }
            InRecordTerm => {
                if self.term.is_crlf() && b'\r' == c {
                    (CRLF, true, false)
                } else {
                    (EndRecord, true, false)
                }
            }
            CRLF => {
                if b'\n' == c {
                    (StartRecord, true, false)
                } else {
                    (StartRecord, false, false)
                }
            }
        }
    }
}

/// The number of slots in the DFA transition table.
///
/// This number is computed by multiplying the maximum number of transition
/// classes (6) by the total number of NFA states that are used in the DFA
/// (9).
///
/// The number of transition classes is determined by an equivalence class of
/// bytes, where every byte in the same equivalence classes is
/// indistinguishable from any other byte with respect to the DFA. For example,
/// if neither `a` nor `b` are specifed as a delimiter/quote/terminator/escape,
/// then the DFA will never discriminate between `a` or `b`, so they can
/// effectively be treated as identical. This reduces storage space
/// substantially.
///
/// The total number of NFA states (12) is greater than the total number of
/// NFA states that are in the DFA. In particular, any NFA state that can only
/// be reached by epsilon transitions will never have explicit usage in the
/// DFA.
const TRANS_SIZE: usize = 54;

/// The number of possible transition classes. (See the comment on `TRANS_SIZE`
/// for more details.)
const CLASS_SIZE: usize = 256;

struct Dfa {
    trans: [DfaState; TRANS_SIZE],
    has_output: [bool; TRANS_SIZE],
    classes: DfaClasses,
    final_field: DfaState,
    final_record: DfaState,
}

impl Dfa {
    fn new() -> Dfa {
        Dfa {
            trans: [DfaState(0); TRANS_SIZE],
            has_output: [false; TRANS_SIZE],
            classes: DfaClasses::new(),
            final_field: DfaState(0),
            final_record: DfaState(0),
        }
    }

    fn new_state(&self, nfa_state: NfaState) -> DfaState {
        let nclasses = self.classes.num_classes() as u8;
        let idx = (nfa_state as u8).checked_mul(nclasses).unwrap();
        DfaState(idx)
    }

    fn new_state_final_end(&self) -> DfaState {
        self.new_state(NfaState::StartRecord)
    }

    fn new_state_final_record(&self) -> DfaState {
        self.new_state(NfaState::EndRecord)
    }

    fn get(&self, state: DfaState, c: u8) -> DfaState {
        let cls = self.classes.classes[c as usize];
        let idx = state.0 as usize + cls as usize;
        self.trans[idx]
    }

    fn get_output(&self, state: DfaState, c: u8) -> (DfaState, bool) {
        let cls = self.classes.classes[c as usize];
        let idx = state.0 as usize + cls as usize;
        (self.trans[idx], self.has_output[idx])
    }

    fn set(&mut self, from: DfaState, c: u8, to: DfaState, output: bool) {
        let cls = self.classes.classes[c as usize];
        self.trans[from.0 as usize + cls as usize] = to;
        self.has_output[from.0 as usize + cls as usize] = output;
    }

    fn finish(&mut self) {
        self.final_field = self.new_state(NfaState::EndFieldDelim);
        self.final_record = self.new_state(NfaState::EndRecord);
    }

    fn new_read_field_result(
        &self,
        state: DfaState,
        is_final_trans: bool,
        inpdone: bool,
        outdone: bool,
    ) -> ReadFieldResult {
        if state >= self.final_record {
            ReadFieldResult::Field { record_end: true }
        } else if state == self.final_field {
            ReadFieldResult::Field { record_end: false }
        } else if is_final_trans && state.is_start() {
            ReadFieldResult::End
        } else {
            debug_assert!(state < self.final_field);
            if !inpdone && outdone {
                ReadFieldResult::OutputFull
            } else {
                ReadFieldResult::InputEmpty
            }
        }
    }

    fn new_read_field_nocopy_result(
        &self,
        state: DfaState,
        is_final_trans: bool,
    ) -> ReadFieldNoCopyResult {
        if state >= self.final_record {
            ReadFieldNoCopyResult::Field { record_end: true }
        } else if state == self.final_field {
            ReadFieldNoCopyResult::Field { record_end: false }
        } else if is_final_trans && state.is_start() {
            ReadFieldNoCopyResult::End
        } else {
            debug_assert!(state < self.final_field);
            ReadFieldNoCopyResult::InputEmpty
        }
    }

    fn new_read_record_result(
        &self,
        state: DfaState,
        is_final_trans: bool,
        inpdone: bool,
        outdone: bool,
        endsdone: bool,
    ) -> ReadRecordResult {
        if state >= self.final_record {
            ReadRecordResult::Record
        } else if is_final_trans && state.is_start() {
            ReadRecordResult::End
        } else {
            debug_assert!(state < self.final_record);
            if !inpdone && outdone {
                ReadRecordResult::OutputFull
            } else if !inpdone && endsdone {
                ReadRecordResult::OutputEndsFull
            } else {
                ReadRecordResult::InputEmpty
            }
        }
    }

    fn new_read_record_nocopy_result(
        &self,
        state: DfaState,
        is_final_trans: bool,
    ) -> ReadRecordNoCopyResult {
        if state >= self.final_record {
            ReadRecordNoCopyResult::Record
        } else if is_final_trans && state.is_start() {
            ReadRecordNoCopyResult::End
        } else {
            debug_assert!(state < self.final_record);
            ReadRecordNoCopyResult::InputEmpty
        }
    }
}

struct DfaClasses {
    classes: [u8; CLASS_SIZE],
    next_class: usize,
}

impl DfaClasses {
    fn new() -> DfaClasses {
        DfaClasses { classes: [0; CLASS_SIZE], next_class: 1 }
    }

    fn add(&mut self, b: u8) {
        if self.next_class > CLASS_SIZE {
            panic!("added too many classes")
        }
        self.classes[b as usize] = self.next_class as u8;
        self.next_class = self.next_class + 1;
    }

    fn num_classes(&self) -> usize {
        self.next_class as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DfaState(u8);

impl DfaState {
    fn start() -> DfaState {
        DfaState(0)
    }

    fn is_start(&self) -> bool {
        self.0 == 0
    }
}

impl fmt::Debug for Dfa {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Dfa(N/A)")
    }
}

impl fmt::Debug for DfaClasses {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "DfaClasses {{ classes: N/A, next_class: {:?} }}",
            self.next_class)
    }
}

impl Clone for Dfa {
    fn clone(&self) -> Dfa {
        let mut dfa = Dfa::new();
        dfa.trans.copy_from_slice(&self.trans);
        dfa
    }
}

impl Clone for DfaClasses {
    fn clone(&self) -> DfaClasses {
        let mut x = DfaClasses::new();
        x.classes.copy_from_slice(&self.classes);
        x
    }
}

#[cfg(test)]
mod tests {
    use core::str;

    use arrayvec::{ArrayString, ArrayVec};

    use super::{
        Reader, ReaderBuilder, ReadFieldResult, Terminator,
    };

    type Csv = ArrayVec<[Row; 10]>;
    type Row = ArrayVec<[Field; 10]>;
    type Field = ArrayString<[u8; 10]>;

    // OMG I HATE BYTE STRING LITERALS SO MUCH.
    fn b(s: &str) -> &[u8] { s.as_bytes() }

    macro_rules! csv {
        ($([$($field:expr),*]),*) => {{
            #[allow(unused_mut)]
            fn x() -> Csv {
                let mut csv = Csv::new();
                $(
                    let mut row = Row::new();
                    $(
                        row.push(Field::from($field).unwrap());
                    )*
                    csv.push(row);
                )*
                csv
            }
            x()
        }}
    }

    macro_rules! parses_to {
        ($name:ident, $data:expr, $expected:expr) => {
            parses_to!($name, $data, $expected, |builder| builder);
        };
        ($name:ident, $data:expr, $expected:expr, $config:expr) => {
            #[test]
            fn $name() {
                let mut builder = ReaderBuilder::new();
                $config(&mut builder);
                let mut rdr = builder.build();
                let got = parse_by_field(&mut rdr, $data);
                let expected = $expected;
                assert_eq!(expected, got, "dfa by field");

                let mut builder = ReaderBuilder::new();
                builder.nfa(true);
                $config(&mut builder);
                let mut rdr = builder.build();
                let got = parse_by_field(&mut rdr, $data);
                let expected = $expected;
                assert_eq!(expected, got, "nfa by field");

                let mut builder = ReaderBuilder::new();
                $config(&mut builder);
                let mut rdr = builder.build();
                let got = parse_by_record(&mut rdr, $data);
                let expected = $expected;
                assert_eq!(expected, got, "dfa by record");

                let mut builder = ReaderBuilder::new();
                builder.nfa(true);
                $config(&mut builder);
                let mut rdr = builder.build();
                let got = parse_by_record(&mut rdr, $data);
                let expected = $expected;
                assert_eq!(expected, got, "nfa by record");
            }
        };
    }

    fn parse_by_field(rdr: &mut Reader, data: &str) -> Csv {
        let mut data = data.as_bytes();
        let mut field = [0u8; 10];
        let mut csv = Csv::new();
        let mut row = Row::new();
        let mut outpos = 0;
        loop {
            let (res, nin, nout) = rdr.read_field(data, &mut field[outpos..]);
            data = &data[nin..];
            outpos += nout;

            match res {
                ReadFieldResult::InputEmpty => {
                    if !data.is_empty() {
                        panic!("missing input data")
                    }
                }
                ReadFieldResult::OutputFull => panic!("field too large"),
                ReadFieldResult::Field { record_end } => {
                    let s = str::from_utf8(&field[..outpos]).unwrap();
                    row.push(Field::from(s).unwrap());
                    outpos = 0;
                    if record_end {
                        csv.push(row);
                        row = Row::new();
                    }
                }
                ReadFieldResult::End => {
                    return csv;
                }
            }
        }
    }

    fn parse_by_record(rdr: &mut Reader, data: &str) -> Csv {
        use ReadRecordResult::*;

        let mut data = data.as_bytes();
        let mut record = [0; 1024];
        let mut ends = [0; 10];

        let mut csv = Csv::new();
        let (mut outpos, mut endpos) = (0, 0);
        loop {
            let (res, nin, nout, nend) = rdr.read_record(
                data, &mut record[outpos..], &mut ends[endpos..]);
            data = &data[nin..];
            outpos += nout;
            endpos += nend;

            match res {
                InputEmpty => {
                    if !data.is_empty() {
                        panic!("missing input data")
                    }
                }
                OutputFull => panic!("record too large (out buffer)"),
                OutputEndsFull => panic!("record too large (end buffer)"),
                Record => {
                    let s = str::from_utf8(&record[..outpos]).unwrap();
                    let mut start = 0;
                    let mut row = Row::new();
                    for &end in &ends[..endpos] {
                        row.push(Field::from(&s[start..end]).unwrap());
                        start = end;
                    }
                    csv.push(row);
                    outpos = 0;
                    endpos = 0;
                }
                End => return csv,
            }
        }
    }

    parses_to!(one_row_one_field, "a", csv![["a"]]);
    parses_to!(one_row_many_fields, "a,b,c", csv![["a", "b", "c"]]);
    parses_to!(one_row_trailing_comma, "a,b,", csv![["a", "b", ""]]);
    parses_to!(one_row_one_field_lf, "a\n", csv![["a"]]);
    parses_to!(one_row_many_fields_lf, "a,b,c\n", csv![["a", "b", "c"]]);
    parses_to!(one_row_trailing_comma_lf, "a,b,\n", csv![["a", "b", ""]]);
    parses_to!(one_row_one_field_crlf, "a\r\n", csv![["a"]]);
    parses_to!(one_row_many_fields_crlf, "a,b,c\r\n", csv![["a", "b", "c"]]);
    parses_to!(one_row_trailing_comma_crlf, "a,b,\r\n", csv![["a", "b", ""]]);
    parses_to!(one_row_one_field_cr, "a\r", csv![["a"]]);
    parses_to!(one_row_many_fields_cr, "a,b,c\r", csv![["a", "b", "c"]]);
    parses_to!(one_row_trailing_comma_cr, "a,b,\r", csv![["a", "b", ""]]);

    parses_to!(many_rows_one_field, "a\nb", csv![["a"], ["b"]]);
    parses_to!(
        many_rows_many_fields,
       "a,b,c\nx,y,z", csv![["a", "b", "c"], ["x", "y", "z"]]);
    parses_to!(
        many_rows_trailing_comma,
        "a,b,\nx,y,", csv![["a", "b", ""], ["x", "y", ""]]);
    parses_to!(many_rows_one_field_lf, "a\nb\n", csv![["a"], ["b"]]);
    parses_to!(
        many_rows_many_fields_lf,
        "a,b,c\nx,y,z\n", csv![["a", "b", "c"], ["x", "y", "z"]]);
    parses_to!(
        many_rows_trailing_comma_lf,
        "a,b,\nx,y,\n", csv![["a", "b", ""], ["x", "y", ""]]);
    parses_to!(many_rows_one_field_crlf, "a\r\nb\r\n", csv![["a"], ["b"]]);
    parses_to!(
        many_rows_many_fields_crlf,
        "a,b,c\r\nx,y,z\r\n",
        csv![["a", "b", "c"], ["x", "y", "z"]]);
    parses_to!(many_rows_trailing_comma_crlf,
               "a,b,\r\nx,y,\r\n", csv![["a", "b", ""], ["x", "y", ""]]);
    parses_to!(many_rows_one_field_cr, "a\rb\r", csv![["a"], ["b"]]);
    parses_to!(
        many_rows_many_fields_cr,
        "a,b,c\rx,y,z\r", csv![["a", "b", "c"], ["x", "y", "z"]]);
    parses_to!(
        many_rows_trailing_comma_cr,
        "a,b,\rx,y,\r", csv![["a", "b", ""], ["x", "y", ""]]);

    parses_to!(
        trailing_lines_no_record,
        "\n\n\na,b,c\nx,y,z\n\n\n",
        csv![["a", "b", "c"], ["x", "y", "z"]]);
    parses_to!(
        trailing_lines_no_record_cr,
        "\r\r\ra,b,c\rx,y,z\r\r\r",
        csv![["a", "b", "c"], ["x", "y", "z"]]);
    parses_to!(
        trailing_lines_no_record_crlf,
        "\r\n\r\n\r\na,b,c\r\nx,y,z\r\n\r\n\r\n",
        csv![["a", "b", "c"], ["x", "y", "z"]]);

    parses_to!(empty, "", csv![]);
    parses_to!(empty_lines, "\n\n\n\n", csv![]);
    parses_to!(
        empty_lines_interspersed, "\n\na,b\n\n\nx,y\n\n\nm,n\n",
        csv![["a", "b"], ["x", "y"], ["m", "n"]]);
    parses_to!(empty_lines_crlf, "\r\n\r\n\r\n\r\n", csv![]);
    parses_to!(
        empty_lines_interspersed_crlf,
        "\r\n\r\na,b\r\n\r\n\r\nx,y\r\n\r\n\r\nm,n\r\n",
        csv![["a", "b"], ["x", "y"], ["m", "n"]]);
    parses_to!(empty_lines_mixed, "\r\n\n\r\n\n", csv![]);
    parses_to!(
        empty_lines_interspersed_mixed,
        "\n\r\na,b\r\n\n\r\nx,y\r\n\n\r\nm,n\r\n",
        csv![["a", "b"], ["x", "y"], ["m", "n"]]);
    parses_to!(empty_lines_cr, "\r\r\r\r", csv![]);
    parses_to!(
        empty_lines_interspersed_cr, "\r\ra,b\r\r\rx,y\r\r\rm,n\r",
        csv![["a", "b"], ["x", "y"], ["m", "n"]]);

    parses_to!(
        term_weird, "zza,bzc,dzz",
        csv![["a", "b"], ["c", "d"]],
        |b: &mut ReaderBuilder| { b.terminator(Terminator::Any(b'z')); });

    parses_to!(
        ascii_delimited, "a\x1fb\x1ec\x1fd",
        csv![["a", "b"], ["c", "d"]],
        |b: &mut ReaderBuilder| { b.ascii(); });

    parses_to!(quote_empty, "\"\"", csv![[""]]);
    parses_to!(quote_lf, "\"\"\n", csv![[""]]);
    parses_to!(quote_space, "\" \"", csv![[" "]]);
    parses_to!(quote_inner_space, "\" a \"", csv![[" a "]]);
    parses_to!(quote_outer_space, "  \"a\"  ", csv![["  \"a\"  "]]);

    parses_to!(quote_change, "zaz", csv![["a"]],
               |b: &mut ReaderBuilder| { b.quote(b'z'); });

    // This one is pretty hokey.
    // I don't really know what the "right" behavior is.
    parses_to!(quote_delimiter, ",a,,b", csv![["a,b"]],
               |b: &mut ReaderBuilder| { b.quote(b','); });

    parses_to!(quote_no_escapes, r#""a\"b""#, csv![[r#"a\b""#]]);
    parses_to!(quote_escapes_no_double, r#""a""b""#, csv![[r#"a"b""#]],
               |b: &mut ReaderBuilder| { b.double_quote(false); });
    parses_to!(quote_escapes, r#""a\"b""#, csv![[r#"a"b"#]],
               |b: &mut ReaderBuilder| { b.escape(Some(b'\\')); });
    parses_to!(quote_escapes_change, r#""az"b""#, csv![[r#"a"b"#]],
               |b: &mut ReaderBuilder| { b.escape(Some(b'z')); });

    parses_to!(delimiter_tabs, "a\tb", csv![["a", "b"]],
               |b: &mut ReaderBuilder| { b.delimiter(b'\t'); });
    parses_to!(delimiter_weird, "azb", csv![["a", "b"]],
               |b: &mut ReaderBuilder| { b.delimiter(b'z'); });

    parses_to!(extra_record_crlf_1, "foo\n1\n", csv![["foo"], ["1"]]);
    parses_to!(extra_record_crlf_2, "foo\r\n1\r\n", csv![["foo"], ["1"]]);

    macro_rules! assert_read {
        (
            $rdr:expr, $input:expr, $output:expr,
            $expect_in:expr, $expect_out:expr, $expect_res:expr
        ) => {{
            let (res, nin, nout) = $rdr.read_field($input, $output);
            assert_eq!($expect_in, nin);
            assert_eq!($expect_out, nout);
            assert_eq!($expect_res, res);
        }};
    }

    // This tests that feeding a new reader with an empty buffer sends us
    // straight to End.
    #[test]
    fn stream_empty() {
        use ReadFieldResult::*;

        let mut rdr = Reader::new();
        assert_read!(rdr, &[], &mut [], 0, 0, End);
    }

    // Test that a single space is treated as a single field.
    #[test]
    fn stream_space() {
        use ReadFieldResult::*;

        let mut rdr = Reader::new();
        assert_read!(rdr, b(" "), &mut [0], 1, 1, InputEmpty);
        assert_read!(rdr, &[], &mut [0], 0, 0, Field { record_end: true });
        assert_read!(rdr, &[], &mut [0], 0, 0, End);
    }

    // Test that a single comma ...
    #[test]
    fn stream_comma() {
        use ReadFieldResult::*;

        let mut rdr = Reader::new();
        assert_read!(rdr, b(","), &mut [0], 1, 0, Field { record_end: false });
        assert_read!(rdr, &[], &mut [0], 0, 0, Field { record_end: true });
        assert_read!(rdr, &[], &mut [0], 0, 0, End);
    }

    // Test that we can read a single large field in multiple output
    // buffers.
    #[test]
    fn stream_output_chunks() {
        use ReadFieldResult::*;

        let mut inp = b("fooquux");
        let mut out = &mut [0; 2];
        let mut rdr = Reader::new();

        assert_read!(rdr, inp, out, 2, 2, OutputFull);
        assert_eq!(out, b("fo"));
        inp = &inp[2..];

        assert_read!(rdr, inp, out, 2, 2, OutputFull);
        assert_eq!(out, b("oq"));
        inp = &inp[2..];

        assert_read!(rdr, inp, out, 2, 2, OutputFull);
        assert_eq!(out, b("uu"));
        inp = &inp[2..];

        assert_read!(rdr, inp, out, 1, 1, InputEmpty);
        assert_eq!(&out[..1], b("x"));
        inp = &inp[1..];
        assert!(inp.is_empty());

        assert_read!(rdr, &[], out, 0, 0, Field { record_end: true });
        assert_read!(rdr, inp, out, 0, 0, End);
    }

    // Test that we can read a single large field across multiple input
    // buffers.
    #[test]
    fn stream_input_chunks() {
        use ReadFieldResult::*;

        let mut out = &mut [0; 10];
        let mut rdr = Reader::new();

        assert_read!(rdr, b("fo"), out, 2, 2, InputEmpty);
        assert_eq!(&out[..2], b("fo"));

        assert_read!(rdr, b("oq"), &mut out[2..], 2, 2, InputEmpty);
        assert_eq!(&out[..4], b("fooq"));

        assert_read!(rdr, b("uu"), &mut out[4..], 2, 2, InputEmpty);
        assert_eq!(&out[..6], b("fooquu"));

        assert_read!(rdr, b("x"), &mut out[6..], 1, 1, InputEmpty);
        assert_eq!(&out[..7], b("fooquux"));

        assert_read!(rdr, &[], out, 0, 0, Field { record_end: true });
        assert_read!(rdr, &[], out, 0, 0, End);
    }

    // Test we can read doubled quotes correctly in a stream.
    #[test]
    fn stream_doubled_quotes() {
        use ReadFieldResult::*;

        let mut out = &mut [0; 10];
        let mut rdr = Reader::new();

        assert_read!(rdr, b("\"fo\""), out, 4, 2, InputEmpty);
        assert_eq!(&out[..2], b("fo"));

        assert_read!(rdr, b("\"o"), &mut out[2..], 2, 2, InputEmpty);
        assert_eq!(&out[..4], b("fo\"o"));

        assert_read!(rdr, &[], out, 0, 0, Field { record_end: true });
        assert_read!(rdr, &[], out, 0, 0, End);
    }

    // Test we can read escaped quotes correctly in a stream.
    #[test]
    fn stream_escaped_quotes() {
        use ReadFieldResult::*;

        let mut out = &mut [0; 10];
        let mut builder = ReaderBuilder::new();
        let mut rdr = builder.escape(Some(b'\\')).build();

        assert_read!(rdr, b("\"fo\\"), out, 4, 2, InputEmpty);
        assert_eq!(&out[..2], b("fo"));

        assert_read!(rdr, b("\"o"), &mut out[2..], 2, 2, InputEmpty);
        assert_eq!(&out[..4], b("fo\"o"));

        assert_read!(rdr, &[], out, 0, 0, Field { record_end: true });
        assert_read!(rdr, &[], out, 0, 0, End);
    }

    // Test that empty output buffers don't wreak havoc.
    #[test]
    fn stream_empty_output() {
        use ReadFieldResult::*;

        let mut out = &mut [0; 10];
        let mut rdr = Reader::new();

        assert_read!(
            rdr, b("foo,bar"), out, 4, 3, Field { record_end: false });
        assert_eq!(&out[..3], b("foo"));

        assert_read!(rdr, b("bar"), &mut [], 0, 0, OutputFull);

        assert_read!(rdr, b("bar"), out, 3, 3, InputEmpty);
        assert_eq!(&out[..3], b("bar"));

        assert_read!(rdr, &[], out, 0, 0, Field { record_end: true });
        assert_read!(rdr, &[], out, 0, 0, End);
    }

    // Test that we can reset the parser mid-stream and count on it to do
    // the right thing.
    #[test]
    fn reset_works() {
        use ReadFieldResult::*;

        let mut out = &mut [0; 10];
        let mut rdr = Reader::new();

        assert_read!(rdr, b("\"foo"), out, 4, 3, InputEmpty);
        assert_eq!(&out[..3], b("foo"));

        // Without reseting the parser state, the reader will remember that
        // we're in a quoted field, and therefore interpret the leading double
        // quotes below as a single quote and the trailing quote as a matching
        // terminator. With the reset, however, the parser forgets the quoted
        // field and treats the leading double quotes as a syntax quirk and
        // drops them, in addition to hanging on to the trailing unmatched
        // quote. (Matches Python's behavior.)
        rdr.reset();

        assert_read!(rdr, b("\"\"bar\""), out, 6, 4, InputEmpty);
        assert_eq!(&out[..4], b("bar\""));
    }

    // Test the line number reporting is correct.
    #[test]
    fn line_numbers() {
        use ReadFieldResult::*;

        let mut out = &mut [0; 10];
        let mut rdr = Reader::new();

        assert_eq!(1, rdr.line());

        assert_read!(rdr, b("\n\n\n\n"), out, 4, 0, InputEmpty);
        assert_eq!(5, rdr.line());

        assert_read!(rdr, b("foo,"), out, 4, 3, Field { record_end: false });
        assert_eq!(5, rdr.line());

        assert_read!(rdr, b("bar\n"), out, 4, 3, Field { record_end: true });
        assert_eq!(6, rdr.line());

        assert_read!(rdr, &[], &mut [0], 0, 0, End);
        assert_eq!(6, rdr.line());
    }

    macro_rules! assert_read_record {
        (
            $rdr:expr, $input:expr, $output:expr, $ends:expr,
            $expect_in:expr, $expect_out:expr,
            $expect_end:expr, $expect_res:expr
        ) => {{
            let (res, nin, nout, nend) =
                $rdr.read_record($input, $output, $ends);
            assert_eq!($expect_res, res, "result");
            assert_eq!($expect_in, nin, "input");
            assert_eq!($expect_out, nout, "output");
            assert_eq!($expect_end, nend, "ends");
        }};
    }

    // Test that we can incrementally read a record.
    #[test]
    fn stream_record() {
        use ReadRecordResult::*;

        let mut inp = b("foo,bar\nbaz");
        let mut out = &mut [0; 1024];
        let mut ends = &mut [0; 10];
        let mut rdr = Reader::new();

        assert_read_record!(rdr, &inp, out, ends, 8, 6, 2, Record);
        assert_eq!(ends[0], 3);
        assert_eq!(ends[1], 6);
        inp = &inp[8..];

        assert_read_record!(rdr, &inp, out, ends, 3, 3, 0, InputEmpty);
        inp = &inp[3..];

        assert_read_record!(rdr, &inp, out, ends, 0, 0, 1, Record);
        assert_eq!(ends[0], 3);

        assert_read_record!(rdr, &inp, out, ends, 0, 0, 0, End);
    }

    // Test that if our output ends are full during the last read that
    // we get an appropriate state returned.
    #[test]
    fn stream_record_last_end_output_full() {
        use ReadRecordResult::*;

        let mut inp = b("foo,bar\nbaz");
        let mut out = &mut [0; 1024];
        let mut ends = &mut [0; 10];
        let mut rdr = Reader::new();

        assert_read_record!(rdr, &inp, out, ends, 8, 6, 2, Record);
        assert_eq!(ends[0], 3);
        assert_eq!(ends[1], 6);
        inp = &inp[8..];

        assert_read_record!(rdr, &inp, out, ends, 3, 3, 0, InputEmpty);
        inp = &inp[3..];

        assert_read_record!(rdr, &inp, out, &mut [], 0, 0, 0, OutputEndsFull);
        assert_read_record!(rdr, &inp, out, ends, 0, 0, 1, Record);
        assert_eq!(ends[0], 3);

        assert_read_record!(rdr, &inp, out, ends, 0, 0, 0, End);
    }
}
