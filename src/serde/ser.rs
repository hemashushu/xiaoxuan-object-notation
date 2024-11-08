// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::io::Write;

use super::Result;
use crate::error::Error;

use serde::{ser, Serialize};

const DEFAULT_INDEXT_CHARS: &str = "    ";

pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut buf: Vec<u8> = vec![];
    to_writer(value, &mut buf)?;
    let s = String::from_utf8(buf).unwrap();
    Ok(s)
}

pub fn to_writer<T, W: Write>(value: &T, writer: &mut W) -> Result<()>
where
    T: Serialize,
{
    let mut serializer = Serializer::new(DEFAULT_INDEXT_CHARS, writer);
    value.serialize(&mut serializer)
    // Ok(serializer.buffer.join(""))
}

pub struct Serializer<'a, W>
where
    W: Write,
{
    writer: &'a mut W,
    indent_level: usize,
    indent_chars: String,

    is_first_element: bool,
}

impl<'a, W> Serializer<'a, W>
where
    W: Write,
{
    fn new(indent_chars: &str, writer: &'a mut W) -> Self {
        Self {
            writer,
            indent_level: 0,
            indent_chars: indent_chars.to_owned(),
            is_first_element: false,
        }
    }

    // append the text content
    fn append(&mut self, s: String) -> Result<()> {
        match write!(self.writer, "{}", s) {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::Message(e.to_string())),
        }
    }

    // append the leading whitespaces
    fn append_indent(&mut self) -> Result<()> {
        let s = self.indent_chars.repeat(self.indent_level);
        self.append(s)
    }

    fn increase_level(&mut self) {
        self.indent_level += 1;
    }

    fn decrease_level(&mut self) {
        self.indent_level -= 1;
    }
}

