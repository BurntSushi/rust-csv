use std::default::Default;
use std::from_str::FromStr;
use std::io::{MemReader, BufferedReader, File, IoResult};

use serialize;
use serialize::Decodable;

use {ByteString, Parser};
use {Error, ErrEOF};

/// A decoder can decode CSV values (or entire documents) into values with
/// Rust types automatically.
///
/// Raw records (as strings) can also be accessed with any of the `record`
/// or `iter` methods.
pub struct Decoder<R> {
    stack: Vec<Value>,
    p: Parser<R>,
}

/// A representation of a value found in a CSV document.
/// A CSV document's structure is simple (non-recursive).
enum Value {
    Record(Vec<String>),
    Field(String),
}

impl Value {
    fn is_record(&self) -> bool {
        match *self {
            Record(_) => true,
            Field(_) => false,
        }
    }

    fn is_string(&self) -> bool {
        !self.is_record()
    }
}

impl Decoder<IoResult<File>> {
    /// Creates a new CSV decoder from a file using the file path given.
    pub fn from_file(path: &Path) -> Decoder<IoResult<File>> {
        Decoder::from_reader(File::open(path))
    }
}

impl Decoder<MemReader> {
    /// Creates a new CSV decoder that reads CSV data from the string given.
    pub fn from_str(s: &str) -> Decoder<MemReader> {
        Decoder::from_bytes(s.as_bytes())
    }

    /// Creates a new CSV decoder that reads CSV data from the bytes given.
    pub fn from_bytes(bytes: &[u8]) -> Decoder<MemReader> {
        let r = MemReader::new(bytes.to_vec());
        Decoder::from_reader(r)
    }
}

impl<R: Reader> Decoder<R> {
    /// Creates a new CSV decoder that reads CSV data from the `Reader` given.
    /// Note that the `Reader` given may be a stream. Data is only read as it
    /// is decoded.
    ///
    /// The reader given is wrapped in a `BufferedReader` for you.
    pub fn from_reader(r: R) -> Decoder<R> {
        Decoder::from_buffer(BufferedReader::new(r))
    }

    /// This is just like `from_reader`, except it allows you to specify
    /// the capacity used in the underlying buffer.
    pub fn from_reader_capacity(r: R, cap: uint) -> Decoder<R> {
        Decoder::from_buffer(BufferedReader::with_capacity(cap, r))
    }

    fn from_buffer(buf: BufferedReader<R>) -> Decoder<R> {
        Decoder {
            stack: vec!(),
            p: Parser {
                buf: buf,
                sep: b',',
                same_len: true,
                first_len: 0,
                no_headers: false,
                first_record: None,
                cur: Some(0),
                look: None,
                line: 0,
                col: 0,
                byte: 0,
                byte_record_start: 0,
                returned_non_header: false,
            },
        }
    }

    /// Sets the separator character that delimits values in a record.
    ///
    /// This may be changed at any time.
    pub fn separator(mut self, c: u8) -> Decoder<R> {
        self.p.sep = c;
        self
    }

    /// When `yes` is `true`, all records decoded must have the same length.
    /// If a record is decoded that has a different length than other records
    /// already decoded, the decoding will fail.
    ///
    /// This may be toggled at any time.
    pub fn enforce_same_length(mut self, yes: bool) -> Decoder<R> {
        self.p.same_len = yes;
        self
    }

    /// When called, the first record is not interpreted as a header
    /// row. Otherwise, the first record is always saved as headers and not
    /// returned by the normal record iterators. (The header row can be
    /// retrieved with the `headers` method.)
    ///
    /// Calling this method after the first row has been decoded will result
    /// in a task failure.
    pub fn no_headers(mut self) -> Decoder<R> {
        assert!(self.p.first_len == 0);
        self.p.no_headers = true;
        self
    }
}

/// The following methods provide type-aware decoding of CSV records.
impl<R: Reader> Decoder<R> {
    /// Decodes the next record for some type. Note that since this decodes
    /// records, only types corresponding to a record (like structs, tuples or
    /// vectors) can be used.
    pub fn decode<D: Decodable<Decoder<R>, Error>>
                 (&mut self) -> Result<D, Error> {
        Decodable::decode(self)
    }

