use std::fmt;
use std::io::{MemWriter, BufferedWriter, File, IoResult};
use std::str;

use serialize;
use serialize::Encodable;

use ByteString;
use {Error, ErrEncode};

/// An encoder can encode Rust values into CSV records or documents.
pub struct Encoder<W> {
    buf: BufferedWriter<W>,
    sep: u8,
    same_len: bool,
    first_len: uint,
    use_crlf: bool,
}

impl Encoder<MemWriter> {
    /// Creates a new CSV in memory encoder. At any time, `to_string` or
    /// `to_bytes` can be called to retrieve the cumulative CSV data.
    pub fn mem_encoder() -> Encoder<MemWriter> {
        Encoder::to_writer(MemWriter::new())
    }

    /// Returns the encoded CSV data as a string.
    pub fn to_string<'r>(&'r mut self) -> &'r str {
        match self.buf.flush() {
            // shouldn't fail with MemWriter
            Err(err) => fail!("Error flushing to MemWriter: {}", err),
            Ok(()) => str::from_utf8(self.buf.get_ref().get_ref()).unwrap(),
        }
    }

    /// Returns the encoded CSV data as raw bytes.
    pub fn to_bytes<'r>(&'r mut self) -> &'r [u8] {
        match self.buf.flush() {
            // shouldn't fail with MemWriter
            Err(err) => fail!("Error flushing to MemWriter: {}", err),
            Ok(()) => self.buf.get_ref().get_ref(),
        }
    }
}

impl Encoder<IoResult<File>> {
    /// Creates an encoder that writes the file given. If the file doesn't
    /// exist, then it is created. If it already exists, then it is truncated
    /// before writing.
    pub fn to_file(path: &Path) -> Encoder<IoResult<File>> {
        Encoder::to_writer(File::create(path))
    }
}

impl<W: Writer> Encoder<W> {
    /// Creates an encoder that encodes CSV data with the `Writer` given.
    ///
    /// The writer given in wrapped in a `BufferedWriter` for you.
    pub fn to_writer(w: W) -> Encoder<W> {
        Encoder::from_buffer(BufferedWriter::new(w))
    }

    /// This is just like `to_writer`, except you can specify the capacity
    /// used in the underlying buffer.
    pub fn to_writer_capacity(w: W, cap: uint) -> Encoder<W> {
        Encoder::from_buffer(BufferedWriter::with_capacity(cap, w))
    }

    fn from_buffer(buf: BufferedWriter<W>) -> Encoder<W> {
        Encoder {
            buf: buf,
            sep: b',',
            same_len: true,
            first_len: 0,
            use_crlf: false,
        }
    }

    /// Sets the separator character that delimits values in a record.
    ///
    /// This may be changed at any time.
    pub fn separator(mut self, c: u8) -> Encoder<W> {
        self.sep = c;
        self
    }

    /// When `yes` is `true`, all records written must have the same length.
    /// If a record is written that has a different length than other records
    /// already written, the encoding will fail.
    ///
    /// This may be changed at any time.
    pub fn enforce_same_length(mut self, yes: bool) -> Encoder<W> {
        self.same_len = yes;
        self
    }

    /// When `yes` is `true`, CRLF (`\r\n`) line endings will be used.
    ///
    /// This may be changed at any time.
    pub fn crlf(mut self, yes: bool) -> Encoder<W> {
        self.use_crlf = yes;
        self
    }

    fn err<S: StrAllocating>(&self, msg: S) -> Result<(), Error> {
        Err(ErrEncode(msg.into_string()))
    }

    fn write_bytes(&mut self, s: &[u8]) -> Result<(), Error> {
        self.buf.write(s).map_err(Error::io)
    }

    fn write_string<S: StrAllocating>(&mut self, s: S) -> Result<(), Error> {
        self.write_bytes(s.into_string().into_bytes().as_slice())
    }

    fn write_to_string<T: fmt::Show>(&mut self, t: T) -> Result<(), Error> {
        self.write_string(t.to_string())
    }

    fn write_user_string(&mut self, s: &[u8]) -> Result<(), Error> {
        let sep = self.sep;
        let quotable = |&c: &u8| c == sep || c == b'\n' || c == b'"';
        if s.len() == 0 || s.iter().any(quotable) {
            self.write_bytes(quote(s).as_slice())
        } else {
            self.write_bytes(s)
        }
    }

