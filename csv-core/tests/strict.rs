use csv_core::{
    ReadFieldResult, ReadRecordResult, Reader, ReaderBuilder, Terminator,
};

/// One parsed field: its unescaped bytes, whether it ended its record and
/// whether it was reported as `Invalid`.
#[derive(Clone, Debug, PartialEq)]
struct Field {
    data: Vec<u8>,
    record_end: bool,
    invalid: bool,
}

/// One parsed record: its fields and whether it was reported as `Invalid`.
#[derive(Clone, Debug, PartialEq)]
struct Record {
    fields: Vec<Vec<u8>>,
    invalid: bool,
}

/// A parser configuration applied to a `ReaderBuilder`.
#[derive(Clone, Copy, Debug)]
struct Config {
    delimiter: u8,
    quote: u8,
    escape: Option<u8>,
    double_quote: bool,
    quoting: bool,
    comment: Option<u8>,
    term: Option<u8>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            delimiter: b',',
            quote: b'"',
            escape: None,
            double_quote: true,
            quoting: true,
            comment: None,
            term: None,
        }
    }
}

fn build(cfg: &Config, strict: bool, nfa: bool) -> Reader {
    let mut b = ReaderBuilder::new();
    b.delimiter(cfg.delimiter)
        .quote(cfg.quote)
        .escape(cfg.escape)
        .double_quote(cfg.double_quote)
        .quoting(cfg.quoting)
        .comment(cfg.comment)
        .nfa(nfa)
        .strict(strict);
    if let Some(t) = cfg.term {
        b.terminator(Terminator::Any(t));
    }
    b.build()
}

/// Reads all fields, feeding `chunk` input bytes and offering `outcap`
/// output bytes per call. Also returns the `(nin, nout)` of every call with
/// the result kind, `Invalid` folded into `Field`.
fn fields_with(
    rdr: &mut Reader,
    data: &[u8],
    chunk: usize,
    outcap: usize,
) -> (Vec<Field>, Vec<(String, usize, usize)>) {
    let mut out = vec![0u8; data.len() + 16];
    let mut outlen = 0;
    let mut pos = 0;
    let mut limit = chunk.min(data.len());
    let mut fields = vec![];
    let mut trace = vec![];
    for _ in 0..100_000 {
        if pos == limit && limit < data.len() {
            limit = (limit + chunk).min(data.len());
        }
        let end = (outlen + outcap).min(out.len());
        let (res, nin, nout) =
            rdr.read_field(&data[pos..limit], &mut out[outlen..end]);
        pos += nin;
        outlen += nout;
        let kind = match res {
            ReadFieldResult::Invalid { record_end } => {
                format!("Field {{ record_end: {} }}", record_end)
            }
            ref other => format!("{:?}", other),
        };
        trace.push((kind, nin, nout));
        match res {
            ReadFieldResult::InputEmpty | ReadFieldResult::OutputFull => {}
            ReadFieldResult::Field { record_end } => {
                fields.push(Field {
                    data: out[..outlen].to_vec(),
                    record_end,
                    invalid: false,
                });
                outlen = 0;
            }
            ReadFieldResult::Invalid { record_end } => {
                fields.push(Field {
                    data: out[..outlen].to_vec(),
                    record_end,
                    invalid: true,
                });
                outlen = 0;
            }
            ReadFieldResult::End => return (fields, trace),
        }
    }
    panic!(
        "the reader never returned End for {:?}",
        String::from_utf8_lossy(data)
    );
}

