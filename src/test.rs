use std::io::Reader as IoReader;
use std::io::Writer as IoWriter;
use {Reader, Writer, ByteString, CsvResult, collect, IntoVector};

fn ordie<T, E: ::std::fmt::Show>(res: Result<T, E>) -> T {
    match res {
        Ok(t) => t,
        Err(err) => fail!("{}", err),
    }
}

fn bytes<S: IntoVector<u8>>(bs: S) -> ByteString {
    ByteString::from_bytes(bs)
}

#[test]
fn encoder_simple() {
    let mut senc = Writer::from_memory();
    ordie(senc.encode(("springsteen", 's', 1i, 0.14f64, false)));
    assert_eq!("springsteen,s,1,0.14,false\n", senc.as_string());
}

#[test]
fn encoder_simple_crlf() {
    let mut senc = Writer::from_memory().crlf(true);
    ordie(senc.encode(("springsteen", 's', 1i, 0.14f64, false)));
    assert_eq!("springsteen,s,1,0.14,false\r\n", senc.as_string());
}

#[test]
fn encoder_simple_tabbed() {
    let mut senc = Writer::from_memory().delimiter(b'\t');
    ordie(senc.encode(("springsteen", 's', 1i, 0.14f64, false)));
    assert_eq!("springsteen\ts\t1\t0.14\tfalse\n", senc.as_string());
}

#[test]
fn encoder_same_length_records() {
    let mut senc = Writer::from_memory().flexible(false);
    ordie(senc.encode(vec!('a')));
    match senc.encode(vec!('a', 'b')) {
        Ok(_) => fail!("Writer should report an error when records of \
                        varying length are added and records of same \
                        length is enabled."),
        Err(_) => {}
    }
}

#[test]
fn encoder_quoted_quotes() {
    let mut senc = Writer::from_memory();
    ordie(senc.encode(vec!("sprin\"g\"steen")));
    assert_eq!("\"sprin\"\"g\"\"steen\"\n", senc.as_string());
}

#[test]
fn encoder_quoted_sep() {
    let mut senc = Writer::from_memory().delimiter(b',');
    ordie(senc.encode(vec!("spring,steen")));
    assert_eq!("\"spring,steen\"\n", senc.as_string());
}

#[test]
fn encoder_quoted_newlines() {
    let mut senc = Writer::from_memory();
    ordie(senc.encode(vec!("spring\nsteen")));
    assert_eq!("\"spring\nsteen\"\n", senc.as_string());
}

#[test]
fn encoder_zero() {
    let mut senc = Writer::from_memory();
    match senc.encode::<Vec<int>>(vec!()) {
        Ok(_) => fail!("Writer should report an error when trying to \
                        encode records of length 0."),
        Err(_) => {}
    }
}

#[test]
fn decoder_simple_nonl() {
    let mut d = Reader::from_string("springsteen,s,1,0.14,false")
                       .has_headers(false);
    let r: Vec<(String, char, int, f64, bool)> = ordie(collect(d.decode()));
    assert_eq!(r, vec![("springsteen".to_string(), 's', 1, 0.14, false)]);
}

#[test]
fn decoder_simple_nonl_comma() {
    let mut d = Reader::from_string("springsteen,s,").has_headers(false);
    let r: Vec<(String, char, Option<int>)> = ordie(collect(d.decode()));
    assert_eq!(r, vec![("springsteen".to_string(), 's', None)]);
}

#[test]
fn decoder_simple() {
    let mut d = Reader::from_string("springsteen,s,1,0.14,false\n")
                       .has_headers(false);
    let r: Vec<(String, char, int, f64, bool)> = ordie(collect(d.decode()));
    assert_eq!(r, vec![("springsteen".to_string(), 's', 1, 0.14, false)]);
}

#[test]
fn decoder_simple_crlf() {
    let mut d = Reader::from_string("springsteen,s,1,0.1,false\r\n")
                       .has_headers(false);
    let r: Vec<(String, char, int, f64, bool)> = ordie(collect(d.decode()));
    assert_eq!(r, vec![("springsteen".to_string(), 's', 1, 0.1, false)]);
}

#[test]
fn decoder_simple_tabbed() {
    let mut d = Reader::from_string("springsteen\ts\t1\t0.14\tfalse\r\n")
                       .has_headers(false)
                       .delimiter(b'\t');
    let r: Vec<(String, char, int, f64, bool)> = ordie(collect(d.decode()));
    assert_eq!(r, vec![("springsteen".to_string(), 's', 1, 0.14, false)]);
}

