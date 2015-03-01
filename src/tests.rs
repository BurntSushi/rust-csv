use std::borrow::ToOwned;
use std::old_io as io;
use std::old_io::ByRefReader;
use std::old_io::Reader as IoReader;
use std::old_io::Writer as IoWriter;
use {
    Reader, Writer, ByteString, CsvResult,
    IntoVector, RecordTerminator, QuoteStyle,
};

fn assert_svec_eq<S: Str, T: Str>(got: Vec<Vec<S>>, expected: Vec<Vec<T>>) {
    let got: Vec<Vec<&str>> =
        got.iter().map(|row| {
            row.iter().map(|f| f.as_slice()).collect()
        }).collect();
    let expected: Vec<Vec<&str>> =
        expected.iter().map(|row| {
            row.iter().map(|f| f.as_slice()).collect()
        }).collect();

    println!("got len: {}, expected len: {}", got.len(), expected.len());
    println!("got lengths: {:?}",
             got.iter().map(|row: &Vec<&str>| row.len())
                       .collect::<Vec<usize>>());
    println!("expected lengths: {:?}",
             expected.iter().map(|row: &Vec<&str>| row.len())
                            .collect::<Vec<usize>>());
    assert_eq!(got, expected);
}

macro_rules! parses_to {
    ($name:ident, $csv:expr, $vec:expr) => (
        parses_to!($name, $csv, $vec, |rdr| rdr);
    );
    ($name:ident, $csv:expr, $vec:expr, $config:expr) => (
        #[test]
        fn $name() {
            let mut rdr = Reader::from_string($csv).has_headers(false);
            rdr = $config(rdr);
            let rows = rdr.records()
                          .collect::<Result<Vec<Vec<String>>, _>>()
                          .unwrap();
            assert_svec_eq::<String, &str>(rows, $vec);
        }
    );
}

macro_rules! fail_parses_to {
    ($name:ident, $csv:expr, $vec:expr) => (
        fail_parses_to!($name, $csv, $vec, |rdr| rdr);
    );
    ($name:ident, $csv:expr, $vec:expr, $config:expr) => (
        #[test]
        #[should_fail]
        fn $name() {
            let mut rdr = Reader::from_string($csv).has_headers(false);
            rdr = $config(rdr);
            let rows = rdr.records()
                          .collect::<Result<Vec<Vec<String>>, _>>()
                          .unwrap();
            assert_svec_eq::<String, &str>(rows, $vec);
        }
    );
}

macro_rules! decodes_to {
    ($name:ident, $csv:expr, $ty:ty, $vec:expr) => (
        decodes_to!($name, $csv, $ty, $vec, false);
    );
    ($name:ident, $csv:expr, $ty:ty, $vec:expr, $headers:expr) => (
        #[test]
        fn $name() {
            let mut rdr = Reader::from_string($csv)
                                 .has_headers($headers);
            let rows = rdr.decode()
                          .collect::<Result<Vec<$ty>, _>>()
                          .unwrap();
            assert_eq!(rows, $vec);
        }
    );
}

macro_rules! writes_as {
    ($name:ident, $vec:expr, $csv:expr) => (
        writes_as!($name, $vec, $csv, |wtr| wtr);
    );
    ($name:ident, $vec:expr, $csv:expr, $config:expr) => (
        #[test]
        fn $name() {
            let mut wtr = $config(Writer::from_memory());
            for row in $vec.into_iter() {
                wtr.write(row.into_iter()).unwrap();
            }
            assert_eq!(wtr.as_string(), $csv);
        }
    );
}

macro_rules! fail_writes_as {
    ($name:ident, $vec:expr, $csv:expr) => (
        fail_writes_as!($name, $vec, $csv, |wtr| wtr);
    );
    ($name:ident, $vec:expr, $csv:expr, $config:expr) => (
        #[test]
        #[should_fail]
        fn $name() {
            let mut wtr = $config(Writer::from_memory());
            for row in $vec.into_iter() {
                wtr.write(row.into_iter()).unwrap();
            }
            assert_eq!(wtr.as_string(), $csv);
        }
    );
}

macro_rules! encodes_as {
    ($name:ident, $vec:expr, $csv:expr) => (
        #[test]
        fn $name() {
            let mut wtr = Writer::from_memory();
            for row in $vec.into_iter() {
                wtr.encode(row).unwrap();
            }
            assert_eq!(wtr.as_string(), $csv);
        }
    );
}