    fn write_lineterm(&mut self) -> Result<(), Error> {
        if self.use_crlf {
            self.write_bytes(b"\r\n")
        } else {
            self.write_bytes(b"\n")
        }
    }

    fn set_first_len(&mut self, cur_len: uint) -> Result<(), Error> {
        if cur_len == 0 {
            return self.err("Records must have length bigger than 0.")
        }
        if self.same_len {
            if self.first_len == 0 {
                self.first_len = cur_len
            } else if self.first_len != cur_len {
                return self.err(format!(
                    "Record has length {} but other records have length {}",
                    cur_len, self.first_len))
            }
        }
        Ok(())
    }
}

/// The following methods provide type-aware encoding of CSV records.
impl<W: Writer> Encoder<W> {
    /// Encodes a record as CSV data. Only values with types that correspond
    /// to records can be given here (i.e., structs, tuples or vectors).
    pub fn encode<E: Encodable<Encoder<W>, Error>>
                 (&mut self, e: E) -> Result<(), Error> {
        e.encode(self)
    }

    /// Calls `encode` on each element in the iterator given.
    pub fn encode_all<E: Encodable<Encoder<W>, Error>, I: Iterator<E>>
                     (&mut self, mut es: I) -> Result<(), Error> {
        for e in es {
            try!(self.encode(e))
        }
        Ok(())
    }
}

/// The following methods provide direct access to writing CSV records as
/// UTF-8 encoding strings or as raw byte strings.
impl<W: Writer> Encoder<W> {
    pub fn record<'a, S: StrSlice<'a>, I: Iterator<S>>
                 (&mut self, r: I) -> Result<(), Error> {
        self.record_bytes(r.map(|r| r.as_bytes()))
    }

    pub fn record_bytes<S: Slice<u8>, I: Iterator<S>>
                       (&mut self, r: I) -> Result<(), Error> {
        let mut count = 0;
        let sep = self.sep;
        for (i, field) in r.enumerate() {
            count += 1;
            if i > 0 {
                try!(self.write_bytes([sep]));
            }
            try!(self.write_bytes(field.as_slice()));
        }
        try!(self.write_lineterm());
        self.set_first_len(count)
    }

    /// Flushes the underlying buffer.
    pub fn flush(&mut self) -> Result<(), Error> {
        self.buf.flush().map_err(Error::io)
    }
}