/// Reads all records, feeding `chunk` input bytes, offering `outcap` output
/// bytes and `endscap` field ends per call. Also returns the
/// `(nin, nout, nend)` of every call with the result kind, `Invalid` folded
/// into `Record`.
fn records_with(
    rdr: &mut Reader,
    data: &[u8],
    chunk: usize,
    outcap: usize,
    endscap: usize,
) -> (Vec<Record>, Vec<(String, usize, usize, usize)>) {
    let mut out = vec![0u8; data.len() + 16];
    let mut ends = vec![0usize; data.len() + 16];
    let (mut outlen, mut endlen) = (0, 0);
    let mut pos = 0;
    let mut limit = chunk.min(data.len());
    let mut records = vec![];
    let mut trace = vec![];
    for _ in 0..100_000 {
        if pos == limit && limit < data.len() {
            limit = (limit + chunk).min(data.len());
        }
        let oend = (outlen + outcap).min(out.len());
        let eend = (endlen + endscap).min(ends.len());
        let (res, nin, nout, nend) = rdr.read_record(
            &data[pos..limit],
            &mut out[outlen..oend],
            &mut ends[endlen..eend],
        );
        pos += nin;
        outlen += nout;
        endlen += nend;
        let kind = match res {
            ReadRecordResult::Invalid => "Record".to_string(),
            ref other => format!("{:?}", other),
        };
        trace.push((kind, nin, nout, nend));
        let invalid = match res {
            ReadRecordResult::InputEmpty
            | ReadRecordResult::OutputFull
            | ReadRecordResult::OutputEndsFull => continue,
            ReadRecordResult::End => return (records, trace),
            ReadRecordResult::Record => false,
            ReadRecordResult::Invalid => true,
        };
        let mut fields = vec![];
        let mut start = 0;
        for &e in &ends[..endlen] {
            fields.push(out[start..e].to_vec());
            start = e;
        }
        records.push(Record { fields, invalid });
        outlen = 0;
        endlen = 0;
    }
    panic!(
        "the reader never returned End for {:?}",
        String::from_utf8_lossy(data)
    );
}

fn fields(cfg: &Config, strict: bool, nfa: bool, data: &[u8]) -> Vec<Field> {
    fields_with(&mut build(cfg, strict, nfa), data, data.len().max(1), 1 << 20)
        .0
}

fn records(cfg: &Config, strict: bool, nfa: bool, data: &[u8]) -> Vec<Record> {
    records_with(
        &mut build(cfg, strict, nfa),
        data,
        data.len().max(1),
        1 << 20,
        1 << 20,
    )
    .0
}

fn f(data: &str, record_end: bool, invalid: bool) -> Field {
    Field { data: data.as_bytes().to_vec(), record_end, invalid }
}

fn r(fields: &[&str], invalid: bool) -> Record {
    Record {
        fields: fields.iter().map(|s| s.as_bytes().to_vec()).collect(),
        invalid,
    }
}

/// Checks that the strict reader reports exactly the expected fields and
/// records, with both parsers and with every field ending up as the
/// non-strict reader parses it.
fn check(
    cfg: &Config,
    data: &str,
    want_fields: &[Field],
    want_records: &[Record],
) {
    let data = data.as_bytes();
    for &nfa in &[false, true] {
        let engine = if nfa { "nfa" } else { "dfa" };
        assert_eq!(
            fields(cfg, true, nfa, data),
            want_fields,
            "{} read_field of {:?}",
            engine,
            String::from_utf8_lossy(data)
        );
        assert_eq!(
            records(cfg, true, nfa, data),
            want_records,
            "{} read_record of {:?}",
            engine,
            String::from_utf8_lossy(data)
        );
        let lenient: Vec<Field> = want_fields
            .iter()
            .map(|x| Field { invalid: false, ..x.clone() })
            .collect();
        assert_eq!(
            fields(cfg, false, nfa, data),
            lenient,
            "{} non-strict read_field",
            engine
        );
    }
}

#[test]
fn quote_inside_unquoted_field_is_invalid() {
    check(
        &Config::default(),
        "a\"b,c\n",
        &[f("a\"b", false, true), f("c", true, false)],
        &[r(&["a\"b", "c"], true)],
    );
}

#[test]
fn quote_after_leading_space_is_invalid() {
    check(
        &Config::default(),
        "x, \"y\"\n",
        &[f("x", false, false), f(" \"y\"", true, true)],
        &[r(&["x", " \"y\""], true)],
    );
}

#[test]
fn byte_after_closing_quote_is_invalid() {
    check(
        &Config::default(),
        "\"a\"b,c\n\"d\" ,e\n",
        &[
            f("ab", false, true),
            f("c", true, false),
            f("d ", false, true),
            f("e", true, false),
        ],
        &[r(&["ab", "c"], true), r(&["d ", "e"], true)],
    );
}

#[test]
fn closing_quote_before_delimiter_terminator_or_end_is_valid() {
    check(
        &Config::default(),
        "\"a\",\"b\"\n\"c\",\"d\"\r\n\"e\",\"f\"\r\"g\",\"h\"",
        &[
            f("a", false, false),
            f("b", true, false),
            f("c", false, false),
            f("d", true, false),
            f("e", false, false),
            f("f", true, false),
            f("g", false, false),
            f("h", true, false),
        ],
        &[
            r(&["a", "b"], false),
            r(&["c", "d"], false),
            r(&["e", "f"], false),
            r(&["g", "h"], false),
        ],
    );
}