#[test]
fn decoder_same_length_records() {
    let mut d = Reader::from_string("a\na,b")
                       .has_headers(false)
                       .flexible(false);
    match collect(d.decode::<Vec<String>>()) {
        Ok(_) => fail!("Decoder should report an error when records of \
                        varying length are decoded and records of same \
                        length if enabled."),
        Err(_) => {}
    }
}

#[test]
fn decoder_different_length_records() {
    let mut d = Reader::from_string("a\na,b")
                        .has_headers(false)
                        .flexible(true);
    let rs = ordie(collect(d.decode::<Vec<String>>()));
    assert_eq!(rs, vec!(vec!("a".to_string()),
                        vec!("a".to_string(), "b".to_string())));
}

#[test]
fn decoder_headers() {
    let mut d = Reader::from_string("a,b,c\n1,2,3");
    assert_eq!(ordie(d.headers()),
               vec!("a".to_string(), "b".to_string(), "c".to_string()));

    let r: Vec<(uint, uint, uint)> = ordie(collect(d.decode()));
    assert_eq!(r, vec![(1, 2, 3)]);
}

#[test]
fn decoder_empty_lines() {
    let mut d = Reader::from_string("1,2\n\n3,4\n\n\n\n5,6\n\n")
                       .has_headers(false);
    let vals: Vec<(uint, uint)> = ordie(collect(d.decode()));
    assert_eq!(vals, vec!((1, 2), (3, 4), (5, 6)));
}

#[test]
fn decoder_empty_lines_crlf() {
    let mut d = Reader::from_string("1,2\r\n\r\n3,4\r\n\r\n\r\n\r\n5,6\r\n\r\n")
                        .has_headers(false);
    let vals: Vec<(uint, uint)> = ordie(collect(d.decode()));
    assert_eq!(vals, vec!((1, 2), (3, 4), (5, 6)));
}

#[test]
fn decoder_empties_headers() {
    let mut d = Reader::from_string("a,b,c\n\n\n\n");
    assert_eq!(ordie(d.headers()),
               vec!("a".to_string(), "b".to_string(), "c".to_string()));
    assert!(d.next_field().is_none());
}

#[test]
fn decoder_all_empties() {
    let mut d = Reader::from_string("\n\n\n\n").has_headers(false);
    assert!(d.next_field().is_none());
}

#[test]
fn decoder_all_empties_crlf() {
    let mut d = Reader::from_string("\r\n\r\n\r\n\r\n").has_headers(false);
    assert!(d.next_field().is_none());
}

#[test]
fn decoder_empty_strings() {
    let mut d = Reader::from_string("\"\"").has_headers(false);
    let r: Vec<(String,)> = ordie(collect(d.decode()));
    assert_eq!(r, vec![("".to_string(),)]);
}

#[test]
fn decoder_quotes() {
    let mut d = Reader::from_string("\" a \",   \"1\"   ,\"1\",  1  ")
                        .has_headers(false);
    let r: Vec<(String, String, uint, uint)> = ordie(collect(d.decode()));
    assert_eq!(r, vec![(" a ".to_string(), "   \"1\"   ".to_string(), 1, 1)]);
}

#[test]
fn decoder_headers_eof() {
    let mut d = Reader::from_string("");
    assert!(d.headers().is_ok());
    assert!(d.done());
}

#[test]
fn decoder_no_headers_first_record() {
    let mut d = Reader::from_string("a,b").has_headers(false);
    let r = ordie(d.headers());
    assert_eq!(r, vec!("a".to_string(), "b".to_string()));
}

#[test]
fn decoder_no_headers_no_skip() {
    let mut d = Reader::from_string("a,b\nc,d").has_headers(false);
    let _ = ordie(d.headers());
    let rows: Vec<CsvResult<Vec<String>>> = d.records().collect();
    assert_eq!(rows.len(), 2);
}

#[test]
fn decoder_empty_string() {
    let mut d = Reader::from_string("");
    let rows: Vec<CsvResult<Vec<String>>> = d.records().collect();
    assert!(rows.len() == 0);
}

#[deriving(Decodable, Encodable, Show, PartialEq, Eq)]
enum Val {
    Unsigned(uint),
    Signed(int),
    Bool(bool),
}

#[deriving(Show, PartialEq, Eq, Encodable, Decodable)]
enum Color {
    Red, Green, Blue
}

