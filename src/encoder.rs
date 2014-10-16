use std::fmt;

use serialize;

use {ByteString, CsvResult, Error, ErrEncode, IntoVector};

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

    fn push_bytes<S: IntoVector<u8>>(&mut self, s: S) -> CsvResult<()> {
        self.record.push(ByteString::from_bytes(s));
        Ok(())
    }

    fn push_string<S: StrAllocating>(&mut self, s: S) -> CsvResult<()> {
        self.push_bytes(s.into_string().into_bytes())
    }

    fn push_to_string<T: fmt::Show>(&mut self, t: T) -> CsvResult<()> {
        self.push_string(t.to_string())
    }
}

impl serialize::Encoder<Error> for Encoded {
    fn emit_nil(&mut self) -> CsvResult<()> { unimplemented!() }
    fn emit_uint(&mut self, v: uint) -> CsvResult<()> { self.push_to_string(v) }
    fn emit_u64(&mut self, v: u64) -> CsvResult<()> { self.push_to_string(v) }
    fn emit_u32(&mut self, v: u32) -> CsvResult<()> { self.push_to_string(v) }
    fn emit_u16(&mut self, v: u16) -> CsvResult<()> { self.push_to_string(v) }
    fn emit_u8(&mut self, v: u8) -> CsvResult<()> { self.push_to_string(v) }
    fn emit_int(&mut self, v: int) -> CsvResult<()> { self.push_to_string(v) }
    fn emit_i64(&mut self, v: i64) -> CsvResult<()> { self.push_to_string(v) }
    fn emit_i32(&mut self, v: i32) -> CsvResult<()> { self.push_to_string(v) }
    fn emit_i16(&mut self, v: i16) -> CsvResult<()> { self.push_to_string(v) }
    fn emit_i8(&mut self, v: i8) -> CsvResult<()> { self.push_to_string(v) }
    fn emit_bool(&mut self, v: bool) -> CsvResult<()> { self.push_to_string(v) }
    fn emit_f64(&mut self, v: f64) -> CsvResult<()> {
        self.push_string(::std::f64::to_str_digits(v, 10))
    }
    fn emit_f32(&mut self, v: f32) -> CsvResult<()> {
        self.push_string(::std::f32::to_str_digits(v, 10))
    }
    fn emit_char(&mut self, v: char) -> CsvResult<()> {
        let mut bytes = [0u8, ..4];
        let n = v.encode_utf8(bytes.as_mut_slice()).unwrap_or(0);
        self.push_bytes(bytes.slice_to(n))
    }
    fn emit_str(&mut self, v: &str) -> CsvResult<()> {
        self.push_string(v)
    }
    fn emit_enum(&mut self, _: &str, f: |&mut Encoded| -> CsvResult<()>)
                -> CsvResult<()> {
        f(self)
    }
    fn emit_enum_variant(&mut self, v_name: &str, _: uint, len: uint,
                         f: |&mut Encoded| -> CsvResult<()>)
                        -> CsvResult<()> {
        match len {
            0 => self.push_string(v_name),
            1 => f(self),
            _ => Err(ErrEncode("Cannot encode enum variants \
                                with more than one argument.".to_string())),
        }
    }
    fn emit_enum_variant_arg(&mut self, _: uint,
                             f: |&mut Encoded| -> CsvResult<()>)
                            -> CsvResult<()> {
        f(self)
    }
    fn emit_enum_struct_variant(&mut self, v_name: &str, v_id: uint, len: uint,
                                f: |&mut Encoded| -> CsvResult<()>)
                               -> CsvResult<()> {
        self.emit_enum_variant(v_name, v_id, len, f)
    }
    fn emit_enum_struct_variant_field(&mut self, _: &str, _: uint,
                                      _: |&mut Encoded| -> CsvResult<()>)
                                     -> CsvResult<()> {
        Err(ErrEncode("Cannot encode enum \
                       variants with arguments.".to_string()))
    }
    fn emit_struct(&mut self, _: &str, len: uint,
                   f: |&mut Encoded| -> CsvResult<()>) -> CsvResult<()> {
        self.emit_seq(len, f)
    }
    fn emit_struct_field(&mut self, _: &str, f_idx: uint,
                         f: |&mut Encoded| -> CsvResult<()>)
                        -> CsvResult<()> {
        self.emit_seq_elt(f_idx, f)
    }
    fn emit_tuple(&mut self, len: uint,
                  f: |&mut Encoded| -> CsvResult<()>) -> CsvResult<()> {
        self.emit_seq(len, f)
    }
    fn emit_tuple_arg(&mut self, idx: uint,
                      f: |&mut Encoded| -> CsvResult<()>)
                     -> CsvResult<()> {
        self.emit_seq_elt(idx, f)
    }
    fn emit_tuple_struct(&mut self, _: &str, _: uint,
                         _: |&mut Encoded| -> CsvResult<()>)
                        -> CsvResult<()> {
        unimplemented!()
    }
    fn emit_tuple_struct_arg(&mut self, _: uint,
                             _: |&mut Encoded| -> CsvResult<()>)
                            -> CsvResult<()> {
        unimplemented!()
    }
    fn emit_option(&mut self, f: |&mut Encoded| -> CsvResult<()>)
                  -> CsvResult<()> {
        f(self)
    }
    fn emit_option_none(&mut self) -> CsvResult<()> {
        self.push_bytes::<&[u8]>([])
    }
    fn emit_option_some(&mut self, f: |&mut Encoded| -> CsvResult<()>)
                       -> CsvResult<()> {
        f(self)
    }
    fn emit_seq(&mut self, _: uint,
                f: |this: &mut Encoded| -> CsvResult<()>)
               -> CsvResult<()> {
        f(self)
    }
    fn emit_seq_elt(&mut self, _: uint,
                    f: |this: &mut Encoded| -> CsvResult<()>)
                   -> CsvResult<()> {
        f(self)
    }
    fn emit_map(&mut self, _: uint, _: |&mut Encoded| -> CsvResult<()>)
               -> CsvResult<()> {
        unimplemented!()
    }
    fn emit_map_elt_key(&mut self, _: uint,
                        _: |&mut Encoded| -> CsvResult<()>)
                       -> CsvResult<()> {
        unimplemented!()
    }
    fn emit_map_elt_val(&mut self, _: uint,
                        _: |&mut Encoded| -> CsvResult<()>)
                       -> CsvResult<()> {
        unimplemented!()
    }
}