#[test]
fn doubled_quotes_are_valid() {
    check(
        &Config::default(),
        "\"a\"\"b\",\"\"\"\"\n\"\",x\n",
        &[
            f("a\"b", false, false),
            f("\"", true, false),
            f("", false, false),
            f("x", true, false),
        ],
        &[r(&["a\"b", "\""], false), r(&["", "x"], false)],
    );
}

#[test]
fn doubled_quote_is_invalid_without_double_quote() {
    let cfg = Config { double_quote: false, ..Config::default() };
    check(
        &cfg,
        "\"a\"\"b\",c\n",
        &[f("a\"b\"", false, true), f("c", true, false)],
        &[r(&["a\"b\"", "c"], true)],
    );
}

#[test]
fn escaped_quote_is_valid() {
    let cfg = Config {
        double_quote: false,
        escape: Some(b'\\'),
        ..Config::default()
    };
    check(
        &cfg,
        "\"a\\\"b\",c\n\"x\"\\y,z\n",
        &[
            f("a\"b", false, false),
            f("c", true, false),
            f("x\\y", false, true),
            f("z", true, false),
        ],
        &[r(&["a\"b", "c"], false), r(&["x\\y", "z"], true)],
    );
}

#[test]
fn delimiter_and_newline_inside_quotes_are_valid() {
    check(
        &Config::default(),
        "\"a,b\nc\",d\n",
        &[f("a,b\nc", false, false), f("d", true, false)],
        &[r(&["a,b\nc", "d"], false)],
    );
}

#[test]
fn end_of_data_inside_quotes_is_invalid() {
    check(
        &Config::default(),
        "a,b\nc,\"d,e\nf",
        &[
            f("a", false, false),
            f("b", true, false),
            f("c", false, false),
            f("d,e\nf", true, true),
        ],
        &[r(&["a", "b"], false), r(&["c", "d,e\nf"], true)],
    );
}

#[test]
fn end_of_data_after_escape_is_invalid() {
    let cfg = Config {
        double_quote: false,
        escape: Some(b'\\'),
        ..Config::default()
    };
    check(&cfg, "\"ab\\", &[f("ab", true, true)], &[r(&["ab"], true)]);
}

#[test]
fn lone_quote_field_is_invalid() {
    check(
        &Config::default(),
        "a,\"",
        &[f("a", false, false), f("", true, true)],
        &[r(&["a", ""], true)],
    );
    check(
        &Config::default(),
        "\"\"\"\n",
        &[f("\"\n", true, true)],
        &[r(&["\"\n"], true)],
    );
}

#[test]
fn several_errors_in_one_field_report_it_once() {
    check(
        &Config::default(),
        "\"a\"b\"c\"d,e\n",
        &[f("ab\"c\"d", false, true), f("e", true, false)],
        &[r(&["ab\"c\"d", "e"], true)],
    );
}

#[test]
fn only_the_bad_record_is_invalid() {
    check(
        &Config::default(),
        "a,b\nc\"d,e\nf,g\n",
        &[
            f("a", false, false),
            f("b", true, false),
            f("c\"d", false, true),
            f("e", true, false),
            f("f", false, false),
            f("g", true, false),
        ],
        &[
            r(&["a", "b"], false),
            r(&["c\"d", "e"], true),
            r(&["f", "g"], false),
        ],
    );
}

#[test]
fn comment_lines_are_not_checked() {
    let cfg = Config { comment: Some(b'#'), ..Config::default() };
    check(
        &cfg,
        "#\"x\"y a\"b\na,b\n#\"unterminated\nc,d",
        &[
            f("a", false, false),
            f("b", true, false),
            f("c", false, false),
            f("d", true, false),
        ],
        &[r(&["a", "b"], false), r(&["c", "d"], false)],
    );
}

#[test]
fn custom_quote_delimiter_and_terminator() {
    let cfg = Config {
        delimiter: b';',
        quote: b'\'',
        term: Some(b'|'),
        ..Config::default()
    };
    check(
        &cfg,
        "'a';b\"c|'d'e;f|x';y",
        &[
            f("a", false, false),
            f("b\"c", true, false),
            f("de", false, true),
            f("f", true, false),
            f("x'", false, true),
            f("y", true, false),
        ],
        &[
            r(&["a", "b\"c"], false),
            r(&["de", "f"], true),
            r(&["x'", "y"], true),
        ],
    );
}