parses_to!(one_row_one_field, "a", vec![vec!["a"]]);
parses_to!(one_row_many_fields, "a,b,c", vec![vec!["a", "b", "c"]]);
parses_to!(one_row_trailing_comma, "a,b,", vec![vec!["a", "b", ""]]);
parses_to!(one_row_one_field_lf, "a\n", vec![vec!["a"]]);
parses_to!(one_row_many_fields_lf, "a,b,c\n", vec![vec!["a", "b", "c"]]);
parses_to!(one_row_trailing_comma_lf, "a,b,\n", vec![vec!["a", "b", ""]]);
parses_to!(one_row_one_field_crlf, "a\r\n", vec![vec!["a"]]);
parses_to!(one_row_many_fields_crlf, "a,b,c\r\n", vec![vec!["a", "b", "c"]]);
parses_to!(one_row_trailing_comma_crlf, "a,b,\r\n", vec![vec!["a", "b", ""]]);
parses_to!(one_row_one_field_cr, "a\r", vec![vec!["a"]]);
parses_to!(one_row_many_fields_cr, "a,b,c\r", vec![vec!["a", "b", "c"]]);
parses_to!(one_row_trailing_comma_cr, "a,b,\r", vec![vec!["a", "b", ""]]);

parses_to!(many_rows_one_field, "a\nb", vec![vec!["a"], vec!["b"]]);
parses_to!(many_rows_many_fields,
           "a,b,c\nx,y,z", vec![vec!["a", "b", "c"], vec!["x", "y", "z"]]);
parses_to!(many_rows_trailing_comma,
           "a,b,\nx,y,", vec![vec!["a", "b", ""], vec!["x", "y", ""]]);
parses_to!(many_rows_one_field_lf, "a\nb\n", vec![vec!["a"], vec!["b"]]);
parses_to!(many_rows_many_fields_lf,
           "a,b,c\nx,y,z\n", vec![vec!["a", "b", "c"], vec!["x", "y", "z"]]);
parses_to!(many_rows_trailing_comma_lf,
           "a,b,\nx,y,\n", vec![vec!["a", "b", ""], vec!["x", "y", ""]]);
parses_to!(many_rows_one_field_crlf, "a\r\nb\r\n", vec![vec!["a"], vec!["b"]]);
parses_to!(many_rows_many_fields_crlf,
           "a,b,c\r\nx,y,z\r\n",
           vec![vec!["a", "b", "c"], vec!["x", "y", "z"]]);
parses_to!(many_rows_trailing_comma_crlf,
           "a,b,\r\nx,y,\r\n", vec![vec!["a", "b", ""], vec!["x", "y", ""]]);
parses_to!(many_rows_one_field_cr, "a\rb\r", vec![vec!["a"], vec!["b"]]);
parses_to!(many_rows_many_fields_cr,
           "a,b,c\rx,y,z\r", vec![vec!["a", "b", "c"], vec!["x", "y", "z"]]);
parses_to!(many_rows_trailing_comma_cr,
           "a,b,\rx,y,\r", vec![vec!["a", "b", ""], vec!["x", "y", ""]]);

parses_to!(trailing_lines_no_record,
           "\n\n\na,b,c\nx,y,z\n\n\n",
           vec![vec!["a", "b", "c"], vec!["x", "y", "z"]]);
parses_to!(trailing_lines_no_record_cr,
           "\r\r\ra,b,c\rx,y,z\r\r\r",
           vec![vec!["a", "b", "c"], vec!["x", "y", "z"]]);
parses_to!(trailing_lines_no_record_crlf,
           "\r\n\r\n\r\na,b,c\r\nx,y,z\r\n\r\n\r\n",
           vec![vec!["a", "b", "c"], vec!["x", "y", "z"]]);
parses_to!(empty_string_no_headers, "", vec![]);
parses_to!(empty_string_headers, "", vec![],
           |rdr: Reader<_>| rdr.has_headers(true));
parses_to!(empty_lines, "\n\n\n\n", vec![]);
parses_to!(empty_lines_interspersed, "\n\na,b\n\n\nx,y\n\n\nm,n\n",
           vec![vec!["a", "b"], vec!["x", "y"], vec!["m", "n"]]);
parses_to!(empty_lines_crlf, "\r\n\r\n\r\n\r\n", vec![]);
parses_to!(empty_lines_interspersed_crlf,
           "\r\n\r\na,b\r\n\r\n\r\nx,y\r\n\r\n\r\nm,n\r\n",
           vec![vec!["a", "b"], vec!["x", "y"], vec!["m", "n"]]);
