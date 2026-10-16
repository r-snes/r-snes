//! Custom [`serde`] [`Serializer`]s built around
//! [`toml_edit`] to support serialising into existing [`DocumentMut`]s,
//! [`Item`]s or [`Value`]s, preserving formatting and comments

/// Serde trait implementations for [`InItemSerializer`]
pub mod in_item;
/// Serde trait implementations for [`InValueSerializer`]
pub mod in_value;

#[doc(inline)]
pub use toml_edit::ser::Error as SerError;
use toml_edit::{DocumentMut, Item, Value};

/// Serialize into an existing item
pub struct InItemSerializer<'a> {
    /// The item to serialize into
    item: &'a mut Item,
}

impl<'a> InItemSerializer<'a> {
    /// Create a new [`InItemSerializer`] to serialize into
    /// the existing item passed as argument
    pub fn new(item: &'a mut Item) -> Self {
        Self { item }
    }

    /// Create an [`InItemSerializer`] to serialize into a
    /// [`DocumentMut`], using the document's root item
    /// as the item to serialize into
    pub fn in_document(doc: &'a mut DocumentMut) -> Self {
        Self {
            item: doc.as_item_mut(),
        }
    }
}

/// Serialize into an existing value
pub struct InValueSerializer<'a> {
    /// The item to serialize into
    value: &'a mut Value,
}