impl<W: Writer> serialize::Encoder<Error> for Encoder<W> {
    fn emit_nil(&mut self) -> Result<(), Error> { unimplemented!() }
    fn emit_uint(&mut self, v: uint) -> Result<(), Error> {
        self.write_to_string(v)
    }
    fn emit_u64(&mut self, v: u64) -> Result<(), Error> { self.write_to_string(v) }
    fn emit_u32(&mut self, v: u32) -> Result<(), Error> { self.write_to_string(v) }
    fn emit_u16(&mut self, v: u16) -> Result<(), Error> { self.write_to_string(v) }
    fn emit_u8(&mut self, v: u8) -> Result<(), Error> { self.write_to_string(v) }
    fn emit_int(&mut self, v: int) -> Result<(), Error> { self.write_to_string(v) }
    fn emit_i64(&mut self, v: i64) -> Result<(), Error> { self.write_to_string(v) }
    fn emit_i32(&mut self, v: i32) -> Result<(), Error> { self.write_to_string(v) }
    fn emit_i16(&mut self, v: i16) -> Result<(), Error> { self.write_to_string(v) }
    fn emit_i8(&mut self, v: i8) -> Result<(), Error> { self.write_to_string(v) }
    fn emit_bool(&mut self, v: bool) -> Result<(), Error> { self.write_to_string(v) }
    fn emit_f64(&mut self, v: f64) -> Result<(), Error> {
        self.write_string(::std::f64::to_str_digits(v, 10))
    }
    fn emit_f32(&mut self, v: f32) -> Result<(), Error> {
        self.write_string(::std::f32::to_str_digits(v, 10))
    }
    fn emit_char(&mut self, v: char) -> Result<(), Error> {
        let mut bytes = [0u8, ..4];
        let n = v.encode_utf8(bytes.as_mut_slice()).unwrap_or(0);
        self.write_user_string(bytes.slice_to(n))
    }
    fn emit_str(&mut self, v: &str) -> Result<(), Error> {
        self.write_user_string(v.as_bytes())
    }
    fn emit_enum(&mut self, _: &str,
                 f: |&mut Encoder<W>| -> Result<(), Error>)
                -> Result<(), Error> {
        f(self)
    }
    fn emit_enum_variant(&mut self, v_name: &str, _: uint, len: uint,
                         f: |&mut Encoder<W>| -> Result<(), Error>)
                        -> Result<(), Error> {
        match len {
            0 => self.write_bytes(v_name.as_bytes()),
            1 => f(self),
            _ => self.err("Cannot encode enum variants \
                           with more than one argument."),
        }
    }
    fn emit_enum_variant_arg(&mut self, _: uint,
                             f: |&mut Encoder<W>| -> Result<(), Error>)
                            -> Result<(), Error> {
        f(self)
    }
    fn emit_enum_struct_variant(&mut self, v_name: &str, v_id: uint, len: uint,
                                f: |&mut Encoder<W>| -> Result<(), Error>)
                               -> Result<(), Error> {
        self.emit_enum_variant(v_name, v_id, len, f)
    }
    fn emit_enum_struct_variant_field(&mut self, _: &str, _: uint,
                                      _: |&mut Encoder<W>| -> Result<(), Error>)
                                     -> Result<(), Error> {
        self.err("Cannot encode enum variants with arguments.")
    }
    fn emit_struct(&mut self, _: &str, len: uint,
                   f: |&mut Encoder<W>| -> Result<(), Error>)
                  -> Result<(), Error> {
        self.emit_seq(len, f)
    }
    fn emit_struct_field(&mut self, _: &str, f_idx: uint,
                         f: |&mut Encoder<W>| -> Result<(), Error>)
                        -> Result<(), Error> {
        self.emit_seq_elt(f_idx, f)
    }
    fn emit_tuple(&mut self, len: uint,
                  f: |&mut Encoder<W>| -> Result<(), Error>)
                 -> Result<(), Error> {
        self.emit_seq(len, f)
    }
    fn emit_tuple_arg(&mut self, idx: uint,
                      f: |&mut Encoder<W>| -> Result<(), Error>)
                     -> Result<(), Error> {
        self.emit_seq_elt(idx, f)
    }
    fn emit_tuple_struct(&mut self, _: &str, _: uint,
                         _: |&mut Encoder<W>| -> Result<(), Error>)
                        -> Result<(), Error> {
        unimplemented!()
    }
    fn emit_tuple_struct_arg(&mut self, _: uint,
                             _: |&mut Encoder<W>| -> Result<(), Error>)
                            -> Result<(), Error> {
        unimplemented!()
    }
    fn emit_option(&mut self, f: |&mut Encoder<W>| -> Result<(), Error>)
                  -> Result<(), Error> {
        f(self)
    }
    fn emit_option_none(&mut self) -> Result<(), Error> { Ok(()) }
    fn emit_option_some(&mut self, f: |&mut Encoder<W>| -> Result<(), Error>)
                       -> Result<(), Error> {
        f(self)
    }
    fn emit_seq(&mut self, len: uint,
                f: |this: &mut Encoder<W>| -> Result<(), Error>)
               -> Result<(), Error> {
        try!(self.set_first_len(len));
        try!(f(self));
        self.write_lineterm()
    }
    fn emit_seq_elt(&mut self, idx: uint,
                    f: |this: &mut Encoder<W>| -> Result<(), Error>)
                   -> Result<(), Error> {
        if idx > 0 {
            let sep = self.sep;
            try!(self.write_bytes([sep]));
        }
        f(self)
    }
    fn emit_map(&mut self, _: uint,
                _: |&mut Encoder<W>| -> Result<(), Error>)
               -> Result<(), Error> {
        unimplemented!()
    }
    fn emit_map_elt_key(&mut self, _: uint,
                        _: |&mut Encoder<W>| -> Result<(), Error>)
                       -> Result<(), Error> {
        unimplemented!()
    }
    fn emit_map_elt_val(&mut self, _: uint,
                        _: |&mut Encoder<W>| -> Result<(), Error>)
                       -> Result<(), Error> {
        unimplemented!()
    }
}

fn quote(mut s: &[u8]) -> ByteString {
    let mut buf = Vec::with_capacity(s.len() + 2);

    buf.push(b'"');
    loop {
        match s.position_elem(&b'"') {
            None => {
                buf.push_all(s);
                break
            }
            Some(next_quote) => {
                buf.push_all(s.slice_to(next_quote + 1));
                buf.push(b'"');
                s = s.slice_from(next_quote + 1);
            }
        }
    }
    buf.push(b'"');
    ByteString(buf)
}
