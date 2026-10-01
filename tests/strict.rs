use csv::{
    ByteRecord, Error, ErrorKind, ReaderBuilder, StringRecord, Terminator,
};
use serde::Deserialize;

type Pos = (u64, u64, u64);

/// Returns `(byte, line, record)` of an `InvalidQuoting` error, and checks
/// that both position accessors agree with it.
fn invalid_pos(err: &Error) -> Pos {
    let pos = match *err.kind() {
        ErrorKind::InvalidQuoting { ref pos } => {
            pos.clone().expect("InvalidQuoting error without a position")
        }
        ref other => {
            panic!("expected an InvalidQuoting error, got {:?}", other)
        }
    };
    let via_error = err.position().expect("Error::position is None");
    let via_kind = err.kind().position().expect("ErrorKind::position is None");
    assert_eq!(
        (via_error.byte(), via_error.line(), via_error.record()),
        (pos.byte(), pos.line(), pos.record())
    );
    assert_eq!(
        (via_kind.byte(), via_kind.line(), via_kind.record()),
        (pos.byte(), pos.line(), pos.record())
    );
    (pos.byte(), pos.line(), pos.record())
}

#[derive(Debug, PartialEq)]
enum Item {
    Rec(Vec<String>),
    Bad(Pos),
}

fn rec(fields: &[&str]) -> Item {
    Item::Rec(fields.iter().map(|s| s.to_string()).collect())
}

fn item(res: Result<StringRecord, Error>) -> Item {
    match res {
        Ok(r) => Item::Rec(r.iter().map(|s| s.to_string()).collect()),
        Err(err) => Item::Bad(invalid_pos(&err)),
    }
}

fn byte_item(res: Result<ByteRecord, Error>) -> Item {
    match res {
        Ok(r) => Item::Rec(
            r.iter().map(|s| String::from_utf8(s.to_vec()).unwrap()).collect(),
        ),
        Err(err) => Item::Bad(invalid_pos(&err)),
    }
}

fn strict() -> ReaderBuilder {
    let mut b = ReaderBuilder::new();
    b.strict(true);
    b
}

/// Reads `data` through every record reading API of a reader built by
/// `builder` and checks that each gives `want`.
fn check_all_apis(builder: &ReaderBuilder, data: &str, want: &[Item]) {
    let got: Vec<Item> =
        builder.from_reader(data.as_bytes()).records().map(item).collect();
    assert_eq!(got, want, "records() of {:?}", data);
    let got: Vec<Item> = builder
        .from_reader(data.as_bytes())
        .byte_records()
        .map(byte_item)
        .collect();
    assert_eq!(got, want, "byte_records() of {:?}", data);
    let got: Vec<Item> = builder
        .from_reader(data.as_bytes())
        .into_records()
        .map(item)
        .collect();
    assert_eq!(got, want, "into_records() of {:?}", data);
    let got: Vec<Item> = builder
        .from_reader(data.as_bytes())
        .into_byte_records()
        .map(byte_item)
        .collect();
    assert_eq!(got, want, "into_byte_records() of {:?}", data);

    let mut rdr = builder.from_reader(data.as_bytes());
    let mut got = vec![];
    let mut record = StringRecord::new();
    for _ in 0..want.len() + 5 {
        match rdr.read_record(&mut record) {
            Ok(true) => got.push(item(Ok(record.clone()))),
            Ok(false) => break,
            Err(err) => got.push(item(Err(err))),
        }
    }
    assert_eq!(got, want, "read_record of {:?}", data);
    assert!(rdr.is_done());

    let mut rdr = builder.from_reader(data.as_bytes());
    let mut got = vec![];
    let mut record = ByteRecord::new();
    for _ in 0..want.len() + 5 {
        match rdr.read_byte_record(&mut record) {
            Ok(true) => got.push(byte_item(Ok(record.clone()))),
            Ok(false) => break,
            Err(err) => got.push(byte_item(Err(err))),
        }
    }
    assert_eq!(got, want, "read_byte_record of {:?}", data);
}

#[test]
fn invalid_record_is_reported_and_skipped() {
    let data = "h1,h2\na,b\nc\"d,e\nf,g\n";
    check_all_apis(
        &strict(),
        data,
        &[rec(&["a", "b"]), Item::Bad((10, 3, 2)), rec(&["f", "g"])],
    );
}