parses_to!(empty_lines_mixed, "\r\n\n\r\n\n", vec![]);
parses_to!(empty_lines_interspersed_mixed,
           "\n\r\na,b\r\n\n\r\nx,y\r\n\n\r\nm,n\r\n",
           vec![vec!["a", "b"], vec!["x", "y"], vec!["m", "n"]]);
parses_to!(empty_lines_cr, "\r\r\r\r", vec![]);
parses_to!(empty_lines_interspersed_cr, "\r\ra,b\r\r\rx,y\r\r\rm,n\r",
           vec![vec!["a", "b"], vec!["x", "y"], vec!["m", "n"]]);

parses_to!(term_weird, "zza,bzc,dzz", vec![vec!["a", "b"], vec!["c", "d"]],
           |rdr: Reader<_>| rdr.record_terminator(RecordTerminator::Any(b'z')));

parses_to!(ascii_delimited, "a\x1fb\x1ec\x1fd",
           vec![vec!["a", "b"], vec!["c", "d"]],
           |rdr: Reader<_>| {
               rdr.delimiter(b'\x1f')
                  .record_terminator(RecordTerminator::Any(b'\x1e'))
           });

parses_to!(quote_empty, "\"\"", vec![vec![""]]);
parses_to!(quote_lf, "\"\"\n", vec![vec![""]]);
parses_to!(quote_space, "\" \"", vec![vec![" "]]);
parses_to!(quote_inner_space, "\" a \"", vec![vec![" a "]]);
parses_to!(quote_outer_space, "  \"a\"  ", vec![vec!["  \"a\"  "]]);

parses_to!(quote_change, "zaz", vec![vec!["a"]],
           |rdr: Reader<_>| rdr.quote(Some(b'z')));

// This one is pretty hokey. I don't really know what the "right" behavior is.
parses_to!(quote_delimiter, ",a,,b", vec![vec!["a,b"]],
           |rdr: Reader<_>| rdr.quote(Some(b',')));

