use quickcheck::{TestResult, quickcheck};
use super::{Encoder, Decoder};

fn ordie<T, E: ::std::fmt::Show>(res: Result<T, E>) -> T {
    match res {
        Ok(t) => t,
        Err(err) => fail!("{}", err),
    }
}

#[test]
fn same_record() {
    fn prop(input: Vec<String>) -> TestResult {
        if input.len() == 0 {
            return TestResult::discard()
        }
        if input.iter().any(|s| s.len() == 0) {
            return TestResult::discard()
        }

        let mut senc = Encoder::str_encoder();
        ordie(senc.encode(input.as_slice()));

        let mut dec = Decoder::from_str(senc.to_str());
        let output: Vec<String> = ordie(dec.decode());

        TestResult::from_bool(input == output)
    }
    quickcheck(prop);
}

#[test]
fn encoder_simple() {
    let mut senc = Encoder::str_encoder();
    ordie(senc.encode(("springsteen", 's', 1i, 0.14f64, false)));
    assert_eq!("springsteen,s,1,0.14,false\n", senc.to_str());
}

#[test]
fn encoder_simple_crlf() {
    let mut senc = Encoder::str_encoder();
    senc.crlf(true);
    ordie(senc.encode(("springsteen", 's', 1i, 0.14f64, false)));
    assert_eq!("springsteen,s,1,0.14,false\r\n", senc.to_str());
}

#[test]
fn encoder_simple_tabbed() {
    let mut senc = Encoder::str_encoder();
    senc.separator('\t');
    ordie(senc.encode(("springsteen", 's', 1i, 0.14f64, false)));
    assert_eq!("springsteen\ts\t1\t0.14\tfalse\n", senc.to_str());
}

#[test]
fn encoder_same_length_records() {
    let mut senc = Encoder::str_encoder();
    senc.enforce_same_length(true);
    ordie(senc.encode(vec!('a')));
    match senc.encode(vec!('a', 'b')) {
        Ok(_) => fail!("Encoder should report an error when records of \
                        varying length are added and records of same \
                        length is enabled."),
        Err(_) => {}
    }
}

#[test]
fn encoder_quoted_quotes() {
    let mut senc = Encoder::str_encoder();
    ordie(senc.encode(vec!("sprin\"g\"steen")));
    assert_eq!("\"sprin\"\"g\"\"steen\"\n", senc.to_str());
}

#[test]
fn encoder_quoted_sep() {
    let mut senc = Encoder::str_encoder();
    senc.separator(',');
    ordie(senc.encode(vec!("spring,steen")));
    assert_eq!("\"spring,steen\"\n", senc.to_str());
}

#[test]
fn encoder_quoted_newlines() {
    let mut senc = Encoder::str_encoder();
    ordie(senc.encode(vec!("spring\nsteen")));
    assert_eq!("\"spring\nsteen\"\n", senc.to_str());
}

#[test]
fn encoder_zero() {
    let mut senc = Encoder::str_encoder();
    match senc.encode::<Vec<int>>(vec!()) {
        Ok(_) => fail!("Encoder should report an error when trying to \
                        encode records of length 0."),
        Err(_) => {}
    }
}

#[test]
fn decoder_simple_nonl() {
    let mut d = Decoder::from_str("springsteen,s,1,0.14,false");
    let r: (String, char, int, f64, bool) = ordie(d.decode());
    assert_eq!(r, ("springsteen".to_string(), 's', 1, 0.14, false));
}

#[test]
fn decoder_simple_nonl_comma() {
    let mut d = Decoder::from_str("springsteen,s,");
    let r: (String, char, Option<int>) = ordie(d.decode());
    assert_eq!(r, ("springsteen".to_string(), 's', None));
}

#[test]
fn decoder_simple() {
    let mut d = Decoder::from_str("springsteen,s,1,0.14,false\n");
    let r: (String, char, int, f64, bool) = ordie(d.decode());
    assert_eq!(r, ("springsteen".to_string(), 's', 1, 0.14, false));
}

#[test]
fn decoder_simple_crlf() {
    let mut d = Decoder::from_str("springsteen,s,1,0.14,false\r\n");
    let r: (String, char, int, f64, bool) = ordie(d.decode());
    assert_eq!(r, ("springsteen".to_string(), 's', 1, 0.14, false));
}

#[test]
fn decoder_simple_tabbed() {
    let mut d = Decoder::from_str("springsteen\ts\t1\t0.14\tfalse\r\n");
    d.separator('\t');
    let r: (String, char, int, f64, bool) = ordie(d.decode());
    assert_eq!(r, ("springsteen".to_string(), 's', 1, 0.14, false));
}

