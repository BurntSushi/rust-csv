use std::default::Default;
use std::str::FromStr;

use rustc_serialize as serialize;

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

    fn len(&self) -> usize {
        self.stack.len()
    }
}

impl Decoded {
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
        {try!(self.pop())}.into_utf8_string().map_err(|bytes| {
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
        self.push(ByteString::from_bytes(s.into_bytes()));
    }

    fn err<'a, T, S>(&self, msg: S) -> Result<T> where S: Into<String> {
        Err(Error::Decode(msg.into()))
    }
}

impl serialize::Decoder for Decoded {
    type Error = Error;

    fn error(&mut self, err: &str) -> Error {
        Error::Decode(err.into())
    }
    fn read_nil(&mut self) -> Result<()> { unimplemented!() }
    fn read_usize(&mut self) -> Result<usize> { self.pop_from_str() }
    fn read_u64(&mut self) -> Result<u64> { self.pop_from_str() }
    fn read_u32(&mut self) -> Result<u32> { self.pop_from_str() }
    fn read_u16(&mut self) -> Result<u16> { self.pop_from_str() }
    fn read_u8(&mut self) -> Result<u8> { self.pop_from_str() }
    fn read_isize(&mut self) -> Result<isize> { self.pop_from_str() }
    fn read_i64(&mut self) -> Result<i64> { self.pop_from_str() }
    fn read_i32(&mut self) -> Result<i32> { self.pop_from_str() }
    fn read_i16(&mut self) -> Result<i16> { self.pop_from_str() }
    fn read_i8(&mut self) -> Result<i8> { self.pop_from_str() }
    fn read_bool(&mut self) -> Result<bool> { self.pop_from_str() }
    fn read_f64(&mut self) -> Result<f64> { self.pop_from_str() }
    fn read_f32(&mut self) -> Result<f32> { self.pop_from_str() }
    fn read_char(&mut self) -> Result<char> {
        let s = try!(self.pop_string());
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != 1 {
            return self.err(format!(
                "Expected single character but got '{}'.", s))
        }
        Ok(chars[0])
    }
    fn read_str(&mut self) -> Result<String> {
        self.pop_string()
    }
    fn read_enum<T, F>(&mut self, _: &str, f: F) -> Result<T>
            where F: FnOnce(&mut Decoded) -> Result<T> {
        f(self)
    }
    fn read_enum_variant<T, F>(&mut self, names: &[&str], mut f: F)
                              -> Result<T>
            where F: FnMut(&mut Decoded, usize) -> Result<T> {
        for i in 0..names.len() {
            let cur = try!(self.pop_string());
            self.push_string(cur.clone());
            match f(self, i) {
                Ok(v) => return Ok(v),
                Err(_) => { self.push_string(cur); }
            }
        }
        self.err(format!(
            "Could not load value into any variant in {:?}", names))
    }
    fn read_enum_variant_arg<T, F>(&mut self, _: usize, f: F) -> Result<T>
            where F: FnOnce(&mut Decoded) -> Result<T> {
        f(self)
    }
    fn read_enum_struct_variant<T, F>(&mut self, names: &[&str], f: F)
                                     -> Result<T>
            where F: FnMut(&mut Decoded, usize) -> Result<T> {
        self.read_enum_variant(names, f)
    }
    fn read_enum_struct_variant_field<T, F>(&mut self, _: &str,
                                            f_idx: usize, f: F)
                                           -> Result<T>
            where F: FnOnce(&mut Decoded) -> Result<T> {
        self.read_enum_variant_arg(f_idx, f)
    }
    fn read_struct<T, F>(&mut self, s_name: &str, len: usize, f: F)
                        -> Result<T>
            where F: FnOnce(&mut Decoded) -> Result<T> {
        if self.len() < len {
            return self.err(
                format!("Struct '{}' has {} fields but current record \
                         has {} fields.", s_name, len, self.len()));
        }
        f(self)
    }
    fn read_struct_field<T, F>(&mut self, _: &str, _: usize, f: F)
                              -> Result<T>
            where F: FnOnce(&mut Decoded) -> Result<T> {
        f(self)
    }
    fn read_tuple<T, F>(&mut self, _: usize, f: F) -> Result<T>
            where F: FnOnce(&mut Decoded) -> Result<T> {
        f(self)
    }
    fn read_tuple_arg<T, F>(&mut self, _: usize, f: F) -> Result<T>
            where F: FnOnce(&mut Decoded) -> Result<T> {
        f(self)
    }
    fn read_tuple_struct<T, F>(&mut self, _: &str, _: usize, _: F)
                              -> Result<T>
            where F: FnOnce(&mut Decoded) -> Result<T> {
        unimplemented!()
    }
    fn read_tuple_struct_arg<T, F>(&mut self, _: usize, _: F)
                                  -> Result<T>
            where F: FnOnce(&mut Decoded) -> Result<T> {
        unimplemented!()
    }
    fn read_option<T, F>(&mut self, mut f: F) -> Result<T>
            where F: FnMut(&mut Decoded, bool) -> Result<T> {
        let s = try!(self.pop_string());
        if s.is_empty() {
            f(self, false)
        } else {
            self.push_string(s);
            match f(self, true) {
                Ok(v) => Ok(v),
                Err(_) => f(self, false),
            }
        }
    }
    fn read_seq<T, F>(&mut self, f: F) -> Result<T>
            where F: FnOnce(&mut Decoded, usize) -> Result<T> {
        let len = self.len();
        f(self, len)
    }
    fn read_seq_elt<T, F>(&mut self, _: usize, f: F) -> Result<T>
            where F: FnOnce(&mut Decoded) -> Result<T> {
        f(self)
    }
    fn read_map<T, F>(&mut self, _: F) -> Result<T>
            where F: FnOnce(&mut Decoded, usize) -> Result<T> {
        unimplemented!()
    }
    fn read_map_elt_key<T, F>(&mut self, _: usize, _: F) -> Result<T>
            where F: FnOnce(&mut Decoded) -> Result<T> {
        unimplemented!()
    }
    fn read_map_elt_val<T, F>(&mut self, _: usize, _: F) -> Result<T>
            where F: FnOnce(&mut Decoded) -> Result<T> {
        unimplemented!()
    }
}
