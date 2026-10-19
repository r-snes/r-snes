use serde::ser::{SerializeStruct, SerializeStructVariant, SerializeTuple, SerializeTupleVariant};
use toml_edit::{Entry, InlineEntry, InlineTable, Item, Table, Value};

/// Helper struct to implement enum variant serializers (other than
/// unit variants).
///
/// This implements the logic where the enum serializes as a table,
/// with the variant name as the only key, and then delegates the
/// serialization of the variant's data (newtype, tuple, or struct)
/// to an inner serializer
pub struct EnumSerializer<InnerSer> {
    inner: InnerSer,
}
impl<InnerSer> EnumSerializer<InnerSer> {
    /// Build a new enum serializer which writes into a &mut Value
    pub fn new_in_value<'a>(
        variant: &str,
        value: &'a mut Value,
        build_inner: impl FnOnce(InlineEntry<'a>) -> InnerSer,
    ) -> Self {
        let entry = match value {
            Value::InlineTable(tab) => {
                if tab.len() == 1 && tab.get(variant).is_some() {
                    tab.entry(variant)
                } else {
                    tab.clear();
                    tab.entry(variant)
                }
            }
            _ => {
                let decor = std::mem::take(value.decor_mut());
                let mut res = InlineTable::new();
                *res.decor_mut() = decor;
                *value = Value::InlineTable(res);
                let Value::InlineTable(tab) = value else {
                    unreachable!()
                };
                tab.entry(variant)
            }
        };

        Self {
            inner: build_inner(entry),
        }
    }

    /// Build a new enum serializer which writes into a &mut Item
    pub fn new_in_item<'a>(
        variant: &str,
        item: &'a mut Item,
        build_inner: impl FnOnce(TableEntry<'a>) -> InnerSer,
    ) -> Self {
        let entry = match item {
            Item::Table(tab) => {
                if tab.len() == 1 && tab.get(variant).is_some() {
                    TableEntry::TopLevel(tab.entry(variant))
                } else {
                    tab.clear();
                    TableEntry::TopLevel(tab.entry(variant))
                }
            }
            Item::Value(value) => {
                return Self::new_in_value(variant, value, |inline_entry| {
                    build_inner(TableEntry::Inline(inline_entry))
                });
            }
            _ => {
                *item = Item::Table(Table::new());
                let Item::Table(tab) = item else {
                    unreachable!()
                };
                tab.set_implicit(true);
                TableEntry::TopLevel(tab.entry(variant))
            }
        };

        Self {
            inner: build_inner(entry),
        }
    }
}

/// Enum which represents either an [`InlineEntry`] or an [`Entry`]
pub enum TableEntry<'a> {
    /// Contains an [`InlineEntry`]
    Inline(InlineEntry<'a>),
    /// Contains an [`Entry`]
    TopLevel(Entry<'a>),
}

impl<InnerSer: SerializeTuple> SerializeTupleVariant for EnumSerializer<InnerSer> {
    type Ok = InnerSer::Ok;
    type Error = InnerSer::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.inner.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}
impl<InnerSer: SerializeStruct> SerializeStructVariant for EnumSerializer<InnerSer> {
    type Ok = InnerSer::Ok;
    type Error = InnerSer::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.inner.serialize_field(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}