impl<'a, 'b, W> ser::Serializer for &'a mut Serializer<'b, W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        match v {
            true => self.append("true".to_owned()),
            false => self.append("false".to_owned()),
        }
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.append(format!("{}_i8", v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.append(format!("{}_i16", v))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        // 'i32' is the default type for integer numbers,
        // so no explicit type name is needed.
        self.append(format!("{}", v))
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.append(format!("{}_i64", v))
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.append(format!("{}_u8", v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.append(format!("{}_u16", v))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.append(format!("{}_u32", v))
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.append(format!("{}_u64", v))
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        let s = if v.is_nan() {
            "NaN_f32".to_owned()
        } else if v == f32::INFINITY {
            "Inf_f32".to_owned()
        } else if v == f32::NEG_INFINITY {
            "-Inf_f32".to_owned()
        } else {
            format!("{}_f32", v)
        };

        self.append(s)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        // 'f64' is the default type for floating-point numbers.
        // so there is no need for an explicit type name

        let s = if v.is_nan() {
            "NaN".to_owned()
        } else if v == f64::INFINITY {
            "Inf".to_owned()
        } else if v == f64::NEG_INFINITY {
            "-Inf".to_owned()
        } else {
            // a decimal point needs to be appended if there is no decimal point
            // in the literal.
            let mut s = v.to_string();
            if !s.contains('.') {
                s.push_str(".0");
            }
            s
        };
        self.append(s)
    }

    fn serialize_char(self, v: char) -> Result<()> {
        let s = match v {
            '\\' => "\\\\".to_owned(),
            '\'' => "\\'".to_owned(),
            '\t' => {
                // horizontal tabulation
                "\\t".to_owned()
            }
            '\r' => {
                // carriage return, jump to the beginning of the line (CR)
                "\\r".to_owned()
            }
            '\n' => {
                // new line/line feed (LF)
                "\\n".to_owned()
            }
            '\0' => {
                // null char
                "\\0".to_owned()
            }
            _ => v.to_string(),
        };

        self.append(format!("'{}'", s))
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        let s = format!(
            "\"{}\"",
            v.chars()
                .map(|c| match c {
                    '\\' => "\\\\".to_owned(),
                    '"' => "\\\"".to_owned(),
                    // null char is allowed in the ASON string, it is used for represent the string resource.
                    '\0' => "\\0".to_owned(),
                    // some text editors automatically remove the tab when
                    // it is at the end of a line, so it is best to escape the tab char.
                    // therefor it should be escaped
                    '\t' => "\\t".to_owned(),
                    _ => c.to_string(),
                })
                .collect::<Vec<String>>()
                .join("")
        );
        self.append(s)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        // [u8]
        // Similar to strings, during deserialization byte arrays can be transient,
        // owned, or borrowed.
        let s = format!(
            "h\"{}\"",
            v.iter()
                .map(|item| format!("{:02x}", item))
                .collect::<Vec<String>>()
                .join(" ")
        );
        self.append(s)
    }

    fn serialize_none(self) -> Result<()> {
        self.append("Option::None".to_owned())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.append("Option::Some(".to_owned())?;
        value.serialize(&mut *self)?;
        self.append(")".to_owned())
    }

    fn serialize_unit(self) -> Result<()> {
        // The type of `()` in Rust.
        // It represents an anonymous value containing no data.
        Err(Error::Message("Does not support Unit.".to_owned()))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        // For example `struct Unit` or `PhantomData<T>`.
        // It represents a named value containing no data.
        Err(Error::Message(
            "Does not support \"Unit\" style Struct.".to_owned(),
        ))
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        // For example the` E::A` and `E::B` in `enum E { A, B }`.
        self.append(format!("{}::{}", name, variant))
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // For example `struct Millimeters(u8)`.
        Err(Error::Message(
            "Does not support \"New-Type\" style Struct.".to_owned(),
        ))
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // For example the `E::N` in `enum E { N(u8) }`.
        self.append(format!("{}::{}(", name, variant))?;
        value.serialize(&mut *self)?;
        self.append(")".to_owned())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.append("[".to_owned())?;
        self.is_first_element = true;
        self.increase_level();
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        // Note that the Rust fixed length arrays
        // will be treated as tuples, e.g.
        // [i32; 4]
        //
        // per: https://serde.rs/data-model.html

        self.append("(".to_owned())?;
        self.is_first_element = true;
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        // A named tuple, for example `struct Rgb(u8, u8, u8)`.
        Err(Error::Message(
            "Does not support \"Tuple\" style Struct.".to_owned(),
        ))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        // For example the `E::T` in `enum E { T(u8, u8) }`.

        self.append(format!("{}::{}", name, variant))?;
        self.append("(".to_owned())?;
        self.is_first_element = true;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        // A variably sized heterogeneous key-value pairing,
        // for example `BTreeMap<K, V>`.
        // When serializing, the length may or may not be known before
        // iterating through all the entries. When deserializing,
        // the length is determined by looking at the serialized data.

        self.append("{".to_owned())?;
        self.is_first_element = true;
        self.increase_level();
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.append("{".to_owned())?;
        self.is_first_element = true;
        self.increase_level();
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        // For example the `E::S` in `enum E { S { r: u8, g: u8, b: u8 } }`.

        self.append(format!("{}::{}", name, variant))?;
        self.append("{".to_owned())?;
        self.is_first_element = true;
        self.increase_level();
        Ok(self)
    }
}

impl<'a, 'b, W> ser::SerializeSeq for &'a mut Serializer<'b, W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.is_first_element = false;

        self.append("\n".to_owned())?;
        self.append_indent()?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.decrease_level();
        self.append("\n".to_owned())?;
        self.append_indent()?;
        self.append("]".to_owned())
    }
}

impl<'a, 'b, W> ser::SerializeTuple for &'a mut Serializer<'b, W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.is_first_element {
            self.is_first_element = false;
        } else {
            self.append(", ".to_owned())?;
        }

        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.append(")".to_owned())
    }
}

impl<'a, 'b, W> ser::SerializeTupleStruct for &'a mut Serializer<'b, W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        unreachable!()
    }

    fn end(self) -> Result<()> {
        unreachable!()
    }
}