impl<'a> InValueSerializer<'a> {
    /// Creates a new [`InValueSerializer`] to serialize into
    /// the value passed as argument
    pub fn new(value: &'a mut Value) -> Self {
        Self { value }
    }
}

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};
    use toml_edit::{DocumentMut, de::from_str};

    use crate::config::serialize_in_doc::InItemSerializer;

    fn is_default<T: Default + Eq>(x: &T) -> bool {
        x == &T::default()
    }

    #[derive(Default, Serialize, Deserialize, Debug, PartialEq, Eq)]
    struct ComplexStruct {
        #[serde(default, skip_serializing_if = "is_default")]
        simple: SimpleStruct,
        num: Option<i32>,
        #[serde(default)]
        strings: Vec<String>,
        #[serde(default, skip_serializing_if = "is_default")]
        enums: Enums,
    }

    #[derive(Default, Serialize, Deserialize, Debug, PartialEq, Eq)]
    struct SimpleStruct {
        a: Option<i32>,
        b: Option<i32>,
        c: Option<String>,
    }

    #[derive(Default, Serialize, Deserialize, Debug, PartialEq, Eq)]
    struct Enums {
        enum1: Option<SomeEnum>,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    enum SomeEnum {
        UnitVariantA,
        UnitVariantB,
        NewTypeVariant(u8),
        TupleVariant(u32, u32, u32),
        StructVariant { x: i16, y: i16 },
    }

    fn serialize_into_document(doc: &mut DocumentMut, value: &impl Serialize) {
        value
            .serialize(InItemSerializer {
                item: doc.as_item_mut(),
            })
            .unwrap()
    }

    #[test]
    fn define_all_options() {
        let init_toml = "";
        let mut doc: DocumentMut = init_toml.parse().expect("valid TOML");
        let mut config = from_str::<SimpleStruct>(init_toml).unwrap();

        assert_eq!(config, SimpleStruct::default());

        config.a = Some(34);
        config.b = Some(56);
        config.c = Some("foo".to_owned());

        let expected = "a = 34\nb = 56\nc = \"foo\"\n";
        serialize_into_document(&mut doc, &config);
        assert_eq!(doc.to_string(), expected);
    }

    #[test]
    fn change_all_options() {
        let init_toml = r#"
            # some comment
            b = 3

            # other comment
            a = 0 # inline comment stays even when value is changed
            c = "foo"
            # trailing comment
        "#;
        let mut doc: DocumentMut = init_toml.parse().expect("valid TOML");
        let mut config = from_str::<SimpleStruct>(init_toml).unwrap();

        assert_eq!(
            config,
            SimpleStruct {
                a: Some(0),
                b: Some(3),
                c: Some("foo".to_owned()),
            }
        );

        config.a = Some(34);
        config.b = Some(56);
        config.c = Some("bar".to_owned());

        let expected = r#"
            # some comment
            b = 56

            # other comment
            a = 34 # inline comment stays even when value is changed
            c = "bar"
            # trailing comment
        "#;
        serialize_into_document(&mut doc, &config);
        assert_eq!(doc.to_string(), expected);
    }

    #[test]
    fn change_one_option() {
        let init_toml = r#"
            # header comment

            # other comment
            a = 0 # inline comment stays
            b = 3 # inline comment which should be preserved
        "#;
        let mut doc: DocumentMut = init_toml.parse().expect("valid TOML");
        let mut config = from_str::<SimpleStruct>(init_toml).unwrap();

        assert_eq!(
            config,
            SimpleStruct {
                a: Some(0),
                b: Some(3),
                ..Default::default()
            }
        );

        config.a = Some(10);

        let expected = r#"
            # header comment

            # other comment
            a = 10 # inline comment stays
            b = 3 # inline comment which should be preserved
        "#;
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
        let mut config = from_str::<SimpleStruct>(init_toml).unwrap();

        assert_eq!(
            config,
            SimpleStruct {
                a: Some(0),
                b: Some(3),
                ..Default::default()
            }
        );

        config.a = None;

        let expected = "            b = 3 # inline comment which should be preserved\n        ";
        serialize_into_document(&mut doc, &config);
        assert_eq!(doc.to_string(), expected);
    }

    #[test]
    fn add_key_to_inline_table() {
        let init_toml = r#"
            # comment
            simple = { a = 3 } # inline
        "#;
        let mut doc: DocumentMut = init_toml.parse().expect("valid TOML");
        let mut config = from_str::<ComplexStruct>(init_toml).unwrap();

        assert_eq!(
            config,
            ComplexStruct {
                simple: SimpleStruct {
                    a: Some(3),
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        config.simple.b = Some(15);

        let expected = r#"
            # comment
            simple = { a = 3 , b = 15 } # inline
        "#;
        serialize_into_document(&mut doc, &config);
        assert_eq!(doc.to_string(), expected);
    }

    #[test]
    fn remove_key_from_inline_table() {
        let init_toml = r#"
            # comment
            simple = { a = 3 , c = "xxxxxxx" } # inline c
        "#;
        let mut doc: DocumentMut = init_toml.parse().expect("valid TOML");
        let mut config = from_str::<ComplexStruct>(init_toml).unwrap();

        assert_eq!(
            config,
            ComplexStruct {
                simple: SimpleStruct {
                    a: Some(3),
                    c: Some("xxxxxxx".to_owned()),
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        config.simple.c = None;

        let expected = r#"
            # comment
            simple = { a = 3 } # inline c
        "#;
        serialize_into_document(&mut doc, &config);
        assert_eq!(doc.to_string(), expected);
    }

    #[test]
    fn change_enum_value() {
        let init_toml = r#"
            enums.enum1 = "UnitVariantA"
        "#;
        let mut doc: DocumentMut = init_toml.parse().expect("valid TOML");
        let mut config = from_str::<ComplexStruct>(init_toml).unwrap();

        assert_eq!(
            config,
            ComplexStruct {
                enums: Enums {
                    enum1: Some(SomeEnum::UnitVariantA)
                },
                ..Default::default()
            },
        );

        config.enums.enum1 = Some(SomeEnum::UnitVariantB);

        let expected = r#"
            enums.enum1 = "UnitVariantB"
        "#;
        serialize_into_document(&mut doc, &config);
        assert_eq!(doc.to_string(), expected);
    }

    #[test]
    fn change_enum_struct_as_value() {
        let init_toml = r#"
            enums = { enum1.StructVariant = { x = 3, y = 8 }}
        "#;
        let mut doc: DocumentMut = init_toml.parse().expect("valid TOML");
        let mut config = from_str::<ComplexStruct>(init_toml).unwrap();

        assert_eq!(
            config,
            ComplexStruct {
                enums: Enums {
                    enum1: Some(SomeEnum::StructVariant { x: 3, y: 8 })
                },
                ..Default::default()
            },
        );

        config.enums.enum1 = Some(SomeEnum::StructVariant { x: 20, y: -4 });

        let expected = r#"
            enums = { enum1.StructVariant = { x = 20, y = -4 }}
        "#;
        serialize_into_document(&mut doc, &config);
        assert_eq!(doc.to_string(), expected);
    }
}