#[test]
fn decoder_enum() {
    let mut d = Reader::from_string("ReD").has_headers(false);
    let r: Vec<(Color,)> = ordie(collect(d.decode()));
    assert_eq!(r, vec![(Red,)]);
}

#[test]
fn decoder_enum_arg() {
    let mut d = Reader::from_string("false,-5,5").has_headers(false);
    let r: Vec<(Val, Val, Val)> = ordie(collect(d.decode()));
    assert_eq!(r, vec![(Bool(false), Signed(-5), Unsigned(5))]);
}

#[test]
fn decoder_option() {
    let mut d = Reader::from_string(",1").has_headers(false);
    let r: Vec<(Option<bool>, uint)> = ordie(collect(d.decode()));
    assert_eq!(r, vec![(None, 1)]);
}

#[test]
fn encoder_enum() {
    let r = (Red,);
    let mut senc = Writer::from_memory();
    ordie(senc.encode(r));
    assert_eq!("Red\n", senc.as_string());
}

#[test]
fn encoder_enum_arg() {
    let r = (Bool(false), Signed(-5), Unsigned(5));
    let mut senc = Writer::from_memory();
    ordie(senc.encode(r));
    assert_eq!("false,-5,5\n", senc.as_string());
}

#[test]
fn encoder_option() {
    let r: (Option<bool>, uint) = (None, 1);
    let mut senc = Writer::from_memory();
    ordie(senc.encode(r));
    assert_eq!(",1\n", senc.as_string());
}

#[test]
fn trailing_lines_no_record() {
    let s = "
a,b,c
d,e,f
";
    let mut rdr = Reader::from_string(s);
    let mut count = 0u;
    while !rdr.done() {
        loop {
            match rdr.next_field() {
                None => break,
                Some(r) => { ordie(r); }
            }
        }
        count += 1;
    }
    assert_eq!(count, 2);
}

#[test]
fn decoder_sample() {
    let s = "1997,Ford,E350,\n\
            \"1997\", \"Ford\",\"E350\",\"Super, luxurious truck\"\n\
            1997,Ford,E350,\"Go get one now\n\
            they are going fast\"";
    let mut d = Reader::from_string(s);
    let r: Vec<(uint, String, String, String)> = ordie(collect(d.decode()));
    assert_eq!(*r[1].ref0(), 1997);
}

#[test]
fn decoder_byte_strings() {
    let mut d = Reader::from_string("abc,xyz").has_headers(false);
    let r = ordie(d.byte_records().next().unwrap());
    assert_eq!(r, vec![bytes(b"abc"), bytes(b"xyz")]);
}

#[test]
fn decoder_byte_strings_invalid_utf8() {
    let mut d = Reader::from_bytes(b"a\xffbc,xyz").has_headers(false);
    let r = ordie(d.byte_records().next().unwrap());
    assert_eq!(r, vec![bytes(b"a\xffbc"), bytes(b"xyz")]);
}

#[test]
#[should_fail]
fn decoder_invalid_utf8() {
    let mut d = Reader::from_bytes(b"a\xffbc,xyz").has_headers(false);
    ordie(d.records().next().unwrap());
}

#[test]
fn decoder_iter() {
    let mut d = Reader::from_string("andrew,1\nkait,2\ncauchy,3\nplato,4")
                            .has_headers(false);
    let rs: Vec<uint> = d.decode::<(String, uint)>()
                         .map(|r| r.unwrap().val1()).collect();
    assert_eq!(rs, vec!(1u, 2, 3, 4));
}

#[test]
fn decoder_reset() {
    use std::io;

    let data = "1,2\n3,4\n5,6\n";
    let mut buf = io::MemReader::new(data.as_bytes().to_vec());

    {
        let mut d = Reader::from_reader(buf.by_ref()).has_headers(false);
        let vals: Vec<(uint, uint)> = ordie(collect(d.decode()));
        assert_eq!(vals, vec!((1, 2), (3, 4), (5, 6)));
    }

    buf.seek(0, io::SeekSet).unwrap();
    {
        let mut d = Reader::from_reader(buf.by_ref()).has_headers(false);
        let vals: Vec<(uint, uint)> = ordie(collect(d.decode()));
        assert_eq!(vals, vec!((1, 2), (3, 4), (5, 6)));
    }
}

#[test]
fn raw_access() {
    let mut rdr = Reader::from_string("1,2");
    let mut fields = vec![];
    loop {
        let field = match rdr.next_field() {
            None => break,
            Some(result) => result.unwrap(),
        };
        fields.push(field.to_vec());
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
