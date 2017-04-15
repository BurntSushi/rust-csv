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

use error::Error;
use writer::Writer;

/// Serialize the given value to the given writer, and return an error if
/// anything went wrong.
///
/// If `headers` is true, then the serializer will attempt to writer a header
/// row. If it did write a header row, then `true` is returned. In all other
/// cases, `false` is returned.
pub fn serialize<S: Serialize, W: io::Write>(
    wtr: &mut Writer<W>,
    value: S,
    headers: bool,
) -> Result<bool, Error> {
    let mut ser = SeRecord {
        wtr: wtr,
        header_only: headers,
        did_headers: false,
    };
    value.serialize(&mut ser).map(|_| ser.did_headers)
}

struct SeRecord<'w, W: 'w + io::Write> {
    wtr: &'w mut Writer<W>,
    header_only: bool,
    did_headers: bool,
}

impl<'a, 'w, W: io::Write> Serializer for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        if v {
            self.wtr.write_field("true")
        } else {
            self.wtr.write_field("false")
        }
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(v.to_string().as_bytes())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(v.to_string().as_bytes())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(v.to_string().as_bytes())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(v.to_string().as_bytes())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(v.to_string().as_bytes())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(v.to_string().as_bytes())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(v.to_string().as_bytes())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(v.to_string().as_bytes())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(v.to_string().as_bytes())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(v.to_string().as_bytes())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(v.to_string().as_bytes())
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(value)
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(value)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(&[])
    }

    fn serialize_some<T: ?Sized + Serialize>(
        self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        None::<()>.serialize(self)
    }

    fn serialize_unit_struct(
        self,
        name: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(name)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: usize,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.wtr.write_field(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        variant_index: usize,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_seq(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }

    fn serialize_seq_fixed_size(
        self,
        size: usize,
    ) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple(
        self,
        len: usize,
    ) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: usize,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::custom("serializing enum tuple variants is not supported"))
    }

    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeMap, Self::Error> {
        // The right behavior for serializing maps isn't clear.
        Err(Error::custom(
            "serializing maps is not supported, \
             if you have a use case, please file an issue at \
             https://github.com/BurntSushi/rust-csv"))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: usize,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::custom("serializing enum struct variants is not supported"))
    }
}

impl<'a, 'w, W: io::Write> SerializeSeq for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, 'w, W: io::Write> SerializeTuple for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, 'w, W: io::Write> SerializeTupleStruct for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, 'w, W: io::Write> SerializeTupleVariant for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unreachable!()
    }
}

impl<'a, 'w, W: io::Write> SerializeMap for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = Error;

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
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        if self.header_only {
            self.did_headers = true;
            key.serialize(&mut **self)
        } else {
            value.serialize(&mut **self)
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, 'w, W: io::Write> SerializeStructVariant for &'a mut SeRecord<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unreachable!()
    }
}

