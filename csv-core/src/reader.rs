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
    #[inline]
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

#[derive(Clone, Debug)]
pub struct Reader {
    dfa: Dfa,
    state: State,
    copy: bool,
    delimiter: u8,
    term: Terminator,
    quote: u8,
    escape: Option<u8>,
    double_quote: bool,
}

impl Default for Reader {
    fn default() -> Reader {
        Reader {
            dfa: Dfa::new(),
            state: State::StartRecord,
            copy: true,
            delimiter: b',',
            term: Terminator::default(),
            quote: b'"',
            escape: None,
            double_quote: true,
        }
    }
}

#[derive(Debug, Default)]
pub struct ReaderBuilder {
    rdr: Reader,
}

impl ReaderBuilder {
    pub fn new() -> ReaderBuilder {
        ReaderBuilder::default()
    }

    pub fn build(&self) -> Reader {
        let mut rdr = self.rdr.clone();
        rdr.build_dfa();
        rdr
    }

    pub fn delimiter(&mut self, delimiter: u8) -> &mut ReaderBuilder {
        self.rdr.delimiter = delimiter;
        self
    }

    pub fn terminator(
        &mut self,
        term: Terminator,
    ) -> &mut ReaderBuilder {
        self.rdr.term = term;
        self
    }

    pub fn quote(&mut self, quote: u8) -> &mut ReaderBuilder {
        self.rdr.quote = quote;
        self
    }

    pub fn escape(&mut self, escape: Option<u8>) -> &mut ReaderBuilder {
        self.rdr.escape = escape;
        self
    }

    pub fn double_quote(&mut self, yes: bool) -> &mut ReaderBuilder {
        self.rdr.double_quote = yes;
        self
    }

    pub fn copy(&mut self, yes: bool) -> &mut ReaderBuilder {
        self.rdr.copy = yes;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadResult {
    InputEmpty,
    OutputFull,
    Field,
    Record,
    End,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum State {
    End = 0,
    StartRecord = 1,
    EndRecord = 2,
    StartField = 3,
    EndFieldDelim = 4,
    EndFieldTerm = 5,
    InField = 6,
    InQuotedField = 7,
    InEscapedQuote = 8,
    InDoubleEscapedQuote = 9,
    InRecordTerm = 10,
    CRLF = 11,
}

const STATES: &'static [State] = &[
    State::End,
    State::StartRecord,
    State::EndRecord,
    State::StartField,
    State::EndFieldDelim,
    State::EndFieldTerm,
    State::InField,
    State::InQuotedField,
    State::InEscapedQuote,
    State::InDoubleEscapedQuote,
    State::InRecordTerm,
    State::CRLF,
];

impl State {
    fn is_final(&self) -> bool {
        match *self {
            State::End
            | State::EndRecord
            | State::EndFieldDelim
            | State::EndFieldTerm => true,
            _ => false,
        }
    }
}

impl Reader {
    pub fn new() -> Reader {
        ReaderBuilder::new().build()
    }

    pub fn reset(&mut self) {
        self.state = State::StartRecord;
    }

    pub fn read(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> (ReadResult, usize, usize) {
        let (mut nin, mut nout) = (0, 0);
        if input.is_empty() {
            self.state = self.nfa_final_transition(self.state);
        } else {
            let (state, copy) = (self.state, self.copy);
            let state = self.state;
            if self.copy {
                let (state, i, o) = self.consume_and_copy(
                    state, input, output);
                self.state = state;
                nin = i;
                nout = o;
            } else {
                let (state, i) = self.consume(state, input);
                self.state = state;
                nin = i;
            }
        }
        let res = match self.state {
            State::End => ReadResult::End,
            State::EndRecord => ReadResult::Record,
            State::EndFieldDelim | State::EndFieldTerm => ReadResult::Field,
            _ => {
                assert!(!self.state.is_final());
                if nin < input.len() && nout >= output.len() {
                    ReadResult::OutputFull
                } else {
                    ReadResult::InputEmpty
                }
            }
        };
        (res, nin, nout)
    }

    #[inline(always)]
    fn consume_and_copy(
        &mut self,
        mut state: State,
        input: &[u8],
        output: &mut [u8],
    ) -> (State, usize, usize) {
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
        (state, nin, nout)
    }

    #[inline(always)]
    fn consume_and_copy_dfa(
        &mut self,
        mut state: State,
        input: &[u8],
        output: &mut [u8],
    ) -> (State, usize, usize) {
        let (mut state, mut nin, mut nout) = (state as u8, 0, 0);
        while nin < input.len() && nout < output.len() {
            let (s, o, fin) = self.dfa.get(state, input[nin]);
            println!("({:?}, {:?}) |--> {:?} (final? {:?})",
                     STATES[state as usize],
                     input[nin] as char,
                     STATES[s as usize],
                     fin);
            if o {
                output[nout] = input[nin];
                nout += 1;
            }
            nin += 1;
            state = s;
            if fin {
                break;
            }
        }
        (STATES[state as usize], nin, nout)
    }

    #[inline(always)]
    fn consume(
        &mut self,
        mut state: State,
        input: &[u8],
    ) -> (State, usize) {
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
        (state, nin)
    }

    #[inline(always)]
    fn nfa_final_transition(&self, state: State) -> State {
        use self::State::*;
        match state {
            End
            | StartRecord
            | EndRecord => End,
            EndFieldTerm
            | InRecordTerm
            | CRLF => EndRecord,
            StartField
            | EndFieldDelim
            | InField
            | InQuotedField
            | InEscapedQuote
            | InDoubleEscapedQuote => EndFieldTerm,
        }
    }

    #[inline(always)]
    fn nfa_transition(&self, state: State, c: u8) -> (State, bool, bool) {
        use self::State::*;
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
                    (EndRecord, true, false)
                } else {
                    (EndRecord, false, false)
                }
            }
        }
    }