#[test]
fn junk_after_closing_quote_is_reported() {
    let mut b = strict();
    b.has_headers(false);
    check_all_apis(
        &b,
        "\"a\"b,c\n\"d\" ,e\n\"f\",\"g\"\n",
        &[Item::Bad((0, 1, 0)), Item::Bad((7, 2, 1)), rec(&["f", "g"])],
    );
}

#[test]
fn valid_quoting_is_accepted() {
    let mut b = strict();
    b.has_headers(false).flexible(true);
    check_all_apis(
        &b,
        "\"a\"\"b\",\"c,d\ne\"\r\n\"\",x\r\"y\"",
        &[rec(&["a\"b", "c,d\ne"]), rec(&["", "x"]), rec(&["y"])],
    );
}

#[test]
fn unterminated_quote_is_reported_last() {
    let mut b = strict();
    b.has_headers(false);
    check_all_apis(
        &b,
        "a,b\nc,\"d\ne,f\n",
        &[rec(&["a", "b"]), Item::Bad((4, 2, 1))],
    );
}

#[test]
fn escape_and_double_quote_settings_are_respected() {
    let mut b = strict();
    b.has_headers(false).double_quote(false).escape(Some(b'\\'));
    check_all_apis(
        &b,
        "\"a\\\"b\",c\n\"d\"\"e\",f\n\"g\\",
        &[rec(&["a\"b", "c"]), Item::Bad((9, 2, 1)), Item::Bad((18, 3, 2))],
    );
}

#[test]
fn custom_quote_delimiter_terminator_and_comment() {
    let mut b = strict();
    b.has_headers(false)
        .delimiter(b';')
        .quote(b'\'')
        .terminator(Terminator::Any(b'\n'))
        .comment(Some(b'#'));
    check_all_apis(
        &b,
        "#'x'y\n'a';b\"c\nd'e;f\n'g'h;i\n",
        &[rec(&["a", "b\"c"]), Item::Bad((14, 3, 1)), Item::Bad((20, 4, 2))],
    );
}

#[test]
fn strict_is_off_by_default() {
    let data = "a\"b,c\n\"d\"e,f\n\"g";
    let mut b = ReaderBuilder::new();
    b.has_headers(false).flexible(true);
    check_all_apis(
        &b,
        data,
        &[rec(&["a\"b", "c"]), rec(&["de", "f"]), rec(&["g"])],
    );
    b.strict(true).strict(false);
    check_all_apis(
        &b,
        data,
        &[rec(&["a\"b", "c"]), rec(&["de", "f"]), rec(&["g"])],
    );
}

#[test]
fn strict_does_nothing_without_quoting() {
    let mut b = strict();
    b.has_headers(false).flexible(true).quoting(false);
    check_all_apis(
        &b,
        "a\"b,c\n\"d\"e,f\n\"g",
        &[rec(&["a\"b", "c"]), rec(&["\"d\"e", "f"]), rec(&["\"g"])],
    );
}

#[test]
fn error_position_is_the_record_position() {
    let data = "x,y\n\n\"a\nb\",c\n\"p\"q,r\ns,t\n\"u";
    let mut lenient = ReaderBuilder::new();
    lenient.has_headers(false).flexible(true);
    let positions: Vec<Pos> = lenient
        .from_reader(data.as_bytes())
        .byte_records()
        .map(|r| {
            let r = r.unwrap();
            let p = r.position().unwrap();
            (p.byte(), p.line(), p.record())
        })
        .collect();
    assert_eq!(positions.len(), 5);

    let mut b = strict();
    b.has_headers(false).flexible(true);
    let mut rdr = b.from_reader(data.as_bytes());
    let mut got = vec![];
    let mut record = ByteRecord::new();
    loop {
        match rdr.read_byte_record(&mut record) {
            Ok(true) => {
                let p = record.position().unwrap();
                got.push((p.byte(), p.line(), p.record()));
            }
            Ok(false) => break,
            Err(err) => got.push(invalid_pos(&err)),
        }
    }
    assert_eq!(got, positions);
}

#[test]
fn invalid_header_row_is_reported_by_headers() {
    let data = "a\"b,c\nh1,h2\nx,y\n";
    let mut rdr = strict().from_reader(data.as_bytes());
    let err = rdr.headers().unwrap_err();
    assert_eq!(invalid_pos(&err), (0, 1, 0));
    assert_eq!(rdr.headers().unwrap(), vec!["h1", "h2"]);
    let got: Vec<Item> = rdr.records().map(item).collect();
    assert_eq!(got, vec![rec(&["x", "y"])]);
    assert_eq!(rdr.headers().unwrap(), vec!["h1", "h2"]);

    let mut rdr = strict().from_reader(data.as_bytes());
    let err = rdr.byte_headers().unwrap_err();
    assert_eq!(invalid_pos(&err), (0, 1, 0));
    assert_eq!(rdr.byte_headers().unwrap(), vec!["h1", "h2"]);
}

