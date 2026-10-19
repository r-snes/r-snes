use crate::config::serialize_in_doc::{InValueSerializer, in_value::EnumSerializer};

use super::{InItemSerializer, SerError};
use std::ops::{IndexMut, Not};

use serde::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};
use toml_edit::{Decor, Formatted, InlineEntry, InlineTable, Item, Table, TableLike, Value};

macro_rules! serialize_simple_value {
    ($item_mut:expr, $val:expr, $variant:ident) => {{
        let decor = match $item_mut {
            Item::Value(v) => std::mem::take(v.decor_mut()),
            Item::None => Decor::default(),
            Item::Table(_) | Item::ArrayOfTables(_) => {
                eprintln!(concat!(
                    "weird replacement of non-value to ",
                    stringify!($variant)
                ));
                Decor::default()
            }
        };
        let mut res = Formatted::new($val);
        *res.decor_mut() = decor;
        *$item_mut = Item::Value(Value::$variant(res));
        Ok(())
    }};
}

impl<'a> serde::ser::Serializer for InItemSerializer<'a> {
    type Ok = ();
    type Error = SerError;
    type SerializeSeq = InArraySerializer;
    type SerializeTuple = InArraySerializer;
    type SerializeTupleStruct = InArraySerializer;
    type SerializeTupleVariant = EnumSerializer<InArraySerializer>;
    type SerializeMap = InTableSerializer<'a>;
    type SerializeStruct = InTableSerializer<'a>;
    type SerializeStructVariant = EnumSerializer<InTableSerializer<'a>>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        serialize_simple_value!(self.item, v, Boolean)
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
        serialize_simple_value!(self.item, v, Integer)
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

        serialize_simple_value!(self.item, v, Float)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        serialize_simple_value!(self.item, v.to_owned(), String)
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        use serde::ser::Serialize;
        // calls to serialize_seq to encode &[u8] as a serde sequence of u8s
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
        Ok(InArraySerializer {})
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
        Ok(InTableSerializer::in_item(self.item, len))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(InTableSerializer::in_item(self.item, Some(len)))
    }

    // struct-like enum variant like C in `enum E { A, B, C { some_field: i32 } }`
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(EnumSerializer::new_in_item(variant, self.item, |entry| {
            InTableSerializer::in_item(
                entry.or_insert(Item::Table(Table::new())),
                Some(len),
            )
        }))
    }
}

/// Helper enum to handle a mutable reference to either
/// a [`Table`] or an [`InlineTable`]
pub enum TableMut<'a> {
    /// Contains an [`InlineTable`]
    Inline(&'a mut InlineTable),
    /// Contains a [`Table`]
    TopLevel(&'a mut Table),
}

impl<'a> TableMut<'a> {
    /// Forwards the [`TableLike::iter`] method
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Item)> {
        match self {
            Self::Inline(tab) => TableLike::iter(*tab),
            Self::TopLevel(tab) => tab.iter(),
        }
    }

    /// Forwards the [`TableLike::remove`] method
    pub fn remove(&mut self, key: &str) {
        match self {
            Self::Inline(tab) => {
                tab.remove(key);
            }
            Self::TopLevel(tab) => {
                tab.remove(key);
            }
        };
    }
}

/// Custom serializer to write maps and structs in an existing
/// [`Table`] or [`InlineTable`]
pub struct InTableSerializer<'a> {
    /// The table to serialize into
    pub table: TableMut<'a>,
    /// Key which the next item will serialize into (used for [`SerializeMap`])
    pub pending_key: Option<Item>,
    /// Vec of keys we have serialized: kept track of to remove
    /// entries which existed in [`table`](struct.InTableSerializer#structfield.table)
    /// but haven't been listed again with calls to
    /// [`serialize_field`](SerializeStruct::serialize_field)/
    /// [`serialize_key`](SerializeMap::serialize_key) once
    /// [`SerializeStruct::end`] is called: at that point those entries
    /// will be removed from the table.
    pub serialized_keys: Vec<String>,
}

impl<'a> InTableSerializer<'a> {
    /// Constructs a new [`InInlineTableSerializer`] which serializes into
    /// the existing table passed as argument
    pub fn new(table: TableMut<'a>, size_hint: Option<usize>) -> Self {
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

    /// Constructs a new [`InInlineTableSerializer`] which serializes into
    /// the value passed as argument, overriding it to be an [`InlineTable`]
    /// if it isn't one already
    pub fn in_item(item: &'a mut Item, size_hint: Option<usize>) -> Self {
        match item {
            Item::Table(tab) => Self::new(TableMut::TopLevel(tab), size_hint),
            Item::Value(Value::InlineTable(tab)) => Self::new(TableMut::Inline(tab), size_hint),
            i => {
                *i = Item::Value(Value::InlineTable(InlineTable::new()));
                let Item::Value(Value::InlineTable(tab)) = i else {
                    unreachable!()
                };
                tab.fmt();
                Self::new(TableMut::Inline(tab), size_hint)
            }
        }
    }
}

impl<'a> SerializeStruct for InTableSerializer<'a> {
    type Ok = ();
    type Error = SerError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        match &mut self.table {
            TableMut::TopLevel(table) => {
                self.serialized_keys.push(key.to_owned());
                value.serialize(InItemSerializer {
                    // table.index_mut with a &str key is infallible: it creates
                    // an Item::None place to write in in case there was no item before
                    item: table.index_mut(key),
                })
            }
            TableMut::Inline(table) => {
                match table.entry(key) {
                    InlineEntry::Occupied(mut e) => {
                        match value.serialize(InValueSerializer::new(e.get_mut())) {
                            Ok(()) => {
                                self.serialized_keys.push(key.to_owned());
                                Ok(())
                            }
                            // handle the "None value removes entry" by catching the UnsupportedNone error
                            Err(Self::Error::UnsupportedNone) => Ok(()),
                            e @ Err(_) => e,
                        }
                    }
                    InlineEntry::Vacant(e) => {
                        match value.serialize(toml_edit::ser::ValueSerializer::new()) {
                            Ok(val) => {
                                self.serialized_keys.push(key.to_owned());
                                e.insert(val);
                                Ok(())
                            }
                            Err(Self::Error::UnsupportedNone) => Ok(()),
                            Err(e) => Err(e),
                        }
                    }
                }
            }
        }
    }

    /// Removes the keys in [`self.table`] which haven't been serialised
    /// (not mentioned in any call to `serialize_field`)
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
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
            .for_each(|key| self.table.remove(key));
        Ok(())
    }
}
impl<'a> SerializeMap for InTableSerializer<'a> {
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
        Ok(())
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