#[test]
fn newline_after_closing_quote_is_invalid_with_custom_terminator() {
    let cfg = Config { term: Some(b'|'), ..Config::default() };
    check(
        &cfg,
        "\"a\"\n|\"b\"|",
        &[f("a\n", true, true), f("b", true, false)],
        &[r(&["a\n"], true), r(&["b"], false)],
    );
}

#[test]
fn quoting_disabled_never_reports_invalid() {
    let cfg = Config { quoting: false, ..Config::default() };
    check(
        &cfg,
        "a\"b,\"c\"d\n\"e",
        &[
            f("a\"b", false, false),
            f("\"c\"d", true, false),
            f("\"e", true, false),
        ],
        &[r(&["a\"b", "\"c\"d"], false), r(&["\"e"], false)],
    );
}

#[test]
fn strict_is_off_by_default() {
    let data = b"a\"b,\"c\"d\n\"e";
    for &nfa in &[false, true] {
        let mut b = ReaderBuilder::new();
        b.nfa(nfa);
        let got = fields_with(&mut b.build(), data, data.len(), 1 << 20).0;
        assert!(got.iter().all(|x| !x.invalid), "{:?}", got);
        let got =
            records_with(&mut b.build(), data, data.len(), 1 << 20, 1 << 20).0;
        assert!(got.iter().all(|x| !x.invalid), "{:?}", got);
        let got = records_with(
            &mut Reader::new(),
            data,
            data.len(),
            1 << 20,
            1 << 20,
        )
        .0;
        assert!(got.iter().all(|x| !x.invalid), "{:?}", got);
    }
}

#[test]
fn strict_can_be_switched_off_again() {
    let mut b = ReaderBuilder::new();
    b.strict(true).strict(false);
    let data = b"a\"b,c\n";
    let got =
        records_with(&mut b.build(), data, data.len(), 1 << 20, 1 << 20).0;
    assert_eq!(got, vec![r(&["a\"b", "c"], false)]);
}

#[test]
fn invalid_byte_in_a_later_call() {
    for &nfa in &[false, true] {
        let mut rdr = build(&Config::default(), true, nfa);
        let mut out = [0u8; 64];
        let (res, nin, nout) = rdr.read_field(b"\"ab\"", &mut out);
        assert_eq!((res, nin, nout), (ReadFieldResult::InputEmpty, 4, 2));
        let (res, nin, nout) = rdr.read_field(b"c,d\n", &mut out[2..]);
        assert_eq!(
            (res, nin, nout),
            (ReadFieldResult::Invalid { record_end: false }, 2, 1)
        );
        assert_eq!(&out[..3], b"abc");
        let (res, _, _) = rdr.read_field(b"d\n", &mut out);
        assert_eq!(res, ReadFieldResult::Field { record_end: true });
        let (res, _, _) = rdr.read_field(b"", &mut out);
        assert_eq!(res, ReadFieldResult::End);
    }
    // A record split over two calls.
    for &nfa in &[false, true] {
        let mut out = [0u8; 64];
        let mut rdr = build(&Config::default(), true, nfa);
        let mut ends = [0usize; 8];
        let (res, nin, nout, nend) =
            rdr.read_record(b"x,\"ab\"", &mut out, &mut ends);
        assert_eq!(
            (res, nin, nout, nend),
            (ReadRecordResult::InputEmpty, 6, 3, 1)
        );
        let (res, nin, nout, nend) =
            rdr.read_record(b"!\ny\n", &mut out[3..], &mut ends[1..]);
        assert_eq!(
            (res, nin, nout, nend),
            (ReadRecordResult::Invalid, 2, 1, 1)
        );
        assert_eq!((&out[..4], &ends[..2]), (&b"xab!"[..], &[1usize, 4][..]));
        let (res, _, _, _) = rdr.read_record(b"y\n", &mut out, &mut ends);
        assert_eq!(res, ReadRecordResult::Record);

        // Split right before the quote of an unquoted field.
        let mut rdr = build(&Config::default(), true, nfa);
        let (res, nin, nout, nend) =
            rdr.read_record(b"a", &mut out, &mut ends);
        assert_eq!(
            (res, nin, nout, nend),
            (ReadRecordResult::InputEmpty, 1, 1, 0)
        );
        let (res, nin, nout, nend) =
            rdr.read_record(b"\"b,c\n", &mut out[1..], &mut ends);
        assert_eq!(
            (res, nin, nout, nend),
            (ReadRecordResult::Invalid, 5, 3, 2)
        );
        assert_eq!((&out[..4], &ends[..2]), (&b"a\"bc"[..], &[3usize, 4][..]));
    }
}