#[test]
fn invalid_header_row_is_reported_by_first_read() {
    let data = "a\"b,c\n\"h\"1,h2\nh1,h2\nx,y\n";
    check_all_apis(
        &strict(),
        data,
        &[Item::Bad((0, 1, 0)), Item::Bad((6, 2, 1)), rec(&["x", "y"])],
    );
    let mut rdr = strict().from_reader(data.as_bytes());
    let got: Vec<Item> = rdr.byte_records().map(byte_item).collect();
    assert_eq!(got.len(), 3);
    assert_eq!(rdr.headers().unwrap(), vec!["h1", "h2"]);
}

#[test]
fn record_numbers_count_invalid_records() {
    let mut rdr = strict().from_reader("\"h\"x\nh\na\n\"b\"c\nd\n".as_bytes());
    let mut got = vec![];
    let mut record = StringRecord::new();
    loop {
        match rdr.read_record(&mut record) {
            Ok(true) => got.push((
                record.get(0).unwrap().to_string(),
                record.position().unwrap().record(),
            )),
            Ok(false) => break,
            Err(err) => got.push(("error".to_string(), invalid_pos(&err).2)),
        }
    }
    assert_eq!(
        got,
        vec![
            ("error".to_string(), 0),
            ("a".to_string(), 2),
            ("error".to_string(), 3),
            ("d".to_string(), 4)
        ]
    );
    assert_eq!(rdr.position().record(), 5);
}

#[test]
fn invalid_records_are_not_compared_for_length() {
    let mut b = strict();
    b.has_headers(false).flexible(false);
    check_all_apis(
        &b,
        "a,b\n\"x\"y\nc,d\n\"p\"q,r,s\ne,f\n",
        &[
            rec(&["a", "b"]),
            Item::Bad((4, 2, 1)),
            rec(&["c", "d"]),
            Item::Bad((13, 4, 3)),
            rec(&["e", "f"]),
        ],
    );

    let mut rdr = b.from_reader("\"x\"y,1,2\na,b\nc,d,e\n".as_bytes());
    let mut it = rdr.records();
    assert_eq!(invalid_pos(&it.next().unwrap().unwrap_err()), (0, 1, 0));
    assert_eq!(it.next().unwrap().unwrap(), vec!["a", "b"]);
    match *it.next().unwrap().unwrap_err().kind() {
        ErrorKind::UnequalLengths { expected_len, len, .. } => {
            assert_eq!((expected_len, len), (2, 3));
        }
        ref other => panic!("expected UnequalLengths, got {:?}", other),
    }

    let mut rdr = strict()
        .flexible(false)
        .from_reader("h1,h2\n\"a\"b\nc,d\n".as_bytes());
    let got: Vec<Item> = rdr.records().map(item).collect();
    assert_eq!(got, vec![Item::Bad((6, 2, 1)), rec(&["c", "d"])]);
}

#[derive(Debug, Deserialize, PartialEq)]
struct Row {
    name: String,
    n: u32,
}

fn row(name: &str, n: u32) -> Row {
    Row { name: name.to_string(), n }
}

#[test]
fn deserialize_reports_invalid_header_rows() {
    let data = "\"n\"x,name\n\"n\"ame,n\nn,name\n1,bob\n\"2\"x,al\n3,cat\n";
    let want = vec![
        Err((0, 1, 0)),
        Err((10, 2, 1)),
        Ok(row("bob", 1)),
        Err((32, 5, 4)),
        Ok(row("cat", 3)),
    ];

    let mut rdr = strict().from_reader(data.as_bytes());
    let got: Vec<Result<Row, Pos>> =
        rdr.deserialize().map(|r| r.map_err(|e| invalid_pos(&e))).collect();
    assert_eq!(got, want, "deserialize()");
    assert_eq!(rdr.headers().unwrap(), vec!["n", "name"]);

    let got: Vec<Result<Row, Pos>> = strict()
        .from_reader(data.as_bytes())
        .into_deserialize()
        .map(|r| r.map_err(|e| invalid_pos(&e)))
        .collect();
    assert_eq!(got, want, "into_deserialize()");
}

