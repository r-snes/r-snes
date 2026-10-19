//! Custom [`serde`] [`Serializer`]s built around
//! [`toml_edit`] to support serialising into existing [`DocumentMut`]s,
//! [`Item`]s or [`Value`]s, preserving formatting and comments

/// Serde trait implementations for [`InItemSerializer`]
pub mod in_item;
/// Serde trait implementations for [`InValueSerializer`]
pub mod in_value;

/// Helper for enum serialization
pub mod enum_ser;

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
    use std::fmt::Debug;

    use serde::{Deserialize, Serialize};
    use toml_edit::{DocumentMut, de::from_str};

    use crate::config::serialize_in_doc::InItemSerializer;

    fn is_default<T: Default + Eq>(x: &T) -> bool {
        x == &T::default()
    }

    struct InDocSerializeTest<T, F> {
        /// The initial TOML string which the config is built from
        pub init_toml: &'static str,

        /// Optional initial value which will be `assert_eq!`'d with
        /// the value parsed from `init_toml`
        pub init_value: Option<T>,

        /// The function that will mutate the parsed config
        pub edit: F,

        /// The expected string result after serialising the `edit`ed
        /// config value back in the initial DocumentMut
        pub expected_toml: &'static str,
    }

    impl<T, F> InDocSerializeTest<T, F>
    where
        T: Serialize + for<'a> Deserialize<'a> + Eq + Debug,
        F: FnOnce(&mut T),
    {
        #[track_caller]
        fn run(self) {
            let mut doc: DocumentMut = self.init_toml.parse().expect("valid TOML");
            let mut config = from_str::<T>(self.init_toml).unwrap();

            if let Some(init_value) = self.init_value {
                assert_eq!(config, init_value);
            }

            (self.edit)(&mut config);

            serialize_into_document(&mut doc, &config);
            assert_eq!(
                doc.to_string(),
                self.expected_toml,
                "\nExpected:\n==========\n{}==========\nBut got\n==========\n{}==========",
                self.expected_toml,
                doc.to_string(),
            );
        }
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
        InDocSerializeTest {
            init_toml: "",
            init_value: Some(SimpleStruct::default()),
            edit: |config: &mut SimpleStruct| {
                config.a = Some(34);
                config.b = Some(56);
                config.c = Some("foo".to_owned());
            },
            expected_toml: "a = 34\nb = 56\nc = \"foo\"\n",
        }
        .run();
    }

    #[test]
    fn change_all_options() {
        InDocSerializeTest {
            init_toml: r#"
                # some comment
                b = 3

                # other comment
                a = 0 # inline comment stays even when value is changed
                c = "foo"
                # trailing comment
            "#,
            init_value: Some(SimpleStruct {
                a: Some(0),
                b: Some(3),
                c: Some("foo".to_owned()),
            }),
            edit: |config: &mut SimpleStruct| {
                config.a = Some(34);
                config.b = Some(56);
                config.c = Some("bar".to_owned());
            },
            expected_toml: r#"
                # some comment
                b = 56

                # other comment
                a = 34 # inline comment stays even when value is changed
                c = "bar"
                # trailing comment
            "#,
        }
        .run();
    }

    #[test]
    fn change_one_option() {
        InDocSerializeTest {
            init_toml: r#"
                # header comment

                # other comment
                a = 0 # inline comment stays
                b = 3 # inline comment which should be preserved
            "#,
            init_value: Some(SimpleStruct {
                a: Some(0),
                b: Some(3),
                ..Default::default()
            }),
            edit: |config: &mut SimpleStruct| {
                config.a = Some(10);
            },
            expected_toml: r#"
                # header comment

                # other comment
                a = 10 # inline comment stays
                b = 3 # inline comment which should be preserved
            "#,
        }
        .run();
    }

    #[test]
    fn undef_one_option() {
        InDocSerializeTest {
            init_toml: r#"
                # gets removed
                a = 0 # inline comment gets removed too
                b = 3 # inline comment which should be preserved
            "#,
            init_value: Some(SimpleStruct {
                a: Some(0),
                b: Some(3),
                ..Default::default()
            }),
            edit: |config: &mut SimpleStruct| config.a = None,
            expected_toml: &r#"
                b = 3 # inline comment which should be preserved
            "#[1..], // trim leading newline
        }
        .run()
    }

    #[test]
    fn add_key_to_inline_table() {
        InDocSerializeTest {
            init_toml: r#"
                # comment
                simple = { a = 3 } # inline
            "#,
            init_value: Some(ComplexStruct {
                simple: SimpleStruct {
                    a: Some(3),
                    ..Default::default()
                },
                ..Default::default()
            }),
            edit: |config: &mut ComplexStruct| config.simple.b = Some(15),
            expected_toml: r#"
                # comment
                simple = { a = 3 , b = 15 } # inline
            "#,
        }
        .run()
    }

    #[test]
    fn remove_key_from_inline_table() {
        InDocSerializeTest {
            init_toml: r#"
                # comment
                simple = { a = 3 , c = "xxxxxxx" } # inline c
            "#,
            init_value: Some(ComplexStruct {
                simple: SimpleStruct {
                    a: Some(3),
                    c: Some("xxxxxxx".to_owned()),
                    ..Default::default()
                },
                ..Default::default()
            }),
            edit: |config: &mut ComplexStruct| config.simple.c = None,
            expected_toml: r#"
                # comment
                simple = { a = 3 } # inline c
            "#,
        }
        .run()
    }

    #[test]
    fn change_enum_unit_variant() {
        InDocSerializeTest {
            init_toml: r#"enums.enum1 = "UnitVariantA" # c"#,
            init_value: Some(ComplexStruct {
                enums: Enums {
                    enum1: Some(SomeEnum::UnitVariantA),
                },
                ..Default::default()
            }),
            edit: |config: &mut ComplexStruct| config.enums.enum1 = Some(SomeEnum::UnitVariantB),
            expected_toml: "enums.enum1 = \"UnitVariantB\" # c\n",
        }
        .run();
    }

    #[test]
    fn change_enum_struct_as_inline_table() {
        InDocSerializeTest {
            init_toml: "enums = { enum1.StructVariant = { x = 3, y = 8 }}",
            init_value: Some(ComplexStruct {
                enums: Enums {
                    enum1: Some(SomeEnum::StructVariant { x: 3, y: 8 }),
                },
                ..Default::default()
            }),
            edit: |config: &mut ComplexStruct| {
                config.enums.enum1 = Some(SomeEnum::StructVariant { x: 20, y: -4 })
            },
            expected_toml: "enums = { enum1.StructVariant = { x = 20, y = -4 }}\n",
        }
        .run()
    }

    #[test]
    fn change_enum_struct_as_toplevel_table() {
        for (init, expected) in [
            (
                r#"
                    # a
                    [enums.enum1.StructVariant] # b
                    x = 3
                    y = 8 # c
                "#,
                r#"
                    # a
                    [enums.enum1.StructVariant] # b
                    x = 20
                    y = -4 # c
                "#,
            ),
            (
                r#"
                    [enums.enum1] # a
                    # b
                    StructVariant = { x = 3, y = 8 }
                "#,
                r#"
                    [enums.enum1] # a
                    # b
                    StructVariant = { x = 20, y = -4 }
                "#,
            ),
            (
                r#"
                    [enums.enum1] # a
                    # b
                    StructVariant.x = 3 # d
                    StructVariant.y = 8 # e
                "#,
                r#"
                    [enums.enum1] # a
                    # b
                    StructVariant.x = 20 # d
                    StructVariant.y = -4 # e
                "#,
            ),
            (
                r#"
                    enums.enum1.StructVariant.x = 3 # d
                    enums.enum1.StructVariant.y = 8 # e
                "#,
                r#"
                    enums.enum1.StructVariant.x = 20 # d
                    enums.enum1.StructVariant.y = -4 # e
                "#,
            ),
        ] {
            InDocSerializeTest {
                init_toml: init,
                init_value: Some(ComplexStruct {
                    enums: Enums {
                        enum1: Some(SomeEnum::StructVariant { x: 3, y: 8 }),
                    },
                    ..Default::default()
                }),
                edit: |config: &mut ComplexStruct| {
                    config.enums.enum1 = Some(SomeEnum::StructVariant { x: 20, y: -4 })
                },
                expected_toml: expected,
            }
            .run()
        }
    }
}