    /// Provides an iterator to decode one record at a time. Note that this
    /// usually needs to have its type parameter `D` instantiated explicitly.
    /// For example:
    ///
    /// ```no_run
    /// let mut dec = csv::Decoder::from_str("abc,1");
    /// let mut iter = dec.iter_decode::<(String, uint)>();
    /// ```
    pub fn iter_decode<'a, D: Decodable<Decoder<R>, Error>>
                      (&'a mut self) -> Records<'a, R, D> {
        Records {
            decoder: self,
            decode: |d| d.decode(),
            errored: false,
        }
    }

    /// Calls `decode` on every record in the CSV data until EOF and returns
    /// them as a vector. If there was an error decoding a vector, parsing is
    /// stopped and the error is returned.
    pub fn decode_all<D: Decodable<Decoder<R>, Error>>
                     (&mut self) -> Result<Vec<D>, Error> {
        let mut records: Vec<D> = vec!();
        loop {
            match self.decode() {
                Ok(r) => records.push(r),
                Err(ErrEOF) => break,
                Err(err) => return Err(err),
            }
        }
        Ok(records)
    }
}

/// The following methods provide direct access to CSV records as Unicode
/// strings or as byte strings.
impl<R: Reader> Decoder<R> {
    /// Circumvents the decoding interface and forces the parsing of the next
    /// record and returns it. A record returned by this method will never be
    /// decoded.
    pub fn record(&mut self) -> Result<Vec<String>, Error> {
        to_utf8_record(try!(self.record_bytes())).or_else(|m| self.err(m))
    }

    /// Returns the next record as a vector of raw byte strings.
    pub fn record_bytes(&mut self) -> Result<Vec<ByteString>, Error> {
        self.p.parse_record(false)
    }

    /// Circumvents the decoding interface and iterates over the records as
    /// vectors of strings. A record returned by this method will never be
    /// decoded.
    pub fn iter<'a>(&'a mut self) -> Records<'a, R, Vec<String>> {
        Records {
            decoder: self,
            decode: |d| d.record(),
            errored: false,
        }
    }

    /// Circumvents the decoding interface and iterates over the records as
    /// vectors of byte strings. A record returned by this method will never be
    /// decoded.
    pub fn iter_bytes<'a>(&'a mut self) -> Records<'a, R, Vec<ByteString>> {
        Records {
            decoder: self,
            decode: |d| d.record_bytes(),
            errored: false,
        }
    }

    /// Returns the header record for the underlying CSV data. This method may
    /// be called repeatedly and at any time.
    pub fn headers(&mut self) -> Result<Vec<String>, Error> {
        to_utf8_record(try!(self.headers_bytes())).or_else(|m| self.err(m))
    }

    /// Returns the headers as raw byte strings.
    pub fn headers_bytes(&mut self) -> Result<Vec<ByteString>, Error> {
        if self.p.first_record.is_none() {
            let _ = try!(self.p.parse_record(true));
            assert!(self.p.first_record.is_some());
        }
        Ok(self.p.first_record.clone().unwrap())
    }

    /// Returns the byte offset in the data stream that corresponds to the
    /// start of the previously read record. The "start" of the record
    /// corresponds to a position at which this decoder would return that
    /// record if it started reading at that offset.
    pub fn byte_offset(&self) -> u64 {
        self.p.byte_record_start
    }
}

/// An iterator that yields records of various types. All yielded records are
/// wrapped in a `Result`.
///
/// `'a` corresponds to the lifetime of the decoder.
///
/// `R` corresponds to the underlying reader of the decoder. This must satisfy
/// the `Reader` constraint.
///
/// `T` corresponds to the type of the value yielded. When using the `iter`
/// or `iter_bytes` methods, this type corresponds to `Vec<String>` and
/// `Vec<ByteString>`, respectively. When using `iter_decode`, the type varies
/// depending on what the record is being decoded into.
pub struct Records<'a, R: 'a, T> {
    decoder: &'a mut Decoder<R>,
    decode: |&mut Decoder<R>|:'a -> Result<T, Error>,
    errored: bool,
}

impl<'a, R: Reader, T> Iterator<Result<T, Error>> for Records<'a, R, T> {
    fn next(&mut self) -> Option<Result<T, Error>> {
        if self.errored {
            return None;
        }
        let ref mut d = self.decoder;
        match (self.decode)(*d) {
            Ok(r) => Some(Ok(r)),
            Err(ErrEOF) => None,
            Err(err) => { self.errored = true; Some(Err(err)) }
        }
    }
}

