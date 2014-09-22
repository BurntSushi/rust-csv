use std::default::Default;
use std::from_str::FromStr;

use serialize;

use {ByteString, CsvResult, Error, ErrDecode};

/// A record to be decoded.
///
/// This is a "wrapper" type that allows the `Decoder` machinery from the
/// `serialize` crate to decode a *single* CSV record into your custom types.
///
/// Generally, you should not need to use this type directly. Instead, you
/// should prefer the `decode` or `decode_all` methods defined on `CsvReader`.
pub struct Decoded {
    stack: Vec<ByteString>,
    popped: uint,
}

impl Decoded {
    /// Creates a new decodable record from a record of byte strings.
    pub fn new(mut bytes: Vec<ByteString>) -> Decoded {
        bytes.reverse();
        Decoded { stack: bytes, popped: 0 }
    }
}

impl Collection for Decoded {
    fn len(&self) -> uint { self.stack.len() }
}

impl Decoded {
    fn pop(&mut self) -> CsvResult<ByteString> {
        self.popped += 1;
        match self.stack.pop() {
            None => self.err(format!(
                "Expected a record with length at least {}, \
                 but got a record with length {}.",
                self.popped, self.popped - 1)),
            Some(bytes) => Ok(bytes),
        }
    }

    fn pop_string(&mut self) -> CsvResult<String> {
        {try!(self.pop())}.as_utf8_string().map_err(|bytes| {
            ErrDecode(format!("Could not convert bytes '{}' to UTF-8.", bytes))
        })
    }

    fn pop_from_str<T: FromStr + Default>(&mut self) -> CsvResult<T> {
        let s = try!(self.pop_string());
        let s = s.as_slice().trim();
        match FromStr::from_str(s) {
            Some(t) => Ok(t),
            None => self.err(format!("Failed converting '{}' from str.", s)),
        }
    }

    fn push(&mut self, s: ByteString) {
        self.stack.push(s);
    }

    fn push_string(&mut self, s: String) {
        self.push(ByteString::from_bytes(s.into_bytes()));
    }

    fn err<T, S: StrAllocating>(&self, msg: S) -> CsvResult<T> {
        Err(ErrDecode(msg.into_string()))
    }
}

