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
pub enum ReadResult {
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

impl ReadResult {
    fn from_nfa(state: &NfaState, inpdone: bool, outdone: bool) -> ReadResult {
        match *state {
            NfaState::End => ReadResult::End,
            NfaState::EndRecord | NfaState::CRLF => {
                ReadResult::Field { record_end: true }
            }
            NfaState::EndFieldDelim => {
                ReadResult::Field { record_end: false }
            }
            _ => {
                assert!(!state.is_final());
                if !inpdone && outdone {
                    ReadResult::OutputFull
                } else {
                    ReadResult::InputEmpty
                }
            }
        }
    }
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
    fn is_final(&self) -> bool {
        match *self {
            NfaState::End
            | NfaState::EndRecord
            | NfaState::CRLF
            | NfaState::EndFieldDelim => true,
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
    /// a `ReadResult`, tells the caller what to do next. For example,
    /// if the entire input was read or if the output buffer was filled
    /// before a full field had been read, then `ReadResult::InputEmpty` or
    /// `ReadResult::OutputFull` is returned, respectively. See the
    /// documentation for `ReadResult` for more details.
    ///
    /// The second two values returned correspond to the number of bytes
    /// read from `input` and `output`, respectively.
    ///
    /// # Termination
    ///
    /// This reader interprets an empty `input` buffer as an indication that
    /// there is no CSV data left to read. Namely, when the caller has
    /// exhausted all CSV data, the caller should continue to call `read` with
    /// an empty input buffer until `ReadResult::End` is returned.
    ///
    /// # Errors
    ///
    /// This CSV reader can never return an error. Instead, it prefers *a*
    /// parse over *no* parse.
    pub fn read(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> (ReadResult, usize, usize) {
        if self.use_nfa {
            self.read_nfa(input, output)
        } else {
            self.read_dfa(input, output)
        }
    }

    #[inline(always)]
    fn read_dfa(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> (ReadResult, usize, usize) {
        if input.is_empty() {
            self.dfa_state = self.dfa_final_transition(self.dfa_state);
            let res = self.dfa.new_read_result(
                &self.dfa_state, true, false, false);
            (res, 0, 0)
        } else {
            let (res, state, nin, nout) =
                if self.copy {
                    self.consume_and_copy_dfa(self.dfa_state, input, output)
                } else {
                    self.consume_dfa(self.dfa_state, input)
                };
            self.dfa_state = state;
            (res, nin, nout)
        }
    }

    #[inline(always)]
    fn consume_and_copy_dfa(
        &self,
        mut state: DfaState,
        input: &[u8],
        output: &mut [u8],
    ) -> (ReadResult, DfaState, usize, usize) {
        debug_assert!(!input.is_empty());
        if output.is_empty() {
            // If the output buffer is empty, then we can never make progress,
            // so just quit now.
            return (ReadResult::OutputFull, state, 0, 0);
        }
        let (mut nin, mut nout) = (0, 0);
        while nin < input.len() && nout < output.len() {
            let (s, has_out) = self.dfa.get_output(state, input[nin]);
            state = s;
            if has_out {
                output[nout] = input[nin];
                nout += 1;
            }
            nin += 1;
            if state >= self.dfa.final_field {
                break;
            }
        }
        let res = self.dfa.new_read_result(
            &state, false, nin >= input.len(), nout >= output.len());
        (res, state, nin, nout)
    }

    #[inline(always)]
    fn consume_dfa(
        &self,
        mut state: DfaState,
        input: &[u8],
    ) -> (ReadResult, DfaState, usize, usize) {
        let mut nin = 0;
        while nin < input.len() {
            state = self.dfa.get(state, input[nin]);
            nin += 1;
            if state >= self.dfa.final_field {
                break;
            }
        }
        let res = self.dfa.new_read_result(
            &state, false, nin >= input.len(), false);
        (res, state, nin, 0)
    }

    #[inline(always)]
    fn dfa_final_transition(&self, state: DfaState) -> DfaState {
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
                    let (s, i, o) = self.nfa_transition(nextstate, c);
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

    // The NFA implementation follows. The nfa_final_transition and
    // nfa_transition methods are required for the DFA to operate. The
    // rest are included for completeness (and debugging).

    #[inline(always)]
    fn read_nfa(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> (ReadResult, usize, usize) {
        if input.is_empty() {
            self.nfa_state = self.nfa_final_transition(self.nfa_state);
            (ReadResult::from_nfa(&self.nfa_state, false, false), 0, 0)
        } else {
            let (res, state, nin, nout) =
                if self.copy {
                    self.consume_and_copy(self.nfa_state, input, output)
                } else {
                    self.consume(self.nfa_state, input)
                };
            self.nfa_state = state;
            (res, nin, nout)
        }
    }

    #[inline(always)]
    fn consume_and_copy(
        &self,
        mut state: NfaState,
        input: &[u8],
        output: &mut [u8],
    ) -> (ReadResult, NfaState, usize, usize) {
        debug_assert!(!input.is_empty());
        if output.is_empty() {
            // If the output buffer is empty, then we can never make progress,
            // so just quit now.
            return (ReadResult::OutputFull, state, 0, 0);
        }
        let (mut nin, mut nout) = (0, 0);
        while nin < input.len() && nout < output.len() {
            let (s, i, o) = self.nfa_transition(state, input[nin]);
            if o {
                output[nout] = input[nin];
                nout += 1;
            }
            if i {
                nin += 1;
            }
            state = s;
            if state.is_final() {
                break;
            }
        }
        let res = ReadResult::from_nfa(
            &state, nin >= input.len(), nout >= output.len());
        (res, state, nin, nout)
    }

    #[inline(always)]
    fn consume(
        &self,
        mut state: NfaState,
        input: &[u8],
    ) -> (ReadResult, NfaState, usize, usize) {
        let mut nin = 0;
        while nin < input.len() {
            let (s, i, _) = self.nfa_transition(state, input[nin]);
            if i {
                nin += 1;
            }
            state = s;
            if state.is_final() {
                break;
            }
        }
        let res = ReadResult::from_nfa(&state, nin >= input.len(), false);
        (res, state, nin, 0)
    }

    #[inline(always)]
    fn nfa_final_transition(&self, state: NfaState) -> NfaState {
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
    fn nfa_transition(
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

    fn new_read_result(
        &self,
        state: &DfaState,
        is_final_trans: bool,
        inpdone: bool,
        outdone: bool,
    ) -> ReadResult {
        if state >= &self.final_record {
            ReadResult::Field { record_end: true }
        } else if state == &self.final_field {
            ReadResult::Field { record_end: false }
        } else if is_final_trans && state.is_start() {
            ReadResult::End
        } else {
            debug_assert!(state < &self.final_field);
            if !inpdone && outdone {
                ReadResult::OutputFull
            } else {
                ReadResult::InputEmpty
            }
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

    use super::{Reader, ReaderBuilder, ReadResult, Terminator};

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
                let got = parse(&mut rdr, $data);
                let expected = $expected;
                assert_eq!(expected, got);

                let mut builder = ReaderBuilder::new();
                builder.nfa(true);
                $config(&mut builder);
                let mut rdr = builder.build();
                let got = parse(&mut rdr, $data);
                let expected = $expected;
                assert_eq!(expected, got);
            }
        };
    }

    fn parse(rdr: &mut Reader, data: &str) -> Csv {
        let mut data = data.as_bytes();
        let mut field = [0u8; 10];
        let mut csv = Csv::new();
        let mut row = Row::new();
        let mut fieldpos = 0;
        loop {
            let (res, nin, nout) = rdr.read(data, &mut field[fieldpos..]);
            data = &data[nin..];

            match res {
                ReadResult::InputEmpty => {
                    if !data.is_empty() {
                        panic!("missing input data")
                    }
                    fieldpos += nout;
                }
                ReadResult::OutputFull => panic!("field too large"),
                ReadResult::Field { record_end } => {
                    let s = str::from_utf8(&field[..fieldpos + nout]).unwrap();
                    row.push(Field::from(s).unwrap());
                    fieldpos = 0;
                    if record_end {
                        csv.push(row);
                        row = Row::new();
                    }
                }
                ReadResult::End => {
                    return csv;
                }
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
            let (res, nin, nout) = $rdr.read($input, $output);
            assert_eq!($expect_in, nin);
            assert_eq!($expect_out, nout);
            assert_eq!($expect_res, res);
        }};
    }

    // This tests that feeding a new reader with an empty buffer sends us
    // straight to End.
    #[test]
    fn stream_empty() {
        use ReadResult::*;

        let mut rdr = Reader::new();
        assert_read!(rdr, &[], &mut [], 0, 0, End);
    }

    // Test that a single space is treated as a single field.
    #[test]
    fn stream_space() {
        use ReadResult::*;

        let mut rdr = Reader::new();
        assert_read!(rdr, b(" "), &mut [0], 1, 1, InputEmpty);
        assert_read!(rdr, &[], &mut [0], 0, 0, Field { record_end: true });
        assert_read!(rdr, &[], &mut [0], 0, 0, End);
    }

    // Test that a single comma ...
    #[test]
    fn stream_comma() {
        use ReadResult::*;

        let mut rdr = Reader::new();
        assert_read!(rdr, b(","), &mut [0], 1, 0, Field { record_end: false });
        assert_read!(rdr, &[], &mut [0], 0, 0, Field { record_end: true });
        assert_read!(rdr, &[], &mut [0], 0, 0, End);
    }

    // Test that we can read a single large field in multiple output
    // buffers.
    #[test]
    fn stream_output_chunks() {
        use ReadResult::*;

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
        use ReadResult::*;

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
        use ReadResult::*;

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
        use ReadResult::*;

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

    // Test that we can reset the parser mid-stream and count on it to do
    // the right thing.
    #[test]
    fn reset_works() {
        use ReadResult::*;

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
}