#[test]
fn decoder_same_length_records() {
    let mut d = Decoder::from_str("a\na,b");
    d.enforce_same_length(true);
    match d.decode_all::<Vec<String>>() {
        Ok(_) => fail!("Decoder should report an error when records of \
                        varying length are decoded and records of same \
                        length if enabled."),
        Err(_) => {}
    }
}

#[test]
fn decoder_headers() {
    let mut d = Decoder::from_str("a,b,c\n1,2,3");
    d.has_headers(true);
    assert_eq!(ordie(d.headers()),
               vec!("a".to_string(), "b".to_string(), "c".to_string()));

    let r: (uint, uint, uint) = ordie(d.decode());
    assert_eq!(r, (1, 2, 3));
}

#[test]
fn decoder_empty_lines() {
    let mut d = Decoder::from_str("1,2\n\n3,4\n\n\n\n5,6\n\n");
    let vals: Vec<(uint, uint)> = ordie(d.decode_all());
    assert_eq!(vals, vec!((1, 2), (3, 4), (5, 6)));
}

#[test]
fn decoder_empty_lines_crlf() {
    let mut d = Decoder::from_str("1,2\r\n\r\n3,4\r\n\r\n\r\n\r\n5,6\r\n\r\n");
    let vals: Vec<(uint, uint)> = ordie(d.decode_all());
    assert_eq!(vals, vec!((1, 2), (3, 4), (5, 6)));
}

#[test]
fn decoder_empties_headers() {
    let mut d = Decoder::from_str("a,b,c\n\n\n\n");
    d.has_headers(true);
    assert_eq!(ordie(d.headers()),
               vec!("a".to_string(), "b".to_string(), "c".to_string()));
    assert_eq!(d.iter().next(), None);
}

#[test]
fn decoder_all_empties() {
    let mut d = Decoder::from_str("\n\n\n\n");
    assert_eq!(d.iter().next(), None);
}

#[test]
fn decoder_all_empties_crlf() {
    let mut d = Decoder::from_str("\r\n\r\n\r\n\r\n");
    assert_eq!(d.iter().next(), None);
}

#[test]
fn decoder_empty_strings() {
    let mut d = Decoder::from_str("\"\"");
    let r: (String,) = ordie(d.decode());
    assert_eq!(r, ("".to_string(),));
}

#[test]
fn decoder_quotes() {
    let mut d = Decoder::from_str("\" a \",   \"1\"   ,\"1\",  1  ");
    let r: (String, String, uint, uint) = ordie(d.decode());
    assert_eq!(r, (" a ".to_string(), "1".to_string(), 1, 1));
}

#[test]
#[should_fail]
fn decoder_bad_header_access() {
    let mut d = Decoder::from_str("");
    d.has_headers(false);
    let _ = d.headers();
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
    let mut d = Decoder::from_str("ReD");
    let r: (Color,) = ordie(d.decode());
    assert_eq!(r, (Red,));
}

#[test]
fn decoder_enum_arg() {
    let mut d = Decoder::from_str("false,-5,5");
    let r: (Val, Val, Val) = ordie(d.decode());
    assert_eq!(r, (Bool(false), Signed(-5), Unsigned(5)));
}

#[test]
fn decoder_option() {
    let mut d = Decoder::from_str(",1");
    let r: (Option<bool>, uint) = ordie(d.decode());
    assert_eq!(r, (None, 1));
}

#[test]
fn encoder_enum() {
    let r = (Red,);
    let mut senc = Encoder::str_encoder();
    ordie(senc.encode(r));
    assert_eq!("Red\n", senc.to_str());
}

#[test]
fn encoder_enum_arg() {
    let r = (Bool(false), Signed(-5), Unsigned(5));
    let mut senc = Encoder::str_encoder();
    ordie(senc.encode(r));
    assert_eq!("false,-5,5\n", senc.to_str());
}

#[test]
fn encoder_option() {
    let r: (Option<bool>, uint) = (None, 1);
    let mut senc = Encoder::str_encoder();
    ordie(senc.encode(r));
    assert_eq!(",1\n", senc.to_str());
}

#[test]
fn decoder_sample() {
    let s = "1997,Ford,E350,\n\
            \"1997\", \"Ford\", \"E350\", \"Super, luxurious truck\"\n\
            1997,Ford,E350, \"Go get one now\n\
            they are going fast\"";
    let mut d = Decoder::from_str(s);
    let r: Vec<(uint, String, String, String)> = ordie(d.decode_all());
    assert_eq!(*r.get(1).ref0(), 1997);
}

#[test]
fn decoder_iter() {
    let mut d = Decoder::from_str("andrew,1\nkait,2\ncauchy,3\nplato,4");
    let rs: Vec<uint> = d.decode_iter::<(String, uint)>()
                         .map(|(_, num)| num).collect();
    assert_eq!(rs, vec!(1u, 2, 3, 4));
}
