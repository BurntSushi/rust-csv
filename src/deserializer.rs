use std::error::{Error as StdError};
use std::fmt;
use std::iter;
use std::num;
use std::str;

use serde::de::{
    Deserializer, DeserializeSeed,
    Error as SerdeError, Unexpected,
    Visitor, EnumVisitor, VariantVisitor, MapVisitor, SeqVisitor,
};
use serde::de::value::ValueDeserializer;

use reader::Position;
use string_record::{StringRecord, StringRecordIter};

use self::{DeserializeErrorKind as DEK};

pub struct DeStringRecord<'r> {
    it: iter::Peekable<StringRecordIter<'r>>,
    headers: Option<StringRecordIter<'r>>,
    field: u64,
}

impl<'r> DeStringRecord<'r> {
    pub fn new(
        rec: &'r StringRecord,
        headers: Option<&'r StringRecord>,
    ) -> DeStringRecord<'r> {
        DeStringRecord {
            it: rec.iter().peekable(),
            headers: headers.map(|r| r.iter()),
            field: 0,
        }
    }

    /// Returns an error corresponding to the most recently extracted field.
    fn error(&self, kind: DeserializeErrorKind) -> DeserializeError {
        DeserializeError {
            field: Some(self.field.saturating_sub(1)),
            kind: kind,
        }
    }

    /// Returns an arbitrary catch-all error for the most recently extracted
    /// field.
    fn message(&self, msg: String) -> DeserializeError {
        self.error(DEK::Message(msg))
    }

    /// Extracts the next field from the underlying record.
    #[inline(always)]
    fn next_field(&mut self) -> Result<&'r str, DeserializeError> {
        match self.it.next() {
            Some(field) => {
                self.field += 1;
                Ok(field)
            }
            None => Err(DeserializeError {
                field: None,
                kind: DEK::UnexpectedEndOfRow,
            })
        }
    }

    /// Extracts the next header value from the underlying record.
    fn next_header(&mut self) -> Result<&'r str, DeserializeError> {
        match self.headers.as_mut().and_then(|it| it.next()) {
            Some(field) => Ok(field),
            None => Err(DeserializeError {
                field: None,
                kind: DEK::UnexpectedEndOfRow,
            }),
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
        let x = self.next_field()?;
        if x == "true" {
            visitor.visit_bool(true)
        } else if x == "false" {
            visitor.visit_bool(false)
        } else if is_positive_integer(x.as_bytes()) {
            let n: u64 = x.parse().map_err(|e| self.error(DEK::ParseInt(e)))?;
            visitor.visit_u64(n)
        } else if is_negative_integer(x.as_bytes()) {
            let n: i64 = x.parse().map_err(|e| self.error(DEK::ParseInt(e)))?;
            visitor.visit_i64(n)
        } else if let Some(n) = try_float(x) {
            visitor.visit_f64(n)
        } else {
            visitor.visit_str(x)
        }
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
        match self.peek_field() {
            None => visitor.visit_none(),
            Some(f) if f.is_empty() => {
                self.next_field().expect("empty field");
                visitor.visit_none()
            }
            Some(_) => visitor.visit_some(self),
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
        visitor.visit_seq(self)
    }

    fn deserialize_seq_fixed_size<V: Visitor>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(self)
    }

    fn deserialize_tuple<V: Visitor>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(self)
    }

    fn deserialize_tuple_struct<V: Visitor>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(self)
    }

    fn deserialize_map<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if self.headers.is_none() {
            visitor.visit_seq(self)
        } else {
            visitor.visit_map(self)
        }
    }

    fn deserialize_struct<V: Visitor>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if self.headers.is_none() {
            visitor.visit_seq(self)
        } else {
            visitor.visit_map(self)
        }
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
        visitor.visit_enum(self)
    }

    fn deserialize_ignored_any<V: Visitor>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        // Read and drop the next field.
        // This code is reached, e.g., when trying to deserialize a header
        // that doesn't exist in the destination struct.
        let _ = self.next_field()?;
        visitor.visit_unit()
    }
}

impl<'a, 'r: 'a> EnumVisitor for &'a mut DeStringRecord<'r> {
    type Error = DeserializeError;
    type Variant = Self;

    fn visit_variant_seed<V: DeserializeSeed>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let variant_name = self.next_field()?;
        seed.deserialize(variant_name.into_deserializer()).map(|v| (v, self))
    }
}

