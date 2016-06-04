use std::str::FromStr;

use {ByteString, Result, Error};

/// A record to be decoded.
///
/// This is a "wrapper" type that allows the `Decoder` machinery from the
/// `serialize` crate to decode a *single* CSV record into your custom types.
///
/// Generally, you should not need to use this type directly. Instead, you
/// should prefer the `decode` or `decode_all` methods defined on `CsvReader`.
pub struct Decoded {
    stack: Vec<ByteString>,
    popped: usize,
}

impl Decoded {
    /// Creates a new decodable record from a record of byte strings.
    pub fn new(mut bytes: Vec<ByteString>) -> Decoded {
        bytes.reverse();
        Decoded { stack: bytes, popped: 0 }
    }
}

pub trait DecodedHelper {
    fn len(&self) -> usize;
    fn pop(&mut self) -> Result<ByteString>;
    fn pop_string(&mut self) -> Result<String>;
    fn pop_from_str<T: FromStr + Default>(&mut self) -> Result<T>;
    fn push(&mut self, s: ByteString);
    fn push_string(&mut self, s: String);
    fn err<'a, T, S>(&self, msg: S) -> Result<T> where S: Into<String>;
}

impl DecodedHelper for Decoded {
    fn len(&self) -> usize {
        self.stack.len()
    }

    fn pop(&mut self) -> Result<ByteString> {
        self.popped += 1;
        match self.stack.pop() {
            None => self.err(format!(
                "Expected a record with length at least {}, \
                 but got a record with length {}.",
                self.popped, self.popped - 1)),
            Some(bytes) => Ok(bytes),
        }
    }

    fn pop_string(&mut self) -> Result<String> {
        String::from_utf8(try!(self.pop())).map_err(|bytes| {
            Error::Decode(
                format!("Could not convert bytes '{:?}' to UTF-8.", bytes))
        })
    }

    fn pop_from_str<T: FromStr + Default>(&mut self) -> Result<T> {
        let s = try!(self.pop_string());
        let s = s.trim();
        match FromStr::from_str(s) {
            Ok(t) => Ok(t),
            Err(_) => self.err(format!("Failed converting '{}' from str.", s)),
        }
    }

    fn push(&mut self, s: ByteString) {
        self.stack.push(s);
    }

    fn push_string(&mut self, s: String) {
        self.push(s.into_bytes());
    }

    fn err<'a, T, S>(&self, msg: S) -> Result<T> where S: Into<String> {
        Err(Error::Decode(msg.into()))
    }
}


/// A record to be encoded.
///
/// This is a "wrapper" type that allows the `Encoder` machinery from the
/// `serialize` crate to encode a *single* CSV record from your custom types.
///
/// Generally, you should not need to use this type directly. Instead, you
/// should prefer the `encode` or `encode_all` methods defined on `CsvWriter`.
pub struct Encoded {
    record: Vec<ByteString>,
}

impl Encoded {
    /// Creates a new encodable record. The value returned can be passed to
    /// `Encodable::encode`.
    pub fn new() -> Encoded { Encoded { record: vec![] } }

    /// Once a record has been encoded into this value, `unwrap` can be used
    /// to access the raw CSV record.
    pub fn unwrap(self) -> Vec<ByteString> { self.record }
}

pub trait EncodedHelper {
    fn push_bytes<'a, S>(&mut self, s: S) -> Result<()> where S: Into<Vec<u8>>;
    fn push_string<'a, S>(&mut self, s: S) -> Result<()> where S: Into<String>;
    fn push_to_string<T: ToString>(&mut self, t: T) -> Result<()>;
}

impl EncodedHelper for Encoded {
    fn push_bytes<'a, S>(&mut self, s: S) -> Result<()>
            where S: Into<Vec<u8>> {
        self.record.push(s.into());
        Ok(())
    }

    fn push_string<'a, S>(&mut self, s: S) -> Result<()>
            where S: Into<String> {
        self.push_bytes(s.into().into_bytes())
    }

    fn push_to_string<T: ToString>(&mut self, t: T) -> Result<()> {
        self.push_string(t.to_string())
    }
}

pub fn float_to_string(v: f64) -> String {
    let s: String = format!("{:.10}", v).trim_right_matches('0').into();
    if s.ends_with('.') { s + "0" } else { s }
}
