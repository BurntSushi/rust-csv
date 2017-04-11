use std::error::Error as StdError;
use std::fmt;
use std::io;

use serde::ser::{
    Error as SerdeError,
    Serialize, Serializer,
    SerializeSeq, SerializeTuple, SerializeTupleStruct,
    SerializeTupleVariant, SerializeMap, SerializeStruct,
    SerializeStructVariant,
};

use writer::Writer;

use self::SerializeErrorKind as SEK;

struct SeRecord<'w, W: 'w + io::Write> {
    wtr: &'w mut Writer<W>,
    field: u64,
}

impl<'a, 'w, W: io::Write> Serializer for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = SerializeError;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        if v {
            self.wtr.write_field("true").unwrap();
        } else {
            self.wtr.write_field("false").unwrap();
        }
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_some<T: ?Sized + Serialize>(
        self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_unit_struct(
        self,
        name: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: usize,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        variant_index: usize,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_seq(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeSeq, Self::Error> {
        unimplemented!()
    }

    fn serialize_seq_fixed_size(
        self,
        size: usize,
    ) -> Result<Self::SerializeSeq, Self::Error> {
        unimplemented!()
    }

    fn serialize_tuple(
        self,
        len: usize,
    ) -> Result<Self::SerializeTuple, Self::Error> {
        unimplemented!()
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        unimplemented!()
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: usize,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        unimplemented!()
    }

    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeMap, Self::Error> {
        unimplemented!()
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        unimplemented!()
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: usize,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        unimplemented!()
    }
}

impl<'a, 'w, W: io::Write> SerializeSeq for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, 'w, W: io::Write> SerializeTuple for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, 'w, W: io::Write> SerializeTupleStruct for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, 'w, W: io::Write> SerializeTupleVariant for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, 'w, W: io::Write> SerializeMap for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_key<T: ?Sized + Serialize>(
        &mut self,
        key: &T,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_value<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, 'w, W: io::Write> SerializeStruct for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, 'w, W: io::Write> SerializeStructVariant for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeError {
    field: Option<u64>,
    kind: SerializeErrorKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SerializeErrorKind {
    Message(String),
}

impl SerdeError for SerializeError {
    fn custom<T: fmt::Display>(msg: T) -> SerializeError {
        SerializeError {
            field: None,
            kind: SEK::Message(msg.to_string()),
        }
    }
}

impl StdError for SerializeError {
    fn description(&self) -> &str {
        self.kind.description()
    }
}

impl fmt::Display for SerializeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(field) = self.field {
            write!(f, "field {}: {}", field, self.kind)
        } else {
            write!(f, "{}", self.kind)
        }
    }
}

impl fmt::Display for SerializeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::SerializeErrorKind::*;

        match *self {
            Message(ref msg) => write!(f, "{}", msg),
        }
    }
}

impl SerializeError {
    /// Return the field index (starting at 0) of this error, if available.
    pub fn field(&self) -> Option<u64> {
        self.field
    }

    /// Return the underlying error kind.
    pub fn kind(&self) -> &SerializeErrorKind {
        &self.kind
    }
}

impl SerializeErrorKind {
    fn description(&self) -> &str {
        use self::SerializeErrorKind::*;

        match *self {
            Message(_) => "serialization error",
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use writer::Writer;

    use super::SeRecord;

    #[test]
    fn scratch() {
        let mut wtr = Writer::from_writer(vec![]);
        {
            let mut ser = SeRecord {
                wtr: &mut wtr,
                field: 0,
            };
            true.serialize(&mut ser).unwrap();
        }
        wtr.write_record(None::<&str>).unwrap();
        let res = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        assert_eq!(res, "true\n");
    }
}