#[test]
fn unterminated_quote_reported_at_end_of_input() {
    for &nfa in &[false, true] {
        let mut rdr = build(&Config::default(), true, nfa);
        let mut out = [0u8; 64];
        let mut ends = [0usize; 8];
        let (res, _, _, _) = rdr.read_record(b"a,\"bc", &mut out, &mut ends);
        assert_eq!(res, ReadRecordResult::InputEmpty);
        // No room for the final field end: nothing is reported yet.
        let (res, nin, nout, nend) =
            rdr.read_record(b"", &mut out[3..], &mut []);
        assert_eq!(
            (res, nin, nout, nend),
            (ReadRecordResult::OutputEndsFull, 0, 0, 0)
        );
        let (res, nin, nout, nend) =
            rdr.read_record(b"", &mut out[3..], &mut ends[1..]);
        assert_eq!(
            (res, nin, nout, nend),
            (ReadRecordResult::Invalid, 0, 0, 1)
        );
        assert_eq!(&ends[..2], &[1usize, 3][..]);
        let (res, _, _, _) = rdr.read_record(b"", &mut out, &mut ends);
        assert_eq!(res, ReadRecordResult::End);
    }
}

#[test]
fn reset_forgets_a_partly_read_invalid_field() {
    for &nfa in &[false, true] {
        let mut rdr = build(&Config::default(), true, nfa);
        let mut out = [0u8; 64];
        let mut ends = [0usize; 8];
        let (res, _, _) = rdr.read_field(b"\"a\"b", &mut out);
        assert_eq!(res, ReadFieldResult::InputEmpty);
        rdr.reset();
        let (res, _, _) = rdr.read_field(b"c,", &mut out);
        assert_eq!(res, ReadFieldResult::Field { record_end: false });

        let mut rdr = build(&Config::default(), true, nfa);
        let (res, _, _, _) = rdr.read_record(b"a\"b,c", &mut out, &mut ends);
        assert_eq!(res, ReadRecordResult::InputEmpty);
        rdr.reset();
        let (res, _, _, _) = rdr.read_record(b"d,e\n", &mut out, &mut ends);
        assert_eq!(res, ReadRecordResult::Record);
    }
}

#[test]
fn field_reads_then_record_reads_keep_their_own_verdicts() {
    for &nfa in &[false, true] {
        let mut rdr = build(&Config::default(), true, nfa);
        let mut out = [0u8; 64];
        let mut ends = [0usize; 8];
        let data = b"a\"b,c\nd,e\n";
        let (res, nin, _) = rdr.read_field(data, &mut out);
        assert_eq!(res, ReadFieldResult::Invalid { record_end: false });
        let (res, nin2, _) = rdr.read_field(&data[nin..], &mut out);
        assert_eq!(res, ReadFieldResult::Field { record_end: true });
        let (res, _, _, _) =
            rdr.read_record(&data[nin + nin2..], &mut out, &mut ends);
        assert_eq!(res, ReadRecordResult::Record);
    }
}

// Deterministic generator of CSV documents whose invalid fields are known by
// construction.

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum LineEnd {
    Lf,
    CrLf,
    Cr,
    Any(u8),
}

fn plain(rng: &mut Rng, cfg: &Config, min: u64) -> Vec<u8> {
    let mut alphabet = b"abcxyz019 ".to_vec();
    if let Some(e) = cfg.escape {
        alphabet.push(e);
    }
    let n = min + rng.below(4);
    (0..n).map(|_| rng.pick(&alphabet)).collect()
}

fn quoted_body(rng: &mut Rng, cfg: &Config, eol: LineEnd) -> Vec<u8> {
    let mut v = vec![];
    for _ in 0..rng.below(6) {
        match rng.below(7) {
            0 => v.push(cfg.delimiter),
            1 => v.push(b'\n'),
            2 => v.push(b'\r'),
            3 => {
                if let LineEnd::Any(t) = eol {
                    v.push(t);
                }
            }
            4 => {
                if cfg.double_quote {
                    v.push(cfg.quote);
                    v.push(cfg.quote);
                } else if let Some(e) = cfg.escape {
                    v.push(e);
                    v.push(cfg.quote);
                }
            }
            _ => v.push(rng.pick(b"abcxyz ")),
        }
    }
    v
}