impl<'a, 'r: 'a> VariantVisitor for &'a mut DeStringRecord<'r> {
    type Error = DeserializeError;

    fn visit_unit(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_newtype_seed<T: DeserializeSeed>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        let unexp = Unexpected::UnitVariant;
        Err(DeserializeError::invalid_type(unexp, &"newtype variant"))
    }

    fn visit_tuple<V: Visitor>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let unexp = Unexpected::UnitVariant;
        Err(DeserializeError::invalid_type(unexp, &"tuple variant"))
    }

    fn visit_struct<V: Visitor>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let unexp = Unexpected::UnitVariant;
        Err(DeserializeError::invalid_type(unexp, &"struct variant"))
    }
}

impl<'a, 'r: 'a> SeqVisitor for &'a mut DeStringRecord<'r> {
    type Error = DeserializeError;

    fn visit_seed<T: DeserializeSeed>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        if self.peek_field().is_none() {
            Ok(None)
        } else {
            seed.deserialize(&mut **self).map(Some)
        }
    }
}

impl<'a, 'r: 'a> MapVisitor for &'a mut DeStringRecord<'r> {
    type Error = DeserializeError;

    fn visit_key_seed<K: DeserializeSeed>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        let mut it = self.headers.as_mut().expect("headers");
        let field = match it.next() {
            None => return Ok(None),
            Some(field) => field,
        };
        seed.deserialize(field.into_deserializer()).map(Some)
    }

    fn visit_value_seed<K: DeserializeSeed>(
        &mut self,
        seed: K,
    ) -> Result<K::Value, Self::Error> {
        seed.deserialize(&mut **self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeserializeError {
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
        if let Some(field) = self.field {
            write!(f, "field {}: {}", field, self.kind)
        } else {
            write!(f, "{}", self.kind)
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

fn is_positive_integer(bs: &[u8]) -> bool {
    bs.iter().all(|&b| b'0' <= b && b <= b'9')
}

fn is_negative_integer(bs: &[u8]) -> bool {
    !bs.is_empty() && bs[0] == b'-' && is_positive_integer(&bs[1..])
}

fn try_float(s: &str) -> Option<f64> {
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::{Deserialize, Deserializer};
    use serde::bytes::ByteBuf;

    use string_record::StringRecord;
    use super::{DeStringRecord, DeserializeError};

    fn sr(fields: &[&str]) -> StringRecord {
        StringRecord::from(fields)
    }

    fn de<D: Deserialize>(fields: &[&str]) -> Result<D, DeserializeError> {
        let fields = StringRecord::from(fields);
        let mut deser = DeStringRecord::new(&fields, None);
        D::deserialize(&mut deser)
    }

    fn de_headers<D: Deserialize>(
        headers: &[&str],
        fields: &[&str],
    ) -> Result<D, DeserializeError> {
        let headers = StringRecord::from(headers);
        let fields = StringRecord::from(fields);
        let mut deser = DeStringRecord::new(&fields, Some(&headers));
        D::deserialize(&mut deser)
    }

    #[test]
    fn with_header() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Foo {
            z: f64,
            y: i32,
            x: String,
        }

        let got: Foo = de_headers(
            &["x", "y", "z"],
            &["hi", "42", "1.3"],
        ).unwrap();
        assert_eq!(got, Foo { x: "hi".into(), y: 42, z: 1.3 });
    }

    #[test]
    fn with_header_unknown() {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(deny_unknown_fields)]
        struct Foo {
            z: f64,
            y: i32,
            x: String,
        }
        assert!(de_headers::<Foo>(
            &["a", "x", "y", "z"],
            &["foo", "hi", "42", "1.3"],
        ).is_err());
    }

    #[test]
    fn with_header_missing() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Foo {
            z: f64,
            y: i32,
            x: String,
        }
        assert!(de_headers::<Foo>(
            &["y", "z"],
            &["42", "1.3"],
        ).is_err());
    }

    #[test]
    fn with_header_missing_ok() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Foo {
            z: f64,
            y: i32,
            x: Option<String>,
        }

        let got: Foo = de_headers(
            &["y", "z"],
            &["42", "1.3"],
        ).unwrap();
        assert_eq!(got, Foo { x: None, y: 42, z: 1.3 });
    }

    #[test]
    fn with_header_no_fields() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Foo {
            z: f64,
            y: i32,
            x: Option<String>,
        }

        let got = de_headers::<Foo>(&["y", "z"], &[]);
        assert!(got.is_err());
    }

    #[test]
    fn with_header_empty() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Foo {
            z: f64,
            y: i32,
            x: Option<String>,
        }

        let got = de_headers::<Foo>(&[], &[]);
        assert!(got.is_err());
    }

    #[test]
    fn with_header_empty_ok() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Foo;

        #[derive(Deserialize, Debug, PartialEq)]
        struct Bar {};

        let got = de_headers::<Foo>(&[], &[]);
        assert_eq!(got.unwrap(), Foo);

        let got = de_headers::<Bar>(&[], &[]);
        assert_eq!(got.unwrap(), Bar{});

        let got = de_headers::<()>(&[], &[]);
        assert_eq!(got.unwrap(), ());
    }

    #[test]
    fn without_header() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Foo {
            z: f64,
            y: i32,
            x: String,
        }

        let got: Foo = de(&["1.3", "42", "hi"]).unwrap();
        assert_eq!(got, Foo { x: "hi".into(), y: 42, z: 1.3 });
    }

    #[test]
    fn no_fields() {
        assert!(de::<String>(&[]).is_err());
    }

    #[test]
    fn one_field() {
        let got: i32 = de(&["42"]).unwrap();
        assert_eq!(got, 42);
    }

    #[test]
    fn two_fields() {
        let got: (i32, bool) = de(&["42", "true"]).unwrap();
        assert_eq!(got, (42, true));

        #[derive(Deserialize, Debug, PartialEq)]
        struct Foo(i32, bool);

        let got: Foo = de(&["42", "true"]).unwrap();
        assert_eq!(got, Foo(42, true));
    }

    #[test]
    fn two_fields_too_many() {
        let got: (i32, bool) = de(&["42", "true", "z", "z"]).unwrap();
        assert_eq!(got, (42, true));
    }

    #[test]
    fn two_fields_too_few() {
        assert!(de::<(i32, bool)>(&["42"]).is_err());
    }

    #[test]
    fn one_char() {
        let got: char = de(&["a"]).unwrap();
        assert_eq!(got, 'a');
    }

    #[test]
    fn no_chars() {
        assert!(de::<char>(&[""]).is_err());
    }

    #[test]
    fn too_many_chars() {
        assert!(de::<char>(&["ab"]).is_err());
    }

    #[test]
    fn simple_seq() {
        let got: Vec<i32> = de(&["1", "5", "10"]).unwrap();
        assert_eq!(got, vec![1, 5, 10]);
    }

    #[test]
    fn seq_in_struct() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Foo {
            xs: Vec<i32>,
        }
        let got: Foo = de(&["1", "5", "10"]).unwrap();
        assert_eq!(got, Foo { xs: vec![1, 5, 10] });
    }

    #[test]
    fn seq_in_struct_tail() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Foo {
            label: String,
            xs: Vec<i32>,
        }
        let got: Foo = de(&["foo", "1", "5", "10"]).unwrap();
        assert_eq!(got, Foo { label: "foo".into(), xs: vec![1, 5, 10] });
    }

    #[test]
    fn map_headers() {
        let got: HashMap<String, i32> =
            de_headers(&["a", "b", "c"], &["1", "5", "10"]).unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got["a"], 1);
        assert_eq!(got["b"], 5);
        assert_eq!(got["c"], 10);
    }

    #[test]
    fn map_no_headers() {
        let got = de::<HashMap<String, i32>>(&["1", "5", "10"]);
        assert!(got.is_err());
    }

    #[test]
    fn bytes() {
        let got: Vec<u8> = de::<ByteBuf>(&["foobar"]).unwrap().into();
        assert_eq!(got, b"foobar".to_vec());
    }

    #[test]
    fn adjacent_fixed_arrays() {
        let got: ([u32; 2], [u32; 2]) = de(&["1", "5", "10", "15"]).unwrap();
        assert_eq!(got, ([1, 5], [10, 15]));
    }

    #[test]
    fn enum_label_simple_tagged() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Row {
            label: Label,
            x: f64,
        }

        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "snake_case")]
        enum Label {
            Foo,
            Bar,
            Baz,
        }

        let got: Row = de_headers(&["label", "x"], &["bar", "5"]).unwrap();
        assert_eq!(got, Row { label: Label::Bar, x: 5.0 });
    }

    #[test]
    fn enum_untagged() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Row {
            x: Boolish,
            y: Boolish,
            z: Boolish,
        }

        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "snake_case")]
        #[serde(untagged)]
        enum Boolish {
            Bool(bool),
            Number(i64),
            String(String),
        }

        let got: Row = de_headers(
            &["x", "y", "z"],
            &["true", "null", "1"],
        ).unwrap();
        assert_eq!(got, Row {
            x: Boolish::Bool(true),
            y: Boolish::String("null".into()),
            z: Boolish::Number(1),
        });
    }

    #[test]
    fn option_empty_field() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Foo {
            a: Option<i32>,
            b: String,
            c: Option<i32>,
        }

        let got: Foo = de_headers(
            &["a", "b", "c"],
            &["", "foo", "5"],
        ).unwrap();
        assert_eq!(got, Foo { a: None, b: "foo".into(), c: Some(5) });
    }
}
