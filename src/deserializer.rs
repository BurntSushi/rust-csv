use std::error::{Error as StdError};
use std::fmt;
use std::iter;
use std::num;
use std::str;

use serde::de::{Deserializer, Error as SerdeError, Visitor};

use reader::Position;
use string_record::{StringRecord, StringRecordIter};

use self::{DeserializeErrorKind as DEK};

pub struct DeStringRecord<'r> {
    it: iter::Peekable<StringRecordIter<'r>>,
    headers: Option<StringRecordIter<'r>>,
    pos: Option<Position>,
    field: u64,
}

impl<'r> DeStringRecord<'r> {
    pub fn new(
        rec: &'r StringRecord,
        headers: Option<&'r StringRecord>,
        pos: Option<Position>,
    ) -> DeStringRecord<'r> {
        DeStringRecord {
            it: rec.iter().peekable(),
            headers: headers.map(|r| r.iter()),
            pos: pos,
            field: 0,
        }
    }

    /// Returns an error corresponding to the most recently extracted field.
    fn error(&self, kind: DeserializeErrorKind) -> DeserializeError {
        DeserializeError {
            pos: self.pos.clone(),
            field: Some(self.field.checked_sub(1).unwrap()),
            kind: kind,
        }
    }

    /// Returns an arbitrary catch-all error for the most recently extracted
    /// field.
    fn message(&self, msg: String) -> DeserializeError {
        self.error(DEK::Message(msg))
    }

    /// Extracts the next field from the underlying record.
    fn next_field(&mut self) -> Result<&'r str, DeserializeError> {
        match self.it.next() {
            Some(field) => {
                self.field += 1;
                Ok(field)
            }
            None => Err(DeserializeError {
                pos: self.pos.clone(),
                field: None,
                kind: DEK::UnexpectedEndOfRow,
            })
        }
    }

    /// Peeks at the next field from the underlying record.
    fn peek_field(&mut self) -> Option<&'r str> {
        self.it.peek().map(|s| *s)
    }
}

macro_rules! deserialize_int {
    ($method:ident, $visit:ident) => {
        fn $method<V: Visitor>(
            mut self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.$visit(
                self.next_field()?
                    .parse().map_err(|err| self.error(DEK::ParseInt(err)))?)
        }
    }
}

