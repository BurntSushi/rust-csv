use rustc_serialize as serialize;
use {Result, Error};

use common::{Encoded, EncodedHelper, float_to_string};

impl serialize::Encoder for Encoded {
    type Error = Error;

    fn emit_nil(&mut self) -> Result<()> { unimplemented!() }
    fn emit_usize(&mut self, v: usize) -> Result<()> {
        self.push_to_string(v)
    }
    fn emit_u64(&mut self, v: u64) -> Result<()> { self.push_to_string(v) }
    fn emit_u32(&mut self, v: u32) -> Result<()> { self.push_to_string(v) }
    fn emit_u16(&mut self, v: u16) -> Result<()> { self.push_to_string(v) }
    fn emit_u8(&mut self, v: u8) -> Result<()> { self.push_to_string(v) }
    fn emit_isize(&mut self, v: isize) -> Result<()> {
        self.push_to_string(v)
    }
    fn emit_i64(&mut self, v: i64) -> Result<()> { self.push_to_string(v) }
    fn emit_i32(&mut self, v: i32) -> Result<()> { self.push_to_string(v) }
    fn emit_i16(&mut self, v: i16) -> Result<()> { self.push_to_string(v) }
    fn emit_i8(&mut self, v: i8) -> Result<()> { self.push_to_string(v) }
    fn emit_bool(&mut self, v: bool) -> Result<()> {
        self.push_to_string(v)
    }
    fn emit_f64(&mut self, v: f64) -> Result<()> {
        self.push_string(float_to_string(v))
    }
    fn emit_f32(&mut self, v: f32) -> Result<()> {
        self.push_string(float_to_string(v as f64))
    }
    fn emit_char(&mut self, v: char) -> Result<()> {
        self.push_string(format!("{}", v))
    }
    fn emit_str(&mut self, v: &str) -> Result<()> {
        self.push_string(v)
    }
    fn emit_enum<F>(&mut self, _: &str, f: F) -> Result<()>
                where F: FnOnce(&mut Encoded) -> Result<()> {
        f(self)
    }
    fn emit_enum_variant<F>(&mut self, v_name: &str, _: usize,
                            len: usize, f: F) -> Result<()>
                        where F: FnOnce(&mut Encoded) -> Result<()> {
        match len {
            0 => self.push_string(v_name),
            1 => f(self),
            _ => Err(
                Error::Encode("Cannot encode enum variants \
                               with more than one argument.".to_string())),
        }
    }
    fn emit_enum_variant_arg<F>(&mut self, _: usize, f: F) -> Result<()>
                            where F: FnOnce(&mut Encoded) -> Result<()> {
        f(self)
    }
    fn emit_enum_struct_variant<F>(&mut self, v_name: &str, v_id: usize,
                                   len: usize, f: F)
                                  -> Result<()>
                               where F: FnOnce(&mut Encoded) -> Result<()> {
        self.emit_enum_variant(v_name, v_id, len, f)
    }
    fn emit_enum_struct_variant_field<F>(&mut self, _: &str, _: usize, _: F)
                                         -> Result<()>
            where F: FnOnce(&mut Encoded) -> Result<()> {
        Err(Error::Encode("Cannot encode enum \
                           variants with arguments.".to_string()))
    }
    fn emit_struct<F>(&mut self, _: &str, len: usize, f: F) -> Result<()>
                  where F: FnOnce(&mut Encoded) -> Result<()> {
        self.emit_seq(len, f)
    }
    fn emit_struct_field<F>(&mut self, _: &str, f_idx: usize, f: F)
                           -> Result<()>
                        where F: FnOnce(&mut Encoded) -> Result<()> {
        self.emit_seq_elt(f_idx, f)
    }
    fn emit_tuple<F>(&mut self, len: usize, f: F) -> Result<()>
                 where F: FnOnce(&mut Encoded) -> Result<()> {
        self.emit_seq(len, f)
    }
    fn emit_tuple_arg<F>(&mut self, idx: usize, f: F) -> Result<()>
                     where F: FnOnce(&mut Encoded) -> Result<()> {
        self.emit_seq_elt(idx, f)
    }
    fn emit_tuple_struct<F>(&mut self, _: &str, _: usize, _: F)
                           -> Result<()>
                        where F: FnOnce(&mut Encoded) -> Result<()> {
        unimplemented!()
    }
    fn emit_tuple_struct_arg<F>(&mut self, _: usize, _: F) -> Result<()>
                            where F: FnOnce(&mut Encoded) -> Result<()> {
        unimplemented!()
    }
    fn emit_option<F>(&mut self, f: F) -> Result<()>
                  where F: FnOnce(&mut Encoded) -> Result<()> {
        f(self)
    }
    fn emit_option_none(&mut self) -> Result<()> {
        self.push_bytes::<&[u8]>(&[])
    }
    fn emit_option_some<F>(&mut self, f: F) -> Result<()>
                       where F: FnOnce(&mut Encoded) -> Result<()> {
        f(self)
    }
    fn emit_seq<F>(&mut self, _: usize, f: F) -> Result<()>
               where F: FnOnce(&mut Encoded) -> Result<()> {
        f(self)
    }
    fn emit_seq_elt<F>(&mut self, _: usize, f: F) -> Result<()>
                   where F: FnOnce(&mut Encoded) -> Result<()> {
        f(self)
    }
    fn emit_map<F>(&mut self, _: usize, _: F) -> Result<()>
               where F: FnOnce(&mut Encoded) -> Result<()> {
        unimplemented!()
    }
    fn emit_map_elt_key<F>(&mut self, _: usize, _: F) -> Result<()>
                       where F: FnOnce(&mut Encoded) -> Result<()> {
        unimplemented!()
    }
    fn emit_map_elt_val<F>(&mut self, _: usize, _: F) -> Result<()>
                       where F: FnOnce(&mut Encoded) -> Result<()> {
        unimplemented!()
    }
}