fn quoted(rng: &mut Rng, cfg: &Config, eol: LineEnd) -> Vec<u8> {
    let mut v = vec![cfg.quote];
    v.extend(quoted_body(rng, cfg, eol));
    v.push(cfg.quote);
    v
}

/// Returns a field and whether it has invalid quoting.
fn gen_field(
    rng: &mut Rng,
    cfg: &Config,
    eol: LineEnd,
    first: bool,
) -> (Vec<u8>, bool) {
    // A record's first field is never empty, so no record is a blank line,
    // and never starts with the comment byte.
    let min = if first { 1 } else { 0 };
    match rng.below(6) {
        0 | 1 => (plain(rng, cfg, min), false),
        2 | 3 => (quoted(rng, cfg, eol), false),
        4 => {
            // A quote inside an unquoted field.
            let mut v = plain(rng, cfg, 1);
            v.push(cfg.quote);
            v.extend(plain(rng, cfg, 0));
            if rng.below(2) == 0 {
                v.push(cfg.quote);
            }
            (v, true)
        }
        _ => {
            // Something other than a delimiter or terminator after the
            // closing quote.
            let mut v = quoted(rng, cfg, eol);
            if !cfg.double_quote && rng.below(3) == 0 {
                v.push(cfg.quote);
            } else {
                v.push(rng.pick(b"abz "));
            }
            v.extend(plain(rng, cfg, 0));
            if rng.below(3) == 0 {
                v.push(cfg.quote);
            }
            (v, true)
        }
    }
}

struct Doc {
    data: Vec<u8>,
    // Validity of every field, per record.
    records: Vec<Vec<bool>>,
}

fn gen_doc(rng: &mut Rng, cfg: &Config, eol: LineEnd) -> Doc {
    let term: &[u8] = match eol {
        LineEnd::Lf => b"\n",
        LineEnd::CrLf => b"\r\n",
        LineEnd::Cr => b"\r",
        LineEnd::Any(_) => b"",
    };
    let term: Vec<u8> = match eol {
        LineEnd::Any(t) => vec![t],
        _ => term.to_vec(),
    };
    let mut data = vec![];
    let mut records = vec![];
    let nrec = 1 + rng.below(5);
    for i in 0..nrec {
        let newline_ends = eol == LineEnd::Lf || eol == LineEnd::CrLf;
        if cfg.comment.is_some() && newline_ends && rng.below(4) == 0 {
            data.push(cfg.comment.unwrap());
            data.extend(plain(rng, cfg, 0));
            data.push(cfg.quote);
            data.extend(plain(rng, cfg, 0));
            data.push(b'\n');
        }
        if rng.below(5) == 0 {
            data.extend(&term);
        }
        let nfields = 1 + rng.below(4);
        let mut valid = vec![];
        for j in 0..nfields {
            if j > 0 {
                data.push(cfg.delimiter);
            }
            let (field, invalid) = gen_field(rng, cfg, eol, j == 0);
            data.extend(field);
            valid.push(invalid);
        }
        let last = i + 1 == nrec;
        if last && rng.below(3) == 0 {
            // The data ends inside a quoted field of the last record.
            data.push(cfg.delimiter);
            data.push(cfg.quote);
            data.extend(quoted_body(rng, cfg, eol));
            if cfg.escape.is_some() && rng.below(2) == 0 {
                data.push(cfg.escape.unwrap());
            }
            valid.push(true);
        } else if !last || rng.below(2) == 0 {
            data.extend(&term);
        }
        records.push(valid);
    }
    Doc { data, records }
}

fn configs() -> Vec<(Config, LineEnd)> {
    let mut v = vec![];
    for &(delimiter, quote) in
        &[(b',', b'"'), (b';', b'\''), (b'\t', b'"'), (b'|', b'\'')]
    {
        for &(double_quote, escape) in &[
            (true, None),
            (false, Some(b'\\')),
            (false, None),
            (true, Some(b'\\')),
        ] {
            for &eol in
                &[LineEnd::Lf, LineEnd::CrLf, LineEnd::Cr, LineEnd::Any(b'~')]
            {
                let comments: &[Option<u8>] = match eol {
                    LineEnd::Lf | LineEnd::CrLf => &[None, Some(b'#')],
                    _ => &[None],
                };
                for &comment in comments {
                    let cfg = Config {
                        delimiter,
                        quote,
                        escape,
                        double_quote,
                        quoting: true,
                        comment,
                        term: match eol {
                            LineEnd::Any(t) => Some(t),
                            _ => None,
                        },
                    };
                    v.push((cfg, eol));
                }
            }
        }
    }
    v
}