impl<'a, 'r: 'a> Deserializer for &'a mut DeStringRecord<'r> {
    type Error = DeserializeError;

    fn deserialize<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(self.error(DEK::Unsupported("deserialize".into())))
    }

    fn deserialize_bool<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_bool(
            self.next_field()?
                .parse().map_err(|err| self.error(DEK::ParseBool(err)))?)
    }

    deserialize_int!(deserialize_u8, visit_u8);
    deserialize_int!(deserialize_u16, visit_u16);
    deserialize_int!(deserialize_u32, visit_u32);
    deserialize_int!(deserialize_u64, visit_u64);
    deserialize_int!(deserialize_i8, visit_i8);
    deserialize_int!(deserialize_i16, visit_i16);
    deserialize_int!(deserialize_i32, visit_i32);
    deserialize_int!(deserialize_i64, visit_i64);

    fn deserialize_f32<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_f32(
            self.next_field()?
                .parse().map_err(|err| self.error(DEK::ParseFloat(err)))?)
    }

    fn deserialize_f64<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_f64(
            self.next_field()?
                .parse().map_err(|err| self.error(DEK::ParseFloat(err)))?)
    }

    fn deserialize_char<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let field = self.next_field()?;
        let len = field.chars().count();
        if len != 1 {
            return Err(self.message(format!(
                "expected single character but got {} characters in '{}'",
                len, field)));
        }
        visitor.visit_char(field.chars().next().unwrap())
    }

    fn deserialize_str<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.next_field().and_then(|f| visitor.visit_str(f))
    }

    fn deserialize_string<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.next_field().and_then(|f| visitor.visit_str(f.into()))
    }

    fn deserialize_bytes<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.next_field().and_then(|f| visitor.visit_bytes(f.as_bytes()))
    }

    fn deserialize_byte_buf<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.next_field()
            .and_then(|f| visitor.visit_byte_buf(f.as_bytes().to_vec()))
    }

    fn deserialize_option<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if self.peek_field().map_or(true, |f| f.is_empty()) {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }

    fn deserialize_seq_fixed_size<V: Visitor>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }

    fn deserialize_tuple<V: Visitor>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }

    fn deserialize_tuple_struct<V: Visitor>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }

    fn deserialize_map<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }

    fn deserialize_struct<V: Visitor>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }

    fn deserialize_struct_field<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(self.error(DEK::Unsupported("deserialize_struct_field".into())))
    }

    fn deserialize_enum<V: Visitor>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }

    fn deserialize_ignored_any<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(self.error(DEK::Unsupported("deserialize_ignored_any".into())))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeserializeError {
    pos: Option<Position>,
    field: Option<u64>,
    kind: DeserializeErrorKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeserializeErrorKind {
    Message(String),
    Unsupported(String),
    UnexpectedEndOfRow,
    ParseBool(str::ParseBoolError),
    ParseInt(num::ParseIntError),
    ParseFloat(num::ParseFloatError),
}

impl SerdeError for DeserializeError {
    fn custom<T: fmt::Display>(msg: T) -> DeserializeError {
        DeserializeError {
            pos: None,
            field: None,
            kind: DeserializeErrorKind::Message(msg.to_string()),
        }
    }
}

impl StdError for DeserializeError {
    fn description(&self) -> &str {
        self.kind.description()
    }
}

impl fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (&self.pos, &self.field) {
            (&None, &None) => {
                write!(f, "CSV deserialize error: {}", self.kind)
            }
            (&None, &Some(ref field)) => {
                write!(
                    f,
                    "CSV deserialize error: field {}: {}",
                    field, self.kind)
            }
            (&Some(ref pos), &None) => {
                write!(
                    f,
                    "CSV deserialize error: record {} \
                     (byte {}, line {}): {}",
                    pos.record(), pos.byte(), pos.line(), self.kind)
            }
            (&Some(ref pos), &Some(ref field)) => {
                write!(
                    f,
                    "CSV deserialize error: record {} \
                     (byte {}, line {}, field: {}): {}",
                    pos.record(), pos.byte(), pos.line(), field, self.kind)
            }
        }
    }
}

impl fmt::Display for DeserializeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::DeserializeErrorKind::*;

        match *self {
            Message(ref msg) => write!(f, "{}", msg),
            Unsupported(ref which) => {
                write!(f, "unsupported deserializer method: {}", which)
            }
            UnexpectedEndOfRow => write!(f, "{}", self.description()),
            ParseBool(ref err) => write!(f, "{}", err),
            ParseInt(ref err) => write!(f, "{}", err),
            ParseFloat(ref err) => write!(f, "{}", err),
        }
    }
}

impl DeserializeError {
    /// Return the position of this error, if available.
    pub fn position(&self) -> Option<&Position> {
        self.pos.as_ref()
    }

    /// Return the field index (starting at 0) of this error, if available.
    pub fn field(&self) -> Option<u64> {
        self.field
    }

    /// Return the underlying error kind.
    pub fn kind(&self) -> &DeserializeErrorKind {
        &self.kind
    }
}

impl DeserializeErrorKind {
    fn description(&self) -> &str {
        use self::DeserializeErrorKind::*;

        match *self {
            Message(_) => "deserialization error",
            Unsupported(_) => "unsupported deserializer method",
            UnexpectedEndOfRow => "expected field, but got end of row",
            ParseBool(ref err) => err.description(),
            ParseInt(ref err) => err.description(),
            ParseFloat(ref err) => err.description(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use string_record::StringRecord;
    use super::DeStringRecord;

    #[test]
    fn scratch() {
        #[derive(Serialize, Deserialize, Debug)]
        struct Foo(i32);

        let rec = StringRecord::from(vec!["42"]);
        let mut drec = DeStringRecord::new(&rec, None, None);

        let z = Foo::deserialize(&mut drec).unwrap();
        println!("{:?}", z);
    }
}