#[test]
fn deserialize_without_headers_reports_invalid_rows() {
    let mut b = strict();
    b.has_headers(false);
    let data = "bob,1\n\"al\"x,2\ncat,3\n";
    let got: Vec<Result<Row, Pos>> = b
        .from_reader(data.as_bytes())
        .deserialize()
        .map(|r| r.map_err(|e| invalid_pos(&e)))
        .collect();
    assert_eq!(
        got,
        vec![Ok(row("bob", 1)), Err((6, 2, 1)), Ok(row("cat", 3))]
    );
}

#[test]
fn invalid_quoting_error_is_displayable() {
    let mut rdr = strict().has_headers(false).from_reader("a\"b\n".as_bytes());
    let err = rdr.records().next().unwrap().unwrap_err();
    assert!(!err.to_string().is_empty());
    assert!(!err.is_io_error());
}

// Deterministic generator of CSV documents whose invalid records are known
// by construction.

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

#[derive(Clone, Copy, Debug)]
struct Config {
    delimiter: u8,
    quote: u8,
    escape: Option<u8>,
    double_quote: bool,
    comment: Option<u8>,
    term: &'static [u8],
}

impl Config {
    fn builder(&self) -> ReaderBuilder {
        let mut b = ReaderBuilder::new();
        b.delimiter(self.delimiter)
            .quote(self.quote)
            .escape(self.escape)
            .double_quote(self.double_quote)
            .comment(self.comment)
            .has_headers(false)
            .flexible(true);
        if self.term == b"~" {
            b.terminator(Terminator::Any(b'~'));
        }
        b
    }
}

fn plain(rng: &mut Rng, min: u64) -> Vec<u8> {
    let n = min + rng.below(4);
    (0..n).map(|_| rng.pick(b"abcxyz019 ")).collect()
}

fn quoted(rng: &mut Rng, cfg: &Config) -> Vec<u8> {
    let mut v = vec![cfg.quote];
    for _ in 0..rng.below(6) {
        match rng.below(6) {
            0 => v.push(cfg.delimiter),
            1 => v.push(b'\n'),
            2 => v.extend(cfg.term),
            3 => {
                if cfg.double_quote {
                    v.extend([cfg.quote, cfg.quote]);
                } else if let Some(e) = cfg.escape {
                    v.extend([e, cfg.quote]);
                }
            }
            _ => v.push(rng.pick(b"abcxyz ")),
        }
    }
    v.push(cfg.quote);
    v
}

/// Returns a document and, per record, whether it has invalid quoting.
fn gen_doc(rng: &mut Rng, cfg: &Config) -> (Vec<u8>, Vec<bool>) {
    let mut data = vec![];
    let mut invalid = vec![];
    let nrec = 1 + rng.below(6);
    for i in 0..nrec {
        if cfg.comment.is_some() && rng.below(4) == 0 {
            data.push(b'#');
            data.extend(plain(rng, 0));
            data.push(cfg.quote);
            data.push(b'\n');
        }
        let mut bad = false;
        for j in 0..1 + rng.below(4) {
            if j > 0 {
                data.push(cfg.delimiter);
            }
            match rng.below(5) {
                0 => data.extend(plain(rng, if j == 0 { 1 } else { 0 })),
                1 => data.extend(quoted(rng, cfg)),
                2 => {
                    data.extend(plain(rng, 1));
                    data.push(cfg.quote);
                    data.extend(plain(rng, 0));
                    bad = true;
                }
                3 => {
                    data.extend(quoted(rng, cfg));
                    data.push(rng.pick(b"xz "));
                    bad = true;
                }
                _ => data.extend(quoted(rng, cfg)),
            }
        }
        let last = i + 1 == nrec;
        if last && rng.below(3) == 0 {
            data.push(cfg.delimiter);
            let q = quoted(rng, cfg);
            data.extend(&q[..q.len() - 1]);
            bad = true;
        } else if !last || rng.below(2) == 0 {
            data.extend(cfg.term);
        }
        invalid.push(bad);
    }
    (data, invalid)
}

fn configs() -> Vec<Config> {
    let mut v = vec![];
    for &(delimiter, quote) in &[(b',', b'"'), (b';', b'\''), (b'\t', b'"')] {
        for &(double_quote, escape) in
            &[(true, None), (false, Some(b'\\')), (false, None)]
        {
            for &term in &[&b"\n"[..], &b"\r\n"[..], &b"~"[..]] {
                for &comment in &[None, Some(b'#')] {
                    if comment.is_some() && term == b"~" {
                        continue;
                    }
                    v.push(Config {
                        delimiter,
                        quote,
                        escape,
                        double_quote,
                        comment,
                        term,
                    });
                }
            }
        }
    }
    v
}

