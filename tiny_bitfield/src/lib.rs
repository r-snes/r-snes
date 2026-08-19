use std::num::NonZeroUsize;

#[derive(PartialEq, Eq, Debug)]
struct Field {
    pub name: String,
    pub width: NonZeroUsize,
}

impl Field {
    pub fn new(name: char, width: NonZeroUsize) -> Self {
        Self {
            name: name.to_string(), width
        }
    }
}

#[derive(PartialEq, Eq, Default)]
struct Fields {
    pub fields: Vec<Field>,
}

impl Fields {
    pub fn bit_length(&self) -> usize {
        self.fields.iter().fold(0, |acc, f| acc + f.width.get())
    }

    pub fn has_char(&self, c: char) -> bool {
        self.fields.iter().any(|f| f.name == c.to_string())
    }
}

impl std::str::FromStr for Fields {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let extend_one_char = |f: &mut Fields, c: char| -> Result<(), Self::Err> {
            if let Some(last) = f.fields.last_mut() && last.name == c.to_string() {
                last.width = last.width.saturating_add(1);
            } else {
                if c != '_' && f.has_char(c) {
                    return Err(());
                }
                f.fields.push(Field {
                    name: c.to_string(),
                    width: NonZeroUsize::new(1).unwrap(),
                });
            }

            Ok(())
        };

        let mut ret = Fields::default();
        for c in s.chars() {
            if c.is_whitespace() {
                continue;
            }
            let c = match c {
                // accept both dash and underscore as ignored bit
                '-' | '_' => '_',

                // non-repeating valid identifier start char,
                c if unicode_ident::is_xid_start(c) => c,

                _ => return Err(()),
            };

            extend_one_char(&mut ret, c)?;
        }

        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_4_4() {
        let f: Fields = "AAAABBBB".parse().expect("successful parsing");

        assert_eq!(f.bit_length(), 8);
        assert_eq!(f.fields, vec![
            Field::new('A', NonZeroUsize::new(4).unwrap()),
            Field::new('B', NonZeroUsize::new(4).unwrap()),
        ]);
    }

    #[test]
    fn space_separators() {
        let f: Fields = "paaa bbbb  cccc dddd".parse().expect("successful parsing");

        assert_eq!(f.bit_length(), 16);
        assert_eq!(f.fields, vec![
            Field::new('p', NonZeroUsize::new(1).unwrap()),
            Field::new('a', NonZeroUsize::new(3).unwrap()),
            Field::new('b', NonZeroUsize::new(4).unwrap()),
            Field::new('c', NonZeroUsize::new(4).unwrap()),
            Field::new('d', NonZeroUsize::new(4).unwrap()),
        ]);
    }

    #[test]
    fn ignored_bits() {
        let f: Fields = "aaa--bb_".parse().expect("successful parsing");

        assert_eq!(f.bit_length(), 8);
        assert_eq!(f.fields, vec![
            Field::new('a', NonZeroUsize::new(3).unwrap()),
            Field::new('_', NonZeroUsize::new(2).unwrap()),
            Field::new('b', NonZeroUsize::new(2).unwrap()),
            Field::new('_', NonZeroUsize::new(1).unwrap()),
        ]);
    }
}