impl serialize::Decoder<Error> for Decoded {
    fn error(&mut self, err: &str) -> Error {
        ErrDecode(err.into_string())
    }
    fn read_nil(&mut self) -> CsvResult<()> { unimplemented!() }
    fn read_uint(&mut self) -> CsvResult<uint> { self.pop_from_str() }
    fn read_u64(&mut self) -> CsvResult<u64> { self.pop_from_str() }
    fn read_u32(&mut self) -> CsvResult<u32> { self.pop_from_str() }
    fn read_u16(&mut self) -> CsvResult<u16> { self.pop_from_str() }
    fn read_u8(&mut self) -> CsvResult<u8> { self.pop_from_str() }
    fn read_int(&mut self) -> CsvResult<int> { self.pop_from_str() }
    fn read_i64(&mut self) -> CsvResult<i64> { self.pop_from_str() }
    fn read_i32(&mut self) -> CsvResult<i32> { self.pop_from_str() }
    fn read_i16(&mut self) -> CsvResult<i16> { self.pop_from_str() }
    fn read_i8(&mut self) -> CsvResult<i8> { self.pop_from_str() }
    fn read_bool(&mut self) -> CsvResult<bool> { self.pop_from_str() }
    fn read_f64(&mut self) -> CsvResult<f64> { self.pop_from_str() }
    fn read_f32(&mut self) -> CsvResult<f32> { self.pop_from_str() }
    fn read_char(&mut self) -> CsvResult<char> {
        let s = try!(self.pop_string());
        let chars: Vec<char> = s.as_slice().chars().collect();
        if chars.len() != 1 {
            return self.err(format!(
                "Expected single character but got '{}'.", s))
        }
        Ok(chars[0])
    }
    fn read_str(&mut self) -> CsvResult<String> {
        self.pop_string()
    }
    fn read_enum<T>(&mut self, _: &str,
                    f: |&mut Decoded| -> CsvResult<T>)
                   -> CsvResult<T> {
        f(self)
    }
    fn read_enum_variant<T>(&mut self, names: &[&str],
                            f: |&mut Decoded, uint| -> CsvResult<T>)
                           -> CsvResult<T> {
        let variant = to_lower(try!(self.pop_string()).as_slice());
        match names.iter().position(|&name| to_lower(name) == variant) {
            Some(idx) => return f(self, idx),
            None => {}
        }

        // At this point, we couldn't find a verbatim Enum variant, so let's
        // assume we're trying to load enum variants of one argument.
        // We don't know which one to pick, so we try each of them until we
        // get a hit.
        //
        // If we fail, it's tough to know what error to report. Probably the
        // right way to do this is to maintain a stack of errors. Ug.
        self.push_string(variant); // push what we popped earlier
        for i in range(0, names.len()) {
            // Copy the top of the stack now. We'll push it back on if
            // decoding into this variant fails.
            let cur = try!(self.pop_string());
            let copy = cur.clone();
            self.push_string(cur);

            match f(self, i) {
                Ok(v) => return Ok(v), // loaded a value successfully; bail!
                Err(_) => {
                    // Put what we popped back on the stack so we can retry.
                    self.push_string(copy);
                }
            }
        }
        return self.err(format!(
            "Could not load value into any variant in {}", names))
    }
    fn read_enum_variant_arg<T>(&mut self, _: uint,
                                f: |&mut Decoded| -> CsvResult<T>)
                               -> CsvResult<T> {
        f(self)
    }
    fn read_enum_struct_variant<T>(&mut self, names: &[&str],
                                   f: |&mut Decoded, uint| -> CsvResult<T>)
                                  -> CsvResult<T> {
        self.read_enum_variant(names, f)
    }
    fn read_enum_struct_variant_field<T>(&mut self, _: &str, f_idx: uint,
                                         f: |&mut Decoded| -> CsvResult<T>)
                                        -> CsvResult<T> {
        self.read_enum_variant_arg(f_idx, f)
    }
    fn read_struct<T>(&mut self, s_name: &str, len: uint,
                      f: |&mut Decoded| -> CsvResult<T>) -> CsvResult<T> {
        if self.len() < len {
            return self.err(
                format!("Struct '{}' has {} fields but current record \
                         has {} fields.", s_name, len, self.len()));
        }
        f(self)
    }
    fn read_struct_field<T>(&mut self, _: &str, _: uint,
                            f: |&mut Decoded| -> CsvResult<T>)
                           -> CsvResult<T> {
        f(self)
    }
    fn read_tuple<T>(&mut self, f: |&mut Decoded, uint| -> CsvResult<T>)
                    -> CsvResult<T> {
        let len = self.len();
        f(self, len)
    }
    fn read_tuple_arg<T>(&mut self, _: uint,
                         f: |&mut Decoded| -> CsvResult<T>)
                        -> CsvResult<T> {
        f(self)
    }
    fn read_tuple_struct<T>(&mut self, _: &str,
                            _: |&mut Decoded, uint| -> CsvResult<T>)
                           -> CsvResult<T> {
        unimplemented!()
    }
    fn read_tuple_struct_arg<T>(&mut self, _: uint,
                                _: |&mut Decoded| -> CsvResult<T>)
                               -> CsvResult<T> {
        unimplemented!()
    }
    fn read_option<T>(&mut self,
                      f: |&mut Decoded, bool| -> CsvResult<T>)
                     -> CsvResult<T> {
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
    fn read_seq<T>(&mut self, f: |&mut Decoded, uint| -> CsvResult<T>)
                  -> CsvResult<T> {
        let len = self.len();
        f(self, len)
    }
    fn read_seq_elt<T>(&mut self, _: uint,
                       f: |&mut Decoded| -> CsvResult<T>)
                      -> CsvResult<T> {
        f(self)
    }
    fn read_map<T>(&mut self, _: |&mut Decoded, uint| -> CsvResult<T>)
                  -> CsvResult<T> {
        unimplemented!()
    }
    fn read_map_elt_key<T>(&mut self, _: uint,
                           _: |&mut Decoded| -> CsvResult<T>)
                          -> CsvResult<T> {
        unimplemented!()
    }
    fn read_map_elt_val<T>(&mut self, _: uint,
                           _: |&mut Decoded| -> CsvResult<T>)
                          -> CsvResult<T> {
        unimplemented!()
    }
}

fn to_lower(s: &str) -> String {
    s.chars().map(|c| c.to_lowercase()).collect()
}