impl<R: Reader> Decoder<R> {
    fn pop(&mut self) -> Result<Value, Error> {
        if self.stack.len() == 0 {
            try!(self.read_to_stack())
        }
        // On successful return, read_to_stack guarantees a non-empty stack.
        assert!(self.stack.len() > 0);
        Ok(self.stack.pop().unwrap())
    }

    fn read_to_stack(&mut self) -> Result<(), Error> {
        let r = try!(self.record());
        self.push_record(r);
        Ok(())
    }

    fn pop_record(&mut self) -> Result<Vec<String>, Error> {
        match try!(self.pop()) {
            Record(r) => Ok(r),
            Field(s) => {
                self.err(format!("Expected record but got value '{}'.", s))
            }
        }
    }

    fn pop_string(&mut self) -> Result<String, Error> {
        match try!(self.pop()) {
            Record(_) => {
                self.err(format!("Expected value but got record."))
            }
            Field(s) => Ok(s),
        }
    }

    fn pop_from_str<T: FromStr + Default>(&mut self) -> Result<T, Error> {
        let s = try!(self.pop_string());
        let s = s.as_slice().trim();
        match FromStr::from_str(s) {
            Some(t) => Ok(t),
            None => {
                self.err(format!("Failed converting '{}' from str.", s))
            }
        }
    }

    fn push_record(&mut self, r: Vec<String>) {
        self.stack.push(Record(r))
    }

    fn push_string(&mut self, s: String) {
        self.stack.push(Field(s))
    }

    fn num_strings_on_top(&self) -> uint {
        let mut count = 0;
        for v in self.stack.iter().rev() {
            if v.is_string() {
                count += 1;
            } else {
                break
            }
        }
        count
    }

    fn err<T, S: StrAllocating>(&self, msg: S) -> Result<T, Error> {
        Err(self.p.err(msg))
    }
}