impl SerdeError for Error {
    fn custom<T: fmt::Display>(msg: T) -> Error {
        Error::Serialize(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde::bytes::Bytes;

    use error::Error;
    use writer::Writer;

    use super::SeRecord;

    fn serialize<S: Serialize>(s: S) -> String {
        let mut wtr = Writer::from_writer(vec![]);
        {
            let mut ser = SeRecord {
                wtr: &mut wtr,
                header_only: false,
                did_headers: false,
            };
            s.serialize(&mut ser).unwrap();
            assert!(!ser.did_headers);
        }
        wtr.write_record(None::<&[u8]>).unwrap();
        String::from_utf8(wtr.into_inner().unwrap()).unwrap()
    }

    fn serialize_header<S: Serialize>(s: S) -> String {
        let mut wtr = Writer::from_writer(vec![]);
        {
            let mut ser = SeRecord {
                wtr: &mut wtr,
                header_only: true,
                did_headers: false,
            };
            s.serialize(&mut ser).unwrap();
            assert!(ser.did_headers);
        }
        wtr.write_record(None::<&[u8]>).unwrap();
        String::from_utf8(wtr.into_inner().unwrap()).unwrap()
    }

    fn serialize_err<S: Serialize>(s: S) -> Error {
        let mut wtr = Writer::from_writer(vec![]);
        let mut ser = SeRecord {
            wtr: &mut wtr,
            header_only: false,
            did_headers: false,
        };
        s.serialize(&mut ser).unwrap_err()
    }

    #[test]
    fn bool() {
        let got = serialize(true);
        assert_eq!(got, "true\n");
    }

    #[test]
    fn integer() {
        let got = serialize(12345);
        assert_eq!(got, "12345\n");
    }

    #[test]
    fn float() {
        let got = serialize(1.23);
        assert_eq!(got, "1.23\n");
    }

    #[test]
    fn char() {
        let got = serialize('☃');
        assert_eq!(got, "☃\n");
    }

    #[test]
    fn str() {
        let got = serialize("how\nare\n\"you\"?");
        assert_eq!(got, "\"how\nare\n\"\"you\"\"?\"\n");
    }

    #[test]
    fn bytes() {
        let got = serialize(Bytes::new(&b"how\nare\n\"you\"?"[..]));
        assert_eq!(got, "\"how\nare\n\"\"you\"\"?\"\n");
    }

    #[test]
    fn option() {
        let got = serialize(None::<()>);
        assert_eq!(got, "\"\"\n");

        let got = serialize(Some(5));
        assert_eq!(got, "5\n");
    }

    #[test]
    fn unit() {
        let got = serialize(());
        assert_eq!(got, "\"\"\n");
    }

    #[test]
    fn struct_unit() {
        #[derive(Serialize)]
        struct Foo;

        let got = serialize(Foo);
        assert_eq!(got, "Foo\n");
    }

    #[test]
    fn struct_newtype() {
        #[derive(Serialize)]
        struct Foo(f64);

        let got = serialize(Foo(1.5));
        assert_eq!(got, "1.5\n");
    }

    #[test]
    fn enum_units() {
        #[derive(Serialize)]
        enum Wat { Foo, Bar, Baz }

        let got = serialize(Wat::Foo);
        assert_eq!(got, "Foo\n");

        let got = serialize(Wat::Bar);
        assert_eq!(got, "Bar\n");

        let got = serialize(Wat::Baz);
        assert_eq!(got, "Baz\n");
    }

    #[test]
    fn enum_newtypes() {
        #[derive(Serialize)]
        enum Wat { Foo(i32), Bar(f32), Baz(bool) }

        let got = serialize(Wat::Foo(5));
        assert_eq!(got, "5\n");

        let got = serialize(Wat::Bar(1.5));
        assert_eq!(got, "1.5\n");

        let got = serialize(Wat::Baz(true));
        assert_eq!(got, "true\n");
    }

    #[test]
    fn seq() {
        let got = serialize(vec![1, 2, 3]);
        assert_eq!(got, "1,2,3\n");
    }

    #[test]
    fn tuple() {
        let got = serialize((true, 1.5, "hi"));
        assert_eq!(got, "true,1.5,hi\n");

        let got = serialize((true, 1.5, vec![1, 2, 3]));
        assert_eq!(got, "true,1.5,1,2,3\n");
    }

    #[test]
    fn tuple_struct() {
        #[derive(Serialize)]
        struct Foo(bool, i32, String);

        let got = serialize(Foo(false, 42, "hi".to_string()));
        assert_eq!(got, "false,42,hi\n");
    }

    #[test]
    fn tuple_variant() {
        #[derive(Serialize)]
        enum Foo {
            X(bool, i32, String),
        }

        let err = serialize_err(Foo::X(false, 42, "hi".to_string()));
        match err {
            Error::Serialize(_) => {}
            x => panic!("expected Error::Serialize but got '{:?}'", x),
        }
    }

    #[test]
    fn enum_struct_variant() {
        #[derive(Serialize)]
        enum Foo {
            X { a: bool, b: i32, c: String },
        }

        let err = serialize_err(Foo::X { a: false, b: 1, c: "hi".into() });
        match err {
            Error::Serialize(_) => {}
            x => panic!("expected Error::Serialize but got '{:?}'", x),
        }
    }

    #[test]
    fn struct_no_headers() {
        #[derive(Serialize)]
        struct Foo {
            x: bool,
            y: i32,
            z: String,
        }

        let got = serialize(Foo { x: true, y: 5, z: "hi".into() });
        assert_eq!(got, "true,5,hi\n");
    }

    #[test]
    fn struct_headers() {
        #[derive(Serialize)]
        struct Foo {
            x: bool,
            y: i32,
            z: String,
        }

        let got = serialize_header(Foo { x: true, y: 5, z: "hi".into() });
        assert_eq!(got, "x,y,z\n");
        let got = serialize(Foo { x: true, y: 5, z: "hi".into() });
        assert_eq!(got, "true,5,hi\n");
    }
}
