use {Result, Error};

use common::{Decoded, DecodedHelper};

use serde::de::{self, Deserializer, Deserialize, Visitor};

macro_rules! simple_deserialize {
    ($name:ident, $visitfn:ident) => {
        #[inline]
        fn $name<V: Visitor>(&mut self, mut visitor: V) -> Result<V::Value> {
            let v = try!(self.pop_from_str());
            visitor.$visitfn(v)
        }
    }
}

impl Deserializer for Decoded {
    type Error = Error;

    #[inline]
    fn deserialize<V: Visitor>(&mut self, mut _visitor: V) -> Result<V::Value> {
        unimplemented!();
    }

    simple_deserialize!(deserialize_i8,  visit_i8);
    simple_deserialize!(deserialize_i16, visit_i16);
    simple_deserialize!(deserialize_i32, visit_i32);
    simple_deserialize!(deserialize_i64, visit_i64);
    simple_deserialize!(deserialize_isize, visit_isize);

    simple_deserialize!(deserialize_u8,  visit_u8);
    simple_deserialize!(deserialize_u16, visit_u16);
    simple_deserialize!(deserialize_u32, visit_u32);
    simple_deserialize!(deserialize_u64, visit_u64);
    simple_deserialize!(deserialize_usize, visit_usize);

    simple_deserialize!(deserialize_f32, visit_f32);
    simple_deserialize!(deserialize_f64, visit_f64);

    #[inline]
    fn deserialize_bool<V: Visitor>(&mut self, mut visitor: V) -> Result<V::Value> {
        let s = try!(self.pop_string());
        let s = s.trim();
        match s {
            "true" => visitor.visit_bool(true),
            "false" => visitor.visit_bool(false),
            _ => self.err(format!("Expected 'true' or 'false' but found '{}'.", s))
        }
    }

    #[inline]
    fn deserialize_str<V: Visitor>(&mut self, mut visitor: V) -> Result<V::Value> {
        let v = try!(self.pop_string());
        visitor.visit_str(&v)
    }

    #[inline]
    fn deserialize_char<V: Visitor>(&mut self, mut visitor: V) -> Result<V::Value> {
        let s = try!(self.pop_string());
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != 1 {
            return self.err(format!("Expected single character but got '{}'.", s))
        }
        visitor.visit_char(chars[0])
    }

    #[inline]
    fn deserialize_option<V: Visitor>(&mut self, mut visitor: V) -> Result<V::Value> {
        let val = try!(self.pop_string());
        if val.is_empty() {
            visitor.visit_none()
        } else {
            self.push_string(val);
            match visitor.visit_some(self) {
                Ok(v) => Ok(v),
                Err(_) => visitor.visit_none()
            }
        }
    }

    #[inline]
    fn deserialize_tuple<V: Visitor>(&mut self, _len: usize,
                            mut visitor: V) -> Result<V::Value> {
        struct TupleVisitor<'a>(&'a mut Decoded);
        impl<'a> de::SeqVisitor for TupleVisitor<'a> {
            type Error = Error;

            fn visit<T>(&mut self) -> Result<Option<T>> where T: Deserialize {
                Deserialize::deserialize(self.0).map(|v| Some(v))
            }

            fn end(&mut self) -> Result<()> {
                Ok(())
            }
        }
        visitor.visit_seq(TupleVisitor(self))
    }

    #[inline]
    fn deserialize_seq<V: Visitor>(&mut self, mut visitor: V) -> Result<V::Value> {
        struct SeqVisitor<'a>(&'a mut Decoded);
        impl<'a> de::SeqVisitor for SeqVisitor<'a> {
            type Error = Error;

            fn visit<T>(&mut self) -> Result<Option<T>> where T: Deserialize {
                // TODO is this the correct way to handle vecs?
                // i.e. keep trying to fetch more until failing with 'end of bytevec'
                // error, then catch Err and return OK
                match Deserialize::deserialize(self.0) {
                    Ok(v) => Ok(Some(v)),
                    Err(_) => Ok(None)
                }
            }

            fn end(&mut self) -> Result<()> {
                Ok(())
            }
        }
        visitor.visit_seq(SeqVisitor(self))
    }

    #[inline]
    fn deserialize_enum<V>(&mut self, _enum: &'static str,
                           variants: &'static [&'static str],
                           mut visitor: V) -> Result<V::Value>
        where V: de::EnumVisitor
    {
        struct VariantVisitor<'a>(usize, &'a mut Decoded);

        impl<'a> de::VariantVisitor for VariantVisitor<'a> {
            type Error = Error;

            #[inline]
            fn visit_variant<D>(&mut self) -> Result<D> where D: Deserialize {
                use serde::de::value::ValueDeserializer;
                let mut deserializer = self.0.into_deserializer();
                Deserialize::deserialize(&mut deserializer)
            }

            #[inline]
            fn visit_newtype<D>(&mut self) -> Result<D> where D: Deserialize {
                Deserialize::deserialize(self.1)
            }
        }

        let cur = try!(self.pop_string());
        self.push_string(cur.clone());
        // try each varient in turn
        for i in 0..variants.len() {
            match visitor.visit(VariantVisitor(i, self)) {
                Ok(v) => return Ok(v),
                Err(_) => { self.push_string(cur.clone()); }
            }
        }
        // No variant matched, bail out
        let cur = try!(self.pop_string());
        self.err(format!("Failed to decode variant '{}'", cur))
    }

    #[inline]
    fn deserialize_struct<V>(&mut self, _name: &'static str,
                             fields: &'static [&'static str],
                             visitor: V) -> Result<V::Value>
        where V: Visitor
    {
        self.deserialize_tuple(fields.len(), visitor)
    }
}