impl<R: Reader> serialize::Decoder<Error> for Decoder<R> {
    fn error(&mut self, err: &str) -> Error {
        self.p.err(err)
    }
    fn read_nil(&mut self) -> Result<(), Error> { unimplemented!() }
    fn read_uint(&mut self) -> Result<uint, Error> { self.pop_from_str() }
    fn read_u64(&mut self) -> Result<u64, Error> { self.pop_from_str() }
    fn read_u32(&mut self) -> Result<u32, Error> { self.pop_from_str() }
    fn read_u16(&mut self) -> Result<u16, Error> { self.pop_from_str() }
    fn read_u8(&mut self) -> Result<u8, Error> { self.pop_from_str() }
    fn read_int(&mut self) -> Result<int, Error> { self.pop_from_str() }
    fn read_i64(&mut self) -> Result<i64, Error> { self.pop_from_str() }
    fn read_i32(&mut self) -> Result<i32, Error> { self.pop_from_str() }
    fn read_i16(&mut self) -> Result<i16, Error> { self.pop_from_str() }
    fn read_i8(&mut self) -> Result<i8, Error> { self.pop_from_str() }
    fn read_bool(&mut self) -> Result<bool, Error> { self.pop_from_str() }
    fn read_f64(&mut self) -> Result<f64, Error> { self.pop_from_str() }
    fn read_f32(&mut self) -> Result<f32, Error> { self.pop_from_str() }
    fn read_char(&mut self) -> Result<char, Error> {
        let s = try!(self.pop_string());
        let chars: Vec<char> = s.as_slice().chars().collect();
        if chars.len() != 1 {
            return self.err(format!(
                "Expected single character but got '{}'.", s))
        }
        Ok(chars[0])
    }
    fn read_str(&mut self) -> Result<String, Error> {
        self.pop_string()
    }
    fn read_enum<T>(&mut self, _: &str,
                    f: |&mut Decoder<R>| -> Result<T, Error>)
                   -> Result<T, Error> {
        f(self)
    }
    fn read_enum_variant<T>(&mut self, names: &[&str],
                            f: |&mut Decoder<R>, uint| -> Result<T, Error>)
                           -> Result<T, Error> {
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
                                f: |&mut Decoder<R>| -> Result<T, Error>)
                               -> Result<T, Error> {
        f(self)
    }
    fn read_enum_struct_variant<T>(&mut self, names: &[&str],
                                   f: |&mut Decoder<R>, uint|
                                      -> Result<T, Error>)
                                  -> Result<T, Error> {
        self.read_enum_variant(names, f)
    }
    fn read_enum_struct_variant_field<T>(&mut self, _: &str, f_idx: uint,
                                         f: |&mut Decoder<R>|
                                            -> Result<T, Error>)
                                        -> Result<T, Error> {
        self.read_enum_variant_arg(f_idx, f)
    }
    fn read_struct<T>(&mut self, s_name: &str, len: uint,
                      f: |&mut Decoder<R>| -> Result<T, Error>)
                     -> Result<T, Error> {
        let r = try!(self.pop_record());
        if r.len() < len {
            return self.err(
                format!("Struct '{}' has {} fields but current record \
                         has {} fields.", s_name, len, r.len()));
        }
        for v in r.into_iter().rev() {
            self.push_string(v)
        }
        let result = f(self);
        match result {
            err @ Err(_) => err,
            ok @ Ok(_) => {
                assert!(self.num_strings_on_top() == 0);
                ok
            }
        }
    }
    fn read_struct_field<T>(&mut self, _: &str, _: uint,
                            f: |&mut Decoder<R>| -> Result<T, Error>)
                           -> Result<T, Error> {
        f(self)
    }
    fn read_tuple<T>(&mut self, f: |&mut Decoder<R>, uint| -> Result<T, Error>)
                    -> Result<T, Error> {
        let r = try!(self.pop_record());
        let len = r.len();
        for v in r.into_iter().rev() {
            self.push_string(v)
        }
        f(self, len)
    }
    fn read_tuple_arg<T>(&mut self, _: uint,
                         f: |&mut Decoder<R>| -> Result<T, Error>)
                        -> Result<T, Error> {
        f(self)
    }
    fn read_tuple_struct<T>(&mut self, _: &str,
                            _: |&mut Decoder<R>, uint| -> Result<T, Error>)
                           -> Result<T, Error> {
        unimplemented!()
    }
    fn read_tuple_struct_arg<T>(&mut self, _: uint,
                                _: |&mut Decoder<R>| -> Result<T, Error>)
                               -> Result<T, Error> {
        unimplemented!()
    }
    fn read_option<T>(&mut self,
                      f: |&mut Decoder<R>, bool| -> Result<T, Error>)
                     -> Result<T, Error> {
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
    fn read_seq<T>(&mut self, f: |&mut Decoder<R>, uint| -> Result<T, Error>)
                  -> Result<T, Error> {
        match self.num_strings_on_top() {
            0 => {
                let r = try!(self.pop_record());
                let len = r.len();
                for v in r.into_iter().rev() {
                    self.push_string(v)
                }
                f(self, len)
            }
            n => {
                f(self, n)
            }
        }
    }
    fn read_seq_elt<T>(&mut self, _: uint,
                       f: |&mut Decoder<R>| -> Result<T, Error>)
                      -> Result<T, Error> {
        f(self)
    }
    fn read_map<T>(&mut self, _: |&mut Decoder<R>, uint| -> Result<T, Error>)
                  -> Result<T, Error> {
        unimplemented!()
    }
    fn read_map_elt_key<T>(&mut self, _: uint,
                           _: |&mut Decoder<R>| -> Result<T, Error>)
                          -> Result<T, Error> {
        unimplemented!()
    }
    fn read_map_elt_val<T>(&mut self, _: uint,
                           _: |&mut Decoder<R>| -> Result<T, Error>)
                          -> Result<T, Error> {
        unimplemented!()
    }
}

fn to_lower(s: &str) -> String {
    s.chars().map(|c| c.to_lowercase()).collect()
}

fn to_utf8_record(raw: Vec<ByteString>) -> Result<Vec<String>, String> {
    let mut strs = Vec::with_capacity(raw.len());
    for bytes in raw.into_iter() {
        match bytes.to_utf8_string() {
            Err(bytes) => {
                return Err(format!(
                    "Could not decode bytes as UTF-8: {}", bytes));
            }
            Ok(s) => strs.push(s),
        }
    }
    Ok(strs)
}
