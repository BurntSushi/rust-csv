use serde::ser;

use {Result, Error};

use common::{Encoded, EncodedHelper, float_to_string};

macro_rules! write_display {
    ($name:ident, $t:ty) => {
        #[inline]
        fn $name(&mut self, value: $t) -> Result<()> {
            self.push_to_string(format!("{}", value))
        }
    }
}

impl ser::Serializer for Encoded {
    type Error = Error;

    write_display!(serialize_bool, bool);

    write_display!(serialize_i8, i8);
    write_display!(serialize_i16, i16);
    write_display!(serialize_i32, i32);
    write_display!(serialize_i64, i64);
    write_display!(serialize_isize, isize);

    write_display!(serialize_u8, u8);
    write_display!(serialize_u16, u16);
    write_display!(serialize_u32, u32);
    write_display!(serialize_u64, u64);
    write_display!(serialize_usize, usize);

    write_display!(serialize_str, &str);
    write_display!(serialize_char, char);

    #[inline]
    fn serialize_f32(&mut self, value: f32) -> Result<()> {
        self.push_to_string(float_to_string(value as f64))
    }

    #[inline]
    fn serialize_f64(&mut self, value: f64) -> Result<()> {
        self.push_to_string(float_to_string(value))
    }

    #[inline]
    fn serialize_none(&mut self) -> Result<()> {
        self.push_bytes::<&[u8]>(&[])
    }

    #[inline]
    fn serialize_some<V>(&mut self, value: V) -> Result<()>
        where V: ser::Serialize
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(&mut self) -> Result<()> {
        unimplemented!()
    }

    #[inline]
    fn serialize_seq<V>(&mut self, mut visitor: V) -> Result<()>
        where V: ser::SeqVisitor,
    {
        while let Some(()) = try!(visitor.visit(self)) {};
        Ok(())
    }

    #[inline]
    fn serialize_seq_elt<T>(&mut self, value: T) -> Result<()>
        where T: ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_map<V>(&mut self, mut visitor: V) -> Result<()>
        where V: ser::MapVisitor,
    {
        while let Some(()) = try!(visitor.visit(self)) {};
        Ok(())
    }

    #[inline]
    fn serialize_map_elt<K, V>(&mut self, _key: K, value: V) -> Result<()>
        where K: ser::Serialize,
              V: ser::Serialize,
    {
        value.serialize(self)
    }

}
