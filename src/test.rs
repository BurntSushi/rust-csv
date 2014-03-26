use quickcheck::{TestResult, quickcheck};
use super::{StrEncoder, Decoder};

#[deriving(Show, Encodable, Decodable)]
enum Color {
    Red, Green, Blue
}

#[deriving(Show, Encodable, Decodable)]
struct Record {
    color: Color,
    a: f64,
    b: ~str,
    c: ~str,
}

fn ordie<T, E: ::std::fmt::Show>(res: Result<T, E>) -> T {
    match res {
        Ok(t) => t,
        Err(err) => fail!("{}", err),
    }
}

#[test]
fn same_record() {
    fn prop(input: Vec<~str>) -> TestResult {
        if input.len() == 0 {
            return TestResult::discard()
        }

        let mut senc = StrEncoder::new();
        senc.encode(input.as_slice());

        let mut dec = Decoder::from_str(senc.to_str());
        let output: Vec<~str> = dec.decode().unwrap();

        TestResult::from_bool(input == output)
    }
    quickcheck(prop);
}

#[test]
fn same_records() {
    fn prop(to_repeat: Vec<~str>, n: uint) -> TestResult {
        if to_repeat.len() == 0 || n == 0 {
            return TestResult::discard()
        }

        let input = Vec::from_fn(n, |_| to_repeat.clone());
        let mut senc = StrEncoder::new();
        senc.encode_all(input.as_slice());

        let mut dec = Decoder::from_str(senc.to_str());
        let output: Vec<Vec<~str>> = dec.decode_all().unwrap();

        TestResult::from_bool(input == output)
    }
    quickcheck(prop);
}

#[test]
fn encoder_simple() {
    let mut senc = StrEncoder::new();
    senc.encode(("springsteen", 's', 1, 0.14, false));
    assert_eq!("springsteen,s,1,0.14,false\n", senc.to_str());
}

#[test]
fn encoder_simple_crlf() {
    let mut senc = StrEncoder::new();
    senc.encoder.crlf(true);
    senc.encode(("springsteen", 's', 1, 0.14, false));
    assert_eq!("springsteen,s,1,0.14,false\r\n", senc.to_str());
}

#[test]
fn encoder_simple_tabbed() {
    let mut senc = StrEncoder::new();
    senc.encoder.separator('\t');
    senc.encode(("springsteen", 's', 1, 0.14, false));
    assert_eq!("springsteen\ts\t1\t0.14\tfalse\n", senc.to_str());
}

#[test]
fn encoder_same_length_records() {
    let mut senc = StrEncoder::new();
    senc.encoder.enforce_same_length(true);
    senc.encode(vec!('a'));
    match senc.encoder.encode(vec!('a', 'b')) {
        Ok(_) => fail!("Encoder should report an error when records of \
                        varying length are added and records of same \
                        length is enabled."),
        Err(_) => {},
    }
}

#[test]
fn encoder_quoted_quotes() {
    let mut senc = StrEncoder::new();
    senc.encode(vec!("sprin\"g\"steen"));
    assert_eq!("\"sprin\"\"g\"\"steen\"\n", senc.to_str());
}

#[test]
fn encoder_quoted_sep() {
    let mut senc = StrEncoder::new();
    senc.encoder.separator(',');
    senc.encode(vec!("spring,steen"));
    assert_eq!("\"spring,steen\"\n", senc.to_str());
}

#[test]
fn encoder_quoted_newlines() {
    let mut senc = StrEncoder::new();
    senc.encode(vec!("spring\nsteen"));
    assert_eq!("\"spring\nsteen\"\n", senc.to_str());
}

#[test]
fn encoder_zero() {
    let mut senc = StrEncoder::new();
    match senc.encoder.encode::<Vec<int>>(vec!()) {
        Ok(_) => fail!("Encoder should report an error when trying to \
                        encode records of length 0."),
        Err(_) => {},
    }
}

#[test]
fn decoder_simple_nonl() {
    let mut d = Decoder::from_str("springsteen,s,1,0.14,false");
    let r: (~str, char, int, f64, bool) = d.decode().unwrap();
    assert_eq!(r, (~"springsteen", 's', 1, 0.14, false));
}

#[test]
fn decoder_simple() {
    let mut d = Decoder::from_str("springsteen,s,1,0.14,false\n");
    let r: (~str, char, int, f64, bool) = d.decode().unwrap();
    assert_eq!(r, (~"springsteen", 's', 1, 0.14, false));
}

#[test]
fn decoder_simple_crlf() {
    let mut d = Decoder::from_str("springsteen,s,1,0.14,false\r\n");
    let r: (~str, char, int, f64, bool) = d.decode().unwrap();
    assert_eq!(r, (~"springsteen", 's', 1, 0.14, false));
}

#[test]
fn decoder_simple_tabbed() {
    let mut d = Decoder::from_str("springsteen\ts\t1\t0.14\tfalse\r\n");
    d.separator('\t');
    let r: (~str, char, int, f64, bool) = d.decode().unwrap();
    assert_eq!(r, (~"springsteen", 's', 1, 0.14, false));
}

#[test]
fn decoder_same_length_records() {
    let mut d = Decoder::from_str("a\na,b");
    d.enforce_same_length(true);
    match d.decode_all::<Vec<~str>>() {
        Ok(_) => fail!("Decoder should report an error when records of \
                        varying length are decoded and records of same \
                        length if enabled."),
        Err(_) => {},
    }
}

#[test]
fn decoder_headers() {
    let mut d = Decoder::from_str("a,b,c\n1,2,3");
    d.has_headers(true);
    assert_eq!(d.headers().unwrap(), vec!(~"a", ~"b", ~"c"));

    let r: (uint, uint, uint) = d.decode().unwrap();
    assert_eq!(r, (1, 2, 3));
}

#[test]
#[should_fail]
fn decoder_bad_header_access() {
    let mut d = Decoder::from_str("");
    d.has_headers(false);
    let _ = d.headers();
}

#[deriving(Decodable, Encodable, Show, Eq, TotalEq)]
enum Val {
    Unsigned(uint),
    Signed(int),
    Bool(bool),
}

#[test]
fn decoder_enum() {
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
    let r = (Bool(false), Signed(-5), Unsigned(5));
    let mut senc = StrEncoder::new();
    senc.encode(r);
    assert_eq!("false,-5,5\n", senc.to_str());
}

#[test]
fn encoder_option() {
    let r: (Option<bool>, uint) = (None, 1);
    let mut senc = StrEncoder::new();
    senc.encode(r);
    assert_eq!(",1\n", senc.to_str());
}