    fn build_dfa(&mut self) {
        for &state in STATES {
            for c in (0..256).map(|c| c as u8) {
                let (mut nextstate, mut inp, mut out, mut fin) =
                    (state, false, false, false);
                while !inp && nextstate != State::End {
                    let (s, i, o) = self.nfa_transition(nextstate, c);
                    nextstate = s;
                    inp = i;
                    out = out || o;
                    fin = fin || nextstate.is_final();
                }
                self.dfa.set(state as u8, c, nextstate as u8, out, fin);
            }
        }
    }
}

struct Dfa([u8; 3072]);

impl fmt::Debug for Dfa {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Dfa(N/A)")
    }
}

impl Clone for Dfa {
    fn clone(&self) -> Dfa {
        let mut dfa = Dfa::new();
        dfa.0.copy_from_slice(&self.0);
        dfa
    }
}

impl Dfa {
    fn new() -> Dfa {
        Dfa([0; 3072])
    }

    fn get(&self, stateidx: u8, c: u8) -> (u8, bool, bool) {
        let mut idx = self.0[stateidx as usize * 256 + c as usize];
        (idx & 0b0011_1111, idx & 0b1000_0000 > 0, idx & 0b0100_0000 > 0)
    }

    fn set(
        &mut self,
        fromidx: u8,
        c: u8,
        mut toidx: u8,
        output: bool,
        fin: bool,
    ) {
        if output {
            toidx |= 0b1000_0000;
        }
        if fin {
            toidx |= 0b0100_0000;
        }
        self.0[fromidx as usize * 256 + c as usize] = toidx;
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
            let mut csv = Csv::new();
            $(
                let mut row = Row::new();
                $(
                    row.push(Field::from($field).unwrap());
                )*
                csv.push(row);
            )*
            csv
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
                    // println!("InputEmpty");
                    if !data.is_empty() {
                        panic!("missing input data")
                    }
                    fieldpos += nout;
                }
                ReadResult::OutputFull => panic!("field too large"),
                ReadResult::Field => {
                    // println!("Field");
                    let s = str::from_utf8(&field[..fieldpos + nout]).unwrap();
                    row.push(Field::from(s).unwrap());
                    fieldpos = 0;
                }
                ReadResult::Record => {
                    // println!("Record");
                    // let s = str::from_utf8(&field[..fieldpos + nout]).unwrap();
                    // row.push(Field::from(s).unwrap());
                    // fieldpos = 0;
                    csv.push(row);
                    row = Row::new();
                }
                ReadResult::End => {
                    // println!("End");
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
        |b: &mut ReaderBuilder| {
            b.delimiter(b'\x1f').terminator(Terminator::Any(b'\x1e'));
        });

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
        assert_read!(rdr, &[], &mut [0], 0, 0, Field);
        assert_read!(rdr, &[], &mut [0], 0, 0, Record);
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

        assert_read!(rdr, inp, out, 0, 0, Field);
        assert_read!(rdr, inp, out, 0, 0, Record);
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

        assert_read!(rdr, &[], out, 0, 0, Field);
        assert_read!(rdr, &[], out, 0, 0, Record);
        assert_read!(rdr, &[], out, 0, 0, End);
    }
}
