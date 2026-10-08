mod serialize_in_doc;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    foo: Option<i32>,
    bar: String,
}

#[cfg(test)]
mod test {
    use super::*;
    use toml_edit::{
        DocumentMut,
        de::from_document,
        ser::{ValueSerializer, to_document},
    };

    #[test]
    fn read_from_toml() {
        let toml = r#"
            foo = 3
            bar = "hello"
        "#;

        let doc = toml.parse::<DocumentMut>().unwrap();
        let config = from_document::<Config>(doc).unwrap();
        assert_eq!(config.foo, Some(3));
        assert_eq!(config.bar, "hello");
    }

    #[test]
    fn read_edit_in_place() {
        let toml = r#"
            # some toml comment
            bar = "hello"
        "#;

        let doc = toml.parse::<DocumentMut>().unwrap();
        let mut config = from_document::<Config>(doc).unwrap();

        config.bar = "bye".to_owned();
        let updated_doc = config.serialize(ValueSerializer::default()).unwrap();
        let updated_doc = to_document(&config).unwrap();
        // assert_eq!(
        //     updated_doc.to_string(),
        //     r#"# some toml comment\nbar = "bye"\n"#,
        // );
    }
}
