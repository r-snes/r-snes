use serde::Serializer;
use toml_edit::{Decor, DocumentMut, Formatted, Item, Value};

pub struct InDocSerializer<'a> {
    pub item: &'a mut Item,
}

// #[derive(Debug)]
// pub struct InDocSerializeError<'a> {
//     pub error: toml_edit::ser::Error,
//     pub item: Option<&'a mut Item>,
// }

// impl std::fmt::Display for InDocSerializeError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         self.error.fmt(f)
//     }
// }

// impl std::error::Error for InDocSerializeError<'_> {}

// impl serde::ser::Error for InDocSerializeError<'_> {
//     fn custom<T>(msg: T) -> Self where T: std::fmt::Display {
//         Self {
//             error: toml_edit::ser::Error::custom(msg),
//             item: None,
//         }
//     }
// }

macro_rules! serialize_simple_value {
    ($item:expr, $val:expr, $ty:ty, $variant:ident) => {
        {
        let decor = match $item {
            Item::Value(v) => std::mem::take(v.decor_mut()),
            Item::None => Decor::default(),
            Item::Table(_) | Item::ArrayOfTables(_) => {
                eprintln!(concat!("weird replacement of non-value to ", stringify!($ty)));
                Decor::default()
            }
        };
        let mut res = Formatted::new($val);
        *res.decor_mut() = decor;
        *$item = Item::Value(Value::$variant(res));
        Ok(())
    }
    }
}

impl serde::ser::Serializer for InDocSerializer<'_> {
    type Ok = ();
    type Error = toml_edit::ser::Error;
    type SerializeSeq = SerializeValueArray;
    type SerializeTuple = SerializeValueArray;
    type SerializeTupleStruct = SerializeValueArray;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = SerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        serialize_simple_value!(self.item, v, bool, Boolean)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        serialize_simple_value!(self.item, v, i64, Integer)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        let v: i64 = v
            .try_into()
            .map_err(|_err| Self::Error::OutOfRange(Some("u64")))?;
        self.serialize_i64(v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, mut v: f64) -> Result<Self::Ok, Self::Error> {
        if v.is_nan() {
            v = v.copysign(1.0);
        }

        serialize_simple_value!(self.item, v, f64, Float)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        serialize_simple_value!(self.item, v.to_owned(), &str, String)
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        use serde::ser::Serialize;
        value.serialize(self)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        *self.item = Item::None;
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Self::Error::UnsupportedType(Some("unit")))
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(Self::Error::UnsupportedType(Some(name)))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        let value = value.serialize(self)?;
        let mut table = toml_edit::InlineTable::new();
        table.insert(variant, value);
        Ok(table.into())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeValueArray::seq(len))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SerializeTupleVariant::tuple(variant, len))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeMap::map(len))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerializeMap::struct_(name, Some(len)))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(SerializeStructVariant::struct_(variant, len))
    }
}

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};
    use toml_edit::{DocumentMut, Item, Value, de::{from_document, from_str}};

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    struct SimpleStruct {
        a: Option<i32>,
        b: Option<i32>,
        c: Option<String>,
    }

    fn serialize_into_document(doc: &mut DocumentMut, ser: &impl Serialize) {
        todo!("IMPL THE THING")
    }

    #[test]
    fn define_all_options() {
        let init_toml = "";
        let mut doc: DocumentMut = init_toml.parse().expect("valid TOML");
        let mut doc_clone = doc.clone();
        let mut config = from_str::<SimpleStruct>(init_toml).unwrap();

        assert_eq!(
            config,
            SimpleStruct {
                a: None,
                b: None,
                c: None,
            }
        );

        doc_clone["a"] = 34.into();
        doc_clone["b"] = 56.into();
        doc_clone["c"] = "foo".into();
        config.a = Some(34);
        config.b = Some(56);
        config.c = Some("foo".to_owned());

        let expected = "a = 34\nb = 56\nc = \"foo\"\n";
        assert_eq!(doc_clone.to_string(), expected);
        serialize_into_document(&mut doc, &config);
        assert_eq!(doc.to_string(), expected);
    }

    #[test]
    fn change_all_options() {
        let init_toml = r#"
            # some comment
            b = 3

            # other comment
            a = 0 # inline comment gets removed when value is edited
            c = "foo"
            # trailing comment
        "#;
        let mut doc: DocumentMut = init_toml.parse().expect("valid TOML");
        let mut doc_clone = doc.clone();
        let mut config = from_str::<SimpleStruct>(init_toml).unwrap();

        assert_eq!(
            config,
            SimpleStruct {
                a: Some(0),
                b: Some(3),
                c: Some("foo".to_owned()),
            }
        );

        doc_clone["a"] = 34.into();
        doc_clone["b"] = 56.into();
        doc_clone["c"] = "bar".into();
        config.a = Some(34);
        config.b = Some(56);
        config.c = Some("bar".to_owned());

        let expected = r#"
            # some comment
            b = 56

            # other comment
            a = 34
            c = "bar"
            # trailing comment
        "#;
        assert_eq!(doc_clone.to_string(), expected);
        serialize_into_document(&mut doc, &config);
        assert_eq!(doc.to_string(), expected);
    }

    #[test]
    fn change_one_option() {
        let init_toml = r#"
            # header comment

            # other comment
            a = 0 # inline comment gets removed when value is edited
            b = 3 # inline comment which should be preserved
        "#;
        let mut doc: DocumentMut = init_toml.parse().expect("valid TOML");
        let mut doc_clone = doc.clone();
        let mut config = from_str::<SimpleStruct>(init_toml).unwrap();

        assert_eq!(
            config,
            SimpleStruct {
                a: Some(0),
                b: Some(3),
                c: None,
            }
        );

        doc_clone["a"] = 10.into();
        doc_clone["b"] = 3.into();
        config.a = Some(10);

        let expected = r#"
            # header comment

            # other comment
            a = 10
            b = 3 # inline comment which should be preserved
        "#;
        assert_eq!(doc_clone.to_string(), expected);
        serialize_into_document(&mut doc, &config);
        assert_eq!(doc.to_string(), expected);
    }

    #[test]
    fn undef_one_option() {
        let init_toml = r#"
            # gets removed
            a = 0 # inline comment gets removed too
            b = 3 # inline comment which should be preserved
        "#;
        let mut doc: DocumentMut = init_toml.parse().expect("valid TOML");
        let mut doc_clone = doc.clone();
        let mut config = from_str::<SimpleStruct>(init_toml).unwrap();

        assert_eq!(
            config,
            SimpleStruct {
                a: Some(0),
                b: Some(3),
                c: None,
            }
        );

        doc_clone["a"] = Item::None;
        config.a = None;

        let expected = "            b = 3 # inline comment which should be preserved\n        ";
        assert_eq!(doc_clone.to_string(), expected);
        serialize_into_document(&mut doc, &config);
        assert_eq!(doc.to_string(), expected);
    }
}