#[derive(Debug, PartialEq)]
enum Out {
    Rec(Vec<Vec<u8>>, Pos),
    Bad(Pos),
    Unequal(u64, u64, Pos),
}

fn read_all(builder: &ReaderBuilder, data: &[u8]) -> Vec<Out> {
    let mut rdr = builder.from_reader(data);
    let mut out = vec![];
    let mut record = ByteRecord::new();
    for _ in 0..1000 {
        match rdr.read_byte_record(&mut record) {
            Ok(true) => {
                let p = record.position().unwrap();
                out.push(Out::Rec(
                    record.iter().map(|f| f.to_vec()).collect(),
                    (p.byte(), p.line(), p.record()),
                ));
            }
            Ok(false) => return out,
            Err(err) => match *err.kind() {
                ErrorKind::UnequalLengths { ref pos, expected_len, len } => {
                    let p = pos.clone().unwrap();
                    out.push(Out::Unequal(
                        expected_len,
                        len,
                        (p.byte(), p.line(), p.record()),
                    ));
                }
                _ => out.push(Out::Bad(invalid_pos(&err))),
            },
        }
    }
    panic!("reader did not finish");
}

#[test]
fn generated_documents() {
    let mut rng = Rng(0xc5f_a781);
    for cfg in configs() {
        for _ in 0..25 {
            let (data, invalid) = gen_doc(&mut rng, &cfg);
            let shown = String::from_utf8_lossy(&data).into_owned();
            let lenient = read_all(&cfg.builder(), &data);
            assert_eq!(
                lenient.len(),
                invalid.len(),
                "generator mismatch for {:?} {:?}",
                cfg,
                shown
            );

            // flexible: every invalid record becomes an error at its position.
            let want: Vec<Out> = lenient
                .iter()
                .zip(&invalid)
                .map(|(o, &bad)| match *o {
                    Out::Rec(_, p) if bad => Out::Bad(p),
                    Out::Rec(ref f, p) => Out::Rec(f.clone(), p),
                    _ => unreachable!(),
                })
                .collect();
            let mut b = cfg.builder();
            b.strict(true);
            assert_eq!(read_all(&b, &data), want, "{:?} {:?}", cfg, shown);

            // not flexible: invalid records neither set nor break the length.
            let mut expected_len = None;
            let want: Vec<Out> = lenient
                .iter()
                .zip(&invalid)
                .map(|(o, &bad)| match *o {
                    Out::Rec(_, p) if bad => Out::Bad(p),
                    Out::Rec(ref f, p) => match expected_len {
                        None => {
                            expected_len = Some(f.len() as u64);
                            Out::Rec(f.clone(), p)
                        }
                        Some(n) if n != f.len() as u64 => {
                            Out::Unequal(n, f.len() as u64, p)
                        }
                        Some(_) => Out::Rec(f.clone(), p),
                    },
                    _ => unreachable!(),
                })
                .collect();
            b.flexible(false);
            assert_eq!(
                read_all(&b, &data),
                want,
                "not flexible: {:?} {:?}",
                cfg,
                shown
            );

            // headers: the first record that is not invalid.
            let mut b = cfg.builder();
            b.strict(true).has_headers(true);
            let mut rdr = b.from_reader(&data[..]);
            let first_good = invalid.iter().position(|&x| !x);
            let mut errors = 0;
            let headers = loop {
                match rdr.byte_headers() {
                    Ok(h) => break h.clone(),
                    Err(err) => {
                        invalid_pos(&err);
                        errors += 1;
                        assert!(
                            errors <= invalid.len(),
                            "too many header errors: {:?}",
                            shown
                        );
                    }
                }
            };
            match first_good {
                Some(i) => {
                    assert_eq!(errors, i, "{:?}", shown);
                    match lenient[i] {
                        Out::Rec(ref f, _) => assert_eq!(
                            headers,
                            ByteRecord::from(f.clone()),
                            "{:?}",
                            shown
                        ),
                        _ => unreachable!(),
                    }
                }
                None => {
                    assert_eq!(errors, invalid.len(), "{:?}", shown);
                    assert!(headers.is_empty(), "{:?}", shown);
                }
            }
        }
    }
}