// Another hokey one...
parses_to!(quote_no_escapes, r#""a\"b""#, vec![vec![r#"a\b""#]]);
parses_to!(quote_escapes_no_double, r#""a""b""#, vec![vec![r#"a"b""#]],
           |rdr: Reader<_>| rdr.double_quote(false));
parses_to!(quote_escapes, r#""a\"b""#, vec![vec![r#"a"b"#]],
           |rdr: Reader<_>| rdr.double_quote(false));
parses_to!(quote_escapes_change, r#""az"b""#, vec![vec![r#"a"b"#]],
           |rdr: Reader<_>| rdr.double_quote(false).escape(b'z'));

parses_to!(delimiter_tabs, "a\tb", vec![vec!["a", "b"]],
           |rdr: Reader<_>| rdr.delimiter(b'\t'));
parses_to!(delimiter_weird, "azb", vec![vec!["a", "b"]],
           |rdr: Reader<_>| rdr.delimiter(b'z'));

parses_to!(headers_absent, "a\nb", vec![vec!["b"]],
           |rdr: Reader<_>| rdr.has_headers(true));

parses_to!(flexible_rows, "a\nx,y", vec![vec!["a"], vec!["x", "y"]],
           |rdr: Reader<_>| rdr.flexible(true));
parses_to!(flexible_rows2, "a,b\nx", vec![vec!["a", "b"], vec!["x"]],
           |rdr: Reader<_>| rdr.flexible(true));

fail_parses_to!(nonflexible, "a\nx,y", vec![]);
fail_parses_to!(nonflexible2, "a,b\nx", vec![]);

#[derive(Debug, RustcDecodable, RustcEncodable, PartialEq, Eq)]
enum Val { Unsigned(usize), Signed(isize), Bool(bool) }

decodes_to!(decode_int, "1", (usize,), vec![(1usize,)]);
decodes_to!(decode_many_int, "1,2", (usize, i16), vec![(1usize, 2i16)]);
decodes_to!(decode_float, "1,1.0,1.5",
            (f64, f64, f64), vec![(1f64, 1.0, 1.5)]);
decodes_to!(decode_char, "a", (char,), vec![('a',)]);
decodes_to!(decode_str, "abc", (String,), vec![("abc".to_owned(),)]);

decodes_to!(decode_opt_int, "a", (Option<usize>,), vec![(None,)]);
decodes_to!(decode_opt_float, "a", (Option<f64>,), vec![(None,)]);
decodes_to!(decode_opt_char, "ab", (Option<char>,), vec![(None,)]);
decodes_to!(decode_opt_empty, "\"\"", (Option<String>,), vec![(None,)]);

decodes_to!(decode_val, "false,-5,5", (Val, Val, Val),
            vec![(Val::Bool(false), Val::Signed(-5), Val::Unsigned(5))]);
decodes_to!(decode_opt_val, "1.0", (Option<Val>,), vec![(None,)]);

decodes_to!(decode_tail, "abc,1,2,3,4", (String, Vec<usize>),
            vec![("abc".to_owned(), vec![1usize, 2, 3, 4])]);

writes_as!(wtr_one_record_one_field, vec![vec!["a"]], "a\n");
writes_as!(wtr_one_record_many_field, vec![vec!["a", "b"]], "a,b\n");
writes_as!(wtr_many_record_one_field, vec![vec!["a"], vec!["b"]], "a\nb\n");
writes_as!(wtr_many_record_many_field,
           vec![vec!["a", "b"], vec!["x", "y"]], "a,b\nx,y\n");
writes_as!(wtr_one_record_one_field_crlf, vec![vec!["a"]], "a\r\n",
           |wtr: Writer<_>| wtr.record_terminator(RecordTerminator::CRLF));
writes_as!(wtr_one_record_many_field_crlf, vec![vec!["a", "b"]], "a,b\r\n",
           |wtr: Writer<_>| wtr.record_terminator(RecordTerminator::CRLF));
writes_as!(wtr_many_record_one_field_crlf,
           vec![vec!["a"], vec!["b"]], "a\r\nb\r\n",
           |wtr: Writer<_>| wtr.record_terminator(RecordTerminator::CRLF));
writes_as!(wtr_many_record_many_field_crlf,
           vec![vec!["a", "b"], vec!["x", "y"]], "a,b\r\nx,y\r\n",
           |wtr: Writer<_>| wtr.record_terminator(RecordTerminator::CRLF));

writes_as!(wtr_tabs, vec![vec!["a", "b"]], "a\tb\n",
           |wtr: Writer<_>| wtr.delimiter(b'\t'));
writes_as!(wtr_weird, vec![vec!["a", "b"]], "azb\n",
           |wtr: Writer<_>| wtr.delimiter(b'z'));
writes_as!(wtr_flexible, vec![vec!["a"], vec!["a", "b"]], "a\na,b\n",
           |wtr: Writer<_>| wtr.flexible(true));
writes_as!(wtr_flexible2, vec![vec!["a", "b"], vec!["a"]], "a,b\na\n",
           |wtr: Writer<_>| wtr.flexible(true));

writes_as!(wtr_quoted_lf, vec![vec!["a\n"]], "\"a\n\"\n");
writes_as!(wtr_quoted_cr, vec![vec!["a\r"]], "\"a\r\"\n");
writes_as!(wtr_quoted_quotes, vec![vec!["\"a\""]], r#""""a"""
"#);
writes_as!(wtr_quoted_delim, vec![vec!["a,b"]], "\"a,b\"\n");

writes_as!(wtr_empty_row, vec![vec![""]], "\"\"\n");
writes_as!(wtr_empty_rows, vec![vec![""], vec!["a"], vec![""]],
           "\"\"\na\n\"\"\n");
writes_as!(wtr_empty_field_one_row, vec![vec!["", "a"]], ",a\n");
writes_as!(wtr_empty_field_rows,
           vec![vec!["", "a"], vec!["a", "b"], vec!["", "a"]],
           ",a\na,b\n,a\n");

writes_as!(wtr_always_quote, vec![vec!["a"]], "\"a\"\n",
           |wtr: Writer<_>| wtr.quote_style(QuoteStyle::Always));
writes_as!(wtr_never_quote, vec![vec!["a"]], "a\n",
           |wtr: Writer<_>| wtr.quote_style(QuoteStyle::Never));

writes_as!(wtr_escape, vec![vec!["a\"b"]], "\"a\\\"b\"\n",
           |wtr: Writer<_>| wtr.double_quote(false));
writes_as!(wtr_escape_weird, vec![vec!["a\"b"]], "\"az\"b\"\n",
           |wtr: Writer<_>| wtr.double_quote(false).escape(b'z'));

writes_as!(wtr_quote_weird, vec![vec!["a,b"]], "za,bz\n",
           |wtr: Writer<_>| wtr.quote(b'z'));

fail_writes_as!(wtr_no_rows,
                { let rows: Vec<Vec<&str>> = vec![vec![]]; rows }, "");
fail_writes_as!(wtr_noflexible, vec![vec!["a"], vec!["a", "b"]], "a\na,b\n");
fail_writes_as!(wtr_noflexible2, vec![vec!["a", "b"], vec!["a"]], "a,b\na\n");
fail_writes_as!(wtr_never_quote_needs_quotes, vec![vec![","]], "\",\"",
                |wtr: Writer<_>| wtr.quote_style(QuoteStyle::Never));

encodes_as!(encode_int, vec![(1usize,)], "1\n");
encodes_as!(encode_many_int, vec![(1usize, 2i16)], "1,2\n");
encodes_as!(encode_float, vec![(1f64, 1.0f64, 1.5f64)], "1,1,1.5\n");
encodes_as!(encode_char, vec![('a',)], "a\n");
encodes_as!(encode_none, vec![(None::<bool>,)], "\"\"\n");
encodes_as!(encode_some, vec![(Some(true),)], "true\n");
encodes_as!(encode_val,
            vec![(Val::Bool(false), Val::Signed(-5), Val::Unsigned(5))],
            "false,-5,5\n");

#[test]
fn no_headers_no_skip_one_record() {
    let mut d = Reader::from_string("a,b").has_headers(false);
    d.headers().unwrap();
    let rows: Vec<CsvResult<Vec<String>>> = d.records().collect();
    assert_eq!(rows.len(), 1);
}

#[test]
fn no_headers_first_record() {
    let mut d = Reader::from_string("a,b").has_headers(false);
    let r = d.headers().unwrap();
    assert_eq!(r, vec!("a".to_string(), "b".to_string()));
}

#[test]
fn no_headers_no_skip() {
    let mut d = Reader::from_string("a,b\nc,d").has_headers(false);
    d.headers().unwrap();
    let rows: Vec<CsvResult<Vec<String>>> = d.records().collect();
    assert_eq!(rows.len(), 2);
}

#[test]
fn headers_trailing_lf() {
    let mut d = Reader::from_string("a,b,c\n\n\n\n");
    assert_eq!(d.headers().unwrap(),
               vec!("a".to_string(), "b".to_string(), "c".to_string()));
    assert!(d.next_field().is_end());
}

#[test]
fn headers_eof() {
    let mut d = Reader::from_string("");
    assert!(d.headers().is_ok());
    assert!(d.done());
}

fn bytes<S: IntoVector<u8>>(bs: S) -> ByteString {
    ByteString::from_bytes(bs)
}

#[test]
fn byte_strings() {
    let mut d = Reader::from_string("abc,xyz").has_headers(false);
    let r = d.byte_records().next().unwrap().unwrap();
    assert_eq!(r, vec![bytes(b"abc"), bytes(b"xyz")]);
}

#[test]
fn byte_strings_invalid_utf8() {
    let mut d = Reader::from_bytes(b"a\xffbc,xyz").has_headers(false);
    let r = d.byte_records().next().unwrap().unwrap();
    assert_eq!(r, vec![bytes(b"a\xffbc"), bytes(b"xyz")]);
}

#[test]
#[should_fail]
fn invalid_utf8() {
    let mut d = Reader::from_bytes(b"a\xffbc,xyz").has_headers(false);
    d.records().next().unwrap().unwrap();
}

#[test]
fn seeking() {
    let data = "1,2\n3,4\n5,6\n";
    let mut buf = io::MemReader::new(data.as_bytes().to_vec());

    {
        let mut d = Reader::from_reader(buf.by_ref()).has_headers(false);
        let vals =
            d.decode().collect::<Result<Vec<(usize, usize)>, _>>().unwrap();
        assert_eq!(vals, vec!((1, 2), (3, 4), (5, 6)));
    }

    buf.seek(0, io::SeekSet).unwrap();
    {
        let mut d = Reader::from_reader(buf.by_ref()).has_headers(false);
        let vals =
            d.decode().collect::<Result<Vec<(usize, usize)>, _>>().unwrap();
        assert_eq!(vals, vec!((1, 2), (3, 4), (5, 6)));
    }
}

#[test]
fn raw_access() {
    let mut rdr = Reader::from_string("1,2");
    let mut fields = vec![];
    while let Some(field) = rdr.next_field().into_iter_result() {
        fields.push(field.unwrap().to_vec());
    }
    assert_eq!(fields[0], b"1".to_vec());
}

// Please help me get this test to pass.
#[test]
#[ignore]
fn raw_unsafe_access() {
    let mut rdr = Reader::from_string("1,2");
    let fields = unsafe {
        rdr.byte_fields().collect::<Result<Vec<_>, _>>().unwrap()
    };
    assert_eq!(fields[0], b"1");
}