#[test]
fn generated_documents_by_record() {
    let mut rng = Rng(0x5eed_a781);
    for (cfg, eol) in configs() {
        for _ in 0..12 {
            let doc = gen_doc(&mut rng, &cfg, eol);
            let shown = String::from_utf8_lossy(&doc.data).into_owned();
            let want: Vec<bool> =
                doc.records.iter().map(|fs| fs.iter().any(|&x| x)).collect();
            for &nfa in &[false, true] {
                for &(chunk, outcap, endscap) in &[
                    (usize::MAX, 1 << 20, 1 << 20),
                    (1, 1 << 20, 1 << 20),
                    (3, 2, 1),
                    (7, 5, 2),
                ] {
                    let chunk = chunk.min(doc.data.len().max(1));
                    let (lenient, lenient_trace) = records_with(
                        &mut build(&cfg, false, nfa),
                        &doc.data,
                        chunk,
                        outcap,
                        endscap,
                    );
                    let (strict, strict_trace) = records_with(
                        &mut build(&cfg, true, nfa),
                        &doc.data,
                        chunk,
                        outcap,
                        endscap,
                    );
                    let ctx = format!(
                        "{:?} nfa={} chunk={} outcap={} endscap={} data={:?}",
                        cfg, nfa, chunk, outcap, endscap, shown
                    );
                    assert_eq!(
                        strict_trace, lenient_trace,
                        "call results differ from the non-strict reader: {}",
                        ctx
                    );
                    let data: Vec<_> =
                        strict.iter().map(|x| x.fields.clone()).collect();
                    let lenient_data: Vec<_> =
                        lenient.iter().map(|x| x.fields.clone()).collect();
                    assert_eq!(
                        data, lenient_data,
                        "record data differs from the non-strict reader: {}",
                        ctx
                    );
                    let got: Vec<bool> =
                        strict.iter().map(|x| x.invalid).collect();
                    assert_eq!(got, want, "invalid records: {}", ctx);
                }
            }
        }
    }
}

#[test]
fn generated_documents_by_field() {
    let mut rng = Rng(0xf1e1_d5);
    for (cfg, eol) in configs() {
        for _ in 0..12 {
            let doc = gen_doc(&mut rng, &cfg, eol);
            let shown = String::from_utf8_lossy(&doc.data).into_owned();
            let want: Vec<bool> =
                doc.records.iter().flatten().copied().collect();
            for &nfa in &[false, true] {
                for &(chunk, outcap) in
                    &[(usize::MAX, 1 << 20), (1, 1 << 20), (2, 1), (5, 3)]
                {
                    let chunk = chunk.min(doc.data.len().max(1));
                    let (lenient, lenient_trace) = fields_with(
                        &mut build(&cfg, false, nfa),
                        &doc.data,
                        chunk,
                        outcap,
                    );
                    let (strict, strict_trace) = fields_with(
                        &mut build(&cfg, true, nfa),
                        &doc.data,
                        chunk,
                        outcap,
                    );
                    let ctx = format!(
                        "{:?} nfa={} chunk={} outcap={} data={:?}",
                        cfg, nfa, chunk, outcap, shown
                    );
                    assert_eq!(
                        strict_trace, lenient_trace,
                        "call results differ from the non-strict reader: {}",
                        ctx
                    );
                    let data: Vec<_> = strict
                        .iter()
                        .map(|x| (x.data.clone(), x.record_end))
                        .collect();
                    let lenient_data: Vec<_> = lenient
                        .iter()
                        .map(|x| (x.data.clone(), x.record_end))
                        .collect();
                    assert_eq!(
                        data, lenient_data,
                        "field data differs from the non-strict reader: {}",
                        ctx
                    );
                    let got: Vec<bool> =
                        strict.iter().map(|x| x.invalid).collect();
                    assert_eq!(got, want, "invalid fields: {}", ctx);
                }
            }
        }
    }
}
