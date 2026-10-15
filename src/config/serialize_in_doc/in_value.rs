use super::{InValueSerializer, SerError};
use std::ops::{IndexMut, Not};

use serde::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};
use toml_edit::{Formatted, InlineTable, Value};

macro_rules! serialize_simple_value {
    ($value_mut:expr, $val:expr, $ty:ty, $variant:ident) => {{
        let decor = std::mem::take($value_mut.decor_mut());
        let mut res = Formatted::new($val);
        *res.decor_mut() = decor;
        *$value_mut = Value::$variant(res);
        Ok(())
    }};
}

impl<'a> serde::ser::Serializer for InValueSerializer<'a> {
    type Ok = ();
    type Error = SerError;
    type SerializeSeq = InArraySerializer;
    type SerializeTuple = InArraySerializer;
    type SerializeTupleStruct = InArraySerializer;
    type SerializeTupleVariant = InEnum<InArraySerializer>;
    type SerializeMap = InInlineTableSerializer<'a>;
    type SerializeStruct = InInlineTableSerializer<'a>;
    type SerializeStructVariant = InEnum<InInlineTableSerializer<'a>>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        serialize_simple_value!(self.value, v, bool, Boolean)
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
        serialize_simple_value!(self.value, v, i64, Integer)
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
        let v: i64 = v.try_into().map_err(|_err| Self::Error::OutOfRange(Some("u64")))?;
        self.serialize_i64(v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, mut v: f64) -> Result<Self::Ok, Self::Error> {
        if v.is_nan() {
            v = v.copysign(1.0);
        }

        serialize_simple_value!(self.value, v, f64, Float)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        serialize_simple_value!(self.value, v.to_owned(), &str, String)
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        use serde::ser::Serialize;
        // calls to serialize_seq to encode &[u8] as a serde sequence of u8s
        value.serialize(self)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(Self::Error::UnsupportedNone)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        // we serialize a Some(T) as just a T, so here we forward to T's Serialize impl
        value.serialize(self)
    }

    // unit type: `()`
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Self::Error::UnsupportedType(Some("unit")))
    }

    // unit structs like `struct A;`
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(Self::Error::UnsupportedType(Some(name)))
    }

    // unit enum variant, like enum E { A, B, C }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    // newtype struct (tuple struct with 1 field), like struct A(i32)
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

    // newtype enum variant (tuple variant with 1 element), like Ok/Err in Result
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
        // let value = value.serialize(self)?;
        // let mut table = toml_edit::InlineTable::new();
        // table.insert(variant, value);
        // Ok(table.into())
        todo!()
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        // Ok(SerializeValueArray::seq(len))
        todo!()
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    // tuple structs (with more than 1 elem): `struct A(i32, i32)`
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    // tuple enum variant (with more than 1 field) like A/B in
    // `enum E { A(i32, i32), B(char, String) }`
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        // Ok(SerializeTupleVariant::tuple(variant, len))
        todo!()
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(InInlineTableSerializer::in_value(self.value, len))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(InInlineTableSerializer::in_value(self.value, Some(len)))
    }

    // struct-like enum variant like C in `enum E { A, B, C { some_field: i32 } }`
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        // Ok(SerializeStructVariant::struct_(variant, len))
        todo!()
    }
}

/// Custom serializer to write maps and structs in an existing
/// [`InlineTable`]
pub struct InInlineTableSerializer<'a> {
    /// The table to serialize into
    pub table: &'a mut InlineTable,
    /// Key which the next item will serialize into (used for [`SerializeMap`])
    pub pending_key: Option<Value>,
    /// Vec of keys we have serialized: kept track of to remove
    /// entries which existed in [`table`](struct.InTableSerializer#structfield.table)
    /// but haven't been listed again with calls to
    /// [`serialize_field`](SerializeStruct::serialize_field)/
    /// [`serialize_key`](SerializeMap::serialize_key) once
    /// [`SerializeStruct::end`] is called: at that point those entries
    /// will be removed from the table.
    pub serialized_keys: Vec<String>,
}

impl<'a> InInlineTableSerializer<'a> {
    /// Constructs a new [`InInlineTableSerializer`] which serializes into
    /// the existing table passed as argument
    pub fn new(table: &'a mut InlineTable, size_hint: Option<usize>) -> Self {
        let key_vec = if let Some(size) = size_hint {
            Vec::with_capacity(size)
        } else {
            Vec::new()
        };

        Self {
            table,
            pending_key: None,
            serialized_keys: key_vec,
        }
    }
}

impl<'a> InInlineTableSerializer<'a> {
    /// Constructs a new [`InInlineTableSerializer`] which serializes into
    /// the value passed as argument, overriding it to be an [`InlineTable`]
    /// if it isn't one already
    pub fn in_value(value: &'a mut Value, size_hint: Option<usize>) -> Self {
        match value {
            Value::InlineTable(tab) => Self::new(tab, size_hint),
            i => {
                *i = Value::InlineTable(InlineTable::new());
                let Value::InlineTable(tab) = i else {
                    unreachable!()
                };
                tab.fmt();
                Self::new(tab, size_hint)
            }
        }
    }
}

impl<'a> SerializeStruct for InInlineTableSerializer<'a> {
    type Ok = ();
    type Error = SerError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        match value.serialize(InValueSerializer::new(self.table.index_mut(key))) {
            Ok(()) => {
                self.serialized_keys.push(key.to_owned());
                Ok(())
            }
            // handle the "None value removes entry" by catching the UnsupportedNone error
            Err(Self::Error::UnsupportedNone) => Ok(()),
            e @ Err(_) => e,
        }
    }

    /// Removes the keys in [`self.table`] which haven't been serialised
    /// (not mentioned in any call to `serialize_field`)
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.table
            .iter()
            .filter_map(|(key, _)| {
                self.serialized_keys
                    .iter()
                    .any(|ser_key| key == ser_key)
                    .not()
                    .then_some(key.to_owned())
            })
            .collect::<Vec<_>>() // intermediate collect to release the borrow on self.table
            .iter()
            .for_each(|key| {
                self.table.remove(key);
            });
        Ok(())
    }
}
impl<'a> SerializeMap for InInlineTableSerializer<'a> {
    type Ok = ();
    type Error = SerError;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

pub struct InArraySerializer {}
impl SerializeSeq for InArraySerializer {
    type Ok = ();
    type Error = SerError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
impl SerializeTuple for InArraySerializer {
    type Ok = ();
    type Error = SerError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
impl SerializeTupleStruct for InArraySerializer {
    type Ok = ();
    type Error = SerError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

pub struct InEnum<InnerSer> {
    inner: InnerSer,
}
impl<InnerSer: SerializeTuple> SerializeTupleVariant for InEnum<InnerSer> {
    type Ok = InnerSer::Ok;
    type Error = InnerSer::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
impl<InnerSer: SerializeStruct> SerializeStructVariant for InEnum<InnerSer> {
    type Ok = InnerSer::Ok;
    type Error = InnerSer::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