impl<'a, 'b, W> ser::SerializeTupleVariant for &'a mut Serializer<'b, W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.is_first_element {
            self.is_first_element = false;
        } else {
            self.append(", ".to_owned())?;
        }

        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.append(")".to_owned())
    }
}

impl<'a, 'b, W> ser::SerializeMap for &'a mut Serializer<'b, W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.is_first_element = false;

        self.append("\n".to_owned())?;
        self.append_indent()?;
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.append(": ".to_owned())?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.decrease_level();
        self.append("\n".to_owned())?;
        self.append_indent()?;
        self.append("}".to_owned())
    }
}

impl<'a, 'b, W> ser::SerializeStruct for &'a mut Serializer<'b, W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.is_first_element = false;

        self.append("\n".to_owned())?;
        self.append_indent()?;
        self.append(format!("{}: ", key))?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.decrease_level();
        self.append("\n".to_owned())?;
        self.append_indent()?;
        self.append("}".to_owned())
    }
}

impl<'a, 'b, W> ser::SerializeStructVariant for &'a mut Serializer<'b, W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.is_first_element = false;

        self.append("\n".to_owned())?;
        self.append_indent()?;
        self.append(format!("{}: ", key))?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.decrease_level();
        self.append("\n".to_owned())?;
        self.append_indent()?;
        self.append("}".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use pretty_assertions::assert_eq;
    use serde::Serialize;
    use serde_bytes::ByteBuf;

    use crate::serde::ser::to_string;

    #[test]
    fn test_primitive_types() {
        // bool
        {
            let v0: bool = false;
            assert_eq!(to_string(&v0).unwrap(), r#"false"#);

            let v1: bool = true;
            assert_eq!(to_string(&v1).unwrap(), r#"true"#);
        }

        // signed integers
        {
            let v0: i8 = 11;
            assert_eq!(to_string(&v0).unwrap(), r#"11_i8"#);

            let v1: i16 = 13;
            assert_eq!(to_string(&v1).unwrap(), r#"13_i16"#);

            let v2: i32 = 17;
            assert_eq!(to_string(&v2).unwrap(), r#"17"#);

            let v3: i64 = 19;
            assert_eq!(to_string(&v3).unwrap(), r#"19_i64"#);
        }

        // unsigned integers
        {
            let v0: u8 = 11;
            assert_eq!(to_string(&v0).unwrap(), r#"11_u8"#);

            let v1: u16 = 13;
            assert_eq!(to_string(&v1).unwrap(), r#"13_u16"#);

            let v2: u32 = 17;
            assert_eq!(to_string(&v2).unwrap(), r#"17_u32"#);

            let v3: u64 = 19;
            assert_eq!(to_string(&v3).unwrap(), r#"19_u64"#);
        }

        // floating-point f32
        {
            let v0: f32 = 123_f32;
            assert_eq!(to_string(&v0).unwrap(), r#"123_f32"#);

            let v1: f32 = -4.56_f32;
            assert_eq!(to_string(&v1).unwrap(), r#"-4.56_f32"#);

            let v2: f32 = std::f32::consts::PI;
            assert_eq!(to_string(&v2).unwrap(), r#"3.1415927_f32"#);

            let v3: f32 = 0f32;
            assert_eq!(to_string(&v3).unwrap(), r#"0_f32"#);

            let v4: f32 = -0f32;
            assert_eq!(to_string(&v4).unwrap(), r#"-0_f32"#);

            assert_eq!(to_string(&f32::NAN).unwrap(), r#"NaN_f32"#);
            assert_eq!(to_string(&f32::INFINITY).unwrap(), r#"Inf_f32"#);
            assert_eq!(to_string(&f32::NEG_INFINITY).unwrap(), r#"-Inf_f32"#);
        }

        // floating-point f64
        {
            let v0: f64 = 123_f64;
            assert_eq!(to_string(&v0).unwrap(), r#"123.0"#);

            let v1: f64 = -4.56_f64;
            assert_eq!(to_string(&v1).unwrap(), r#"-4.56"#);

            let v2: f64 = std::f64::consts::E;
            assert_eq!(to_string(&v2).unwrap(), r#"2.718281828459045"#);

            let v3: f64 = 0f64;
            assert_eq!(to_string(&v3).unwrap(), r#"0.0"#);

            let v4: f64 = -0f64;
            assert_eq!(to_string(&v4).unwrap(), r#"-0.0"#);

            assert_eq!(to_string(&f64::NAN).unwrap(), r#"NaN"#);
            assert_eq!(to_string(&f64::INFINITY).unwrap(), r#"Inf"#);
            assert_eq!(to_string(&f64::NEG_INFINITY).unwrap(), r#"-Inf"#);
        }

        // char
        {
            assert_eq!(to_string(&'a').unwrap(), r#"'a'"#);
            assert_eq!(to_string(&'文').unwrap(), r#"'文'"#);
            assert_eq!(to_string(&'🍒').unwrap(), r#"'🍒'"#);

            // escaped characters
            assert_eq!(to_string(&'\\').unwrap(), r#"'\\'"#);
            assert_eq!(to_string(&'\'').unwrap(), r#"'\''"#);

            // double quote does not necessary to be escaped
            assert_eq!(to_string(&'"').unwrap(), r#"'"'"#);
            assert_eq!(to_string(&'\"').unwrap(), r#"'"'"#);

            assert_eq!(to_string(&'\t').unwrap(), r#"'\t'"#);
            assert_eq!(to_string(&'\r').unwrap(), r#"'\r'"#);
            assert_eq!(to_string(&'\n').unwrap(), r#"'\n'"#);
            assert_eq!(to_string(&'\0').unwrap(), r#"'\0'"#);
        }

        // string
        {
            assert_eq!(to_string(&"abc文字🍒").unwrap(), r#""abc文字🍒""#);
            assert_eq!(to_string(&"abc\"\\\t\0xyz").unwrap(), r#""abc\"\\\t\0xyz""#);
        }
    }

    #[test]
    fn test_byte_data() {
        let v0 = vec![11u8, 13, 17, 19];
        let v0b = ByteBuf::from(v0);
        assert_eq!(to_string(&v0b).unwrap(), r#"h"0b 0d 11 13""#);

        let v1 = b"abc";
        let v1b = ByteBuf::from(v1);
        assert_eq!(to_string(&v1b).unwrap(), r#"h"61 62 63""#);
    }

    #[test]
    fn test_option() {
        let v0: Option<i32> = None;
        assert_eq!(to_string(&v0).unwrap(), r#"Option::None"#);

        let v1: Option<i32> = Some(123);
        assert_eq!(to_string(&v1).unwrap(), r#"Option::Some(123)"#);
    }

    #[test]
    fn test_list() {
        assert_eq!(
            to_string(&vec![11, 13, 17, 19]).unwrap(),
            r#"[
    11
    13
    17
    19
]"#
        );

        assert_eq!(
            to_string(&"abc".as_bytes()).unwrap(),
            r#"[
    97_u8
    98_u8
    99_u8
]"#
        );

        assert_eq!(
            to_string(&vec!["foo", "bar", "2024"]).unwrap(),
            r#"[
    "foo"
    "bar"
    "2024"
]"#
        );

        assert_eq!(
            to_string(&vec![Some(11), None, Some(13)]).unwrap(),
            r#"[
    Option::Some(11)
    Option::None
    Option::Some(13)
]"#
        );

        // nested seq

        assert_eq!(
            to_string(&vec![vec![11, 13], vec![17, 19], vec![23, 29]]).unwrap(),
            r#"[
    [
        11
        13
    ]
    [
        17
        19
    ]
    [
        23
        29
    ]
]"#
        );
    }

    #[test]
    fn test_tuple() {
        assert_eq!(to_string(&(11, 13, 17, 19)).unwrap(), r#"(11, 13, 17, 19)"#);

        // a fixed-length array is treated as tuple
        assert_eq!(to_string(b"abc").unwrap(), r#"(97_u8, 98_u8, 99_u8)"#);

        assert_eq!(
            to_string(&("foo", "bar", "2024")).unwrap(),
            r#"("foo", "bar", "2024")"#
        );

        assert_eq!(
            to_string(&(Some(11), Option::<i32>::None, Some(13))).unwrap(),
            r#"(Option::Some(11), Option::None, Option::Some(13))"#
        );

        // nested tuple
        assert_eq!(
            to_string(&((11, 13), (17, 19), (23, 29))).unwrap(),
            r#"((11, 13), (17, 19), (23, 29))"#
        );
    }

    #[test]
    fn test_object() {
        #[derive(Serialize)]
        struct Object {
            id: i32,
            name: String,
            checked: bool,
        }

        let v0 = Object {
            id: 123,
            name: "foo".to_owned(),
            checked: true,
        };

        let expected0 = r#"{
    id: 123
    name: "foo"
    checked: true
}"#;
        assert_eq!(to_string(&v0).unwrap(), expected0);

        // nested object
        #[derive(Serialize)]
        struct Address {
            code: i32,
            city: String,
        }

        #[derive(Serialize)]
        struct NestedObject {
            id: i32,
            name: String,
            address: Box<Address>,
        }

        let v1 = NestedObject {
            id: 456,
            name: "bar".to_owned(),
            address: Box::new(Address {
                code: 518000,
                city: "sz".to_owned(),
            }),
        };

        let expected1 = r#"{
    id: 456
    name: "bar"
    address: {
        code: 518000
        city: "sz"
    }
}"#;

        assert_eq!(to_string(&v1).unwrap(), expected1);
    }

    #[test]
    fn test_map() {
        let mut m0 = HashMap::<String, Option<String>>::new();
        m0.insert("foo".to_owned(), Some("hello".to_owned()));
        m0.insert("bar".to_owned(), None);
        m0.insert("baz".to_owned(), Some("world".to_owned()));

        let s0 = to_string(&m0).unwrap();
        assert!(s0.starts_with('{'));
        assert!(s0.ends_with('}'));
        assert!(s0.contains(r#""foo": Option::Some("hello")"#));
        assert!(s0.contains(r#""bar": Option::None"#));
        assert!(s0.contains(r#""baz": Option::Some("world")"#));

        let mut m1 = HashMap::<i32, Option<String>>::new();
        m1.insert(223, Some("hello".to_owned()));
        m1.insert(227, None);
        m1.insert(229, Some("world".to_owned()));

        let s1 = to_string(&m1).unwrap();
        assert!(s1.starts_with('{'));
        assert!(s1.ends_with('}'));
        assert!(s1.contains(r#"223: Option::Some("hello")"#));
        assert!(s1.contains(r#"227: Option::None"#));
        assert!(s1.contains(r#"229: Option::Some("world")"#));
    }

    #[test]
    fn test_variant_with_single_value() {
        #[derive(Serialize)]
        enum Color {
            Red,
            Green,
            Blue,
            Grey(u8),
        }

        {
            let v0 = Color::Red;
            assert_eq!(to_string(&v0).unwrap(), r#"Color::Red"#);

            let v1 = Color::Green;
            assert_eq!(to_string(&v1).unwrap(), r#"Color::Green"#);

            let v2 = Color::Grey(11);
            assert_eq!(to_string(&v2).unwrap(), r#"Color::Grey(11_u8)"#);
        }

        // nested
        #[derive(Serialize)]
        enum Apperance {
            Transparent,
            Color(Color),
        }

        {
            let v0 = Apperance::Transparent;
            assert_eq!(to_string(&v0).unwrap(), r#"Apperance::Transparent"#);

            let v1 = Apperance::Color(Color::Blue);
            assert_eq!(to_string(&v1).unwrap(), r#"Apperance::Color(Color::Blue)"#);

            let v2 = Apperance::Color(Color::Grey(13));
            assert_eq!(
                to_string(&v2).unwrap(),
                r#"Apperance::Color(Color::Grey(13_u8))"#
            );
        }
    }

    #[test]
    fn test_variant_with_multiple_values() {
        #[allow(clippy::upper_case_acronyms)]
        #[derive(Serialize)]
        enum Color {
            RGB(u8, u8, u8),
            Grey(u8),
        }

        assert_eq!(
            to_string(&Color::RGB(255, 127, 63)).unwrap(),
            r#"Color::RGB(255_u8, 127_u8, 63_u8)"#
        );

        assert_eq!(
            to_string(&Color::Grey(127)).unwrap(),
            r#"Color::Grey(127_u8)"#
        );
    }

    #[test]
    fn test_variant_with_object_value() {
        #[derive(Serialize)]
        enum Shape {
            Circle(i32),
            Rect { width: i32, height: i32 },
        }

        assert_eq!(
            to_string(&Shape::Rect {
                width: 200,
                height: 100
            })
            .unwrap(),
            r#"Shape::Rect{
    width: 200
    height: 100
}"#
        );

        assert_eq!(
            to_string(&Shape::Circle(127)).unwrap(),
            r#"Shape::Circle(127)"#
        );
    }

    #[test]
    fn test_mix_list_and_tuple() {
        assert_eq!(
            to_string(&vec![(1, "foo"), (2, "bar")]).unwrap(),
            r#"[
    (1, "foo")
    (2, "bar")
]"#
        );

        assert_eq!(
            to_string(&(vec![11, 13], vec!["foo", "bar"])).unwrap(),
            r#"([
    11
    13
], [
    "foo"
    "bar"
])"#
        );
    }

    #[test]
    fn test_mix_list_and_object() {
        #[derive(Serialize)]
        struct Object {
            id: i32,
            name: String,
        }

        #[derive(Serialize)]
        struct ObjectList {
            id: i32,
            items: Vec<i32>,
        }

        assert_eq!(
            to_string(&vec![
                Object {
                    id: 11,
                    name: "foo".to_owned()
                },
                Object {
                    id: 13,
                    name: "bar".to_owned()
                }
            ])
            .unwrap(),
            r#"[
    {
        id: 11
        name: "foo"
    }
    {
        id: 13
        name: "bar"
    }
]"#
        );

        assert_eq!(
            to_string(&ObjectList {
                id: 456,
                items: vec![11, 13, 17, 19]
            })
            .unwrap(),
            r#"{
    id: 456
    items: [
        11
        13
        17
        19
    ]
}"#
        );
    }

    #[test]
    fn test_mix_tuple_and_object() {
        #[derive(Serialize)]
        struct Object {
            id: i32,
            name: String,
        }

        #[derive(Serialize)]
        struct ObjectDetail {
            id: i32,
            address: (i32, String),
        }

        assert_eq!(
            to_string(&(
                123,
                Object {
                    id: 11,
                    name: "foo".to_owned()
                }
            ))
            .unwrap(),
            r#"(123, {
    id: 11
    name: "foo"
})"#
        );

        assert_eq!(
            to_string(&ObjectDetail {
                id: 456,
                address: (11, "sz".to_owned())
            })
            .unwrap(),
            r#"{
    id: 456
    address: (11, "sz")
}"#
        );
    }

    #[test]
    fn test_mix_variant_object() {
        #[derive(Serialize)]
        struct Simple {
            id: i32,
            name: String,
        }

        #[derive(Serialize)]
        struct Complex {
            id: i32,
            checked: bool,
            fullname: String,
        }

        #[derive(Serialize)]
        enum Item {
            Empty,
            Minimal(i32),
            Simple(Simple),
            Complex(Complex),
        }

        assert_eq!(
            to_string(&vec![
                Item::Empty,
                Item::Minimal(11),
                Item::Simple(Simple {
                    id: 13,
                    name: "foo".to_owned()
                }),
                Item::Complex(Complex {
                    id: 17,
                    checked: true,
                    fullname: "foobar".to_owned()
                })
            ])
            .unwrap(),
            r#"[
    Item::Empty
    Item::Minimal(11)
    Item::Simple({
        id: 13
        name: "foo"
    })
    Item::Complex({
        id: 17
        checked: true
        fullname: "foobar"
    })
]"#
        );
    }
}
