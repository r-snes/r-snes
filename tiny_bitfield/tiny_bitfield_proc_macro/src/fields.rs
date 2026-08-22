use std::num::NonZeroU32;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;

#[derive(PartialEq, Eq, Debug)]
pub struct Field {
    pub name: String,
    pub width: NonZeroU32,
}

impl Field {
    #[cfg(test)]
    pub fn new(name: char, width: NonZeroU32) -> Self {
        Self {
            name: name.to_string(),
            width,
        }
    }
}

#[derive(PartialEq, Eq, Default)]
pub struct Fields {
    pub fields: Vec<Field>,
}

impl Fields {
    pub fn bit_length(&self) -> u32 {
        self.fields.iter().fold(0, |acc, f| acc + f.width.get())
    }

    pub fn has_char(&self, c: char) -> bool {
        self.fields.iter().any(|f| f.name == c.to_string())
    }

    pub fn generate_bindings(&self, expr: &TokenStream, ty: &TokenStream) -> TokenStream {
        let mut rshift = 0;
        let mut decls = quote!();
        let mut assigns = quote!();

        for Field { name, width } in self.fields.iter().rev() {
            let name = Ident::new(name, Span::call_site());
            let width = width.get();
            let mask = Literal::u32_unsuffixed(1_u32.strict_shl(width).strict_sub(1));

            decls.extend(quote!(let #name: #ty;));
            assigns.extend(quote!(
                #name = ((#expr) >> #rshift) & #mask;
            ));
            rshift += width;
        }
        quote! {
            #decls
            {
                let expr: #ty = #expr;
                #assigns
            }
        }
    }

    pub fn rename_field(&mut self, from: &str, to: String) -> Result<(), ()> {
        for field in self.fields.iter_mut() {
            if field.name == from {
                field.name = to;
                return Ok(());
            }
        }
        Err(())
    }

    pub fn extend_one_char(&mut self, c: char) -> Result<(), ()> {
        if let Some(last) = self.fields.last_mut()
            && last.name == c.to_string()
        {
            last.width = last.width.saturating_add(1);
        } else {
            if c != '_' && self.has_char(c) {
                return Err(());
            }
            self.fields.push(Field {
                name: c.to_string(),
                width: NonZeroU32::new(1).unwrap(),
            });
        }

        Ok(())
    }
}

impl std::str::FromStr for Fields {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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

            ret.extend_one_char(c)?;
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
        assert_eq!(
            f.fields,
            vec![
                Field::new('A', NonZeroU32::new(4).unwrap()),
                Field::new('B', NonZeroU32::new(4).unwrap()),
            ]
        );
    }

    #[test]
    fn space_separators() {
        let f: Fields = "paaa bbbb  cccc dddd".parse().expect("successful parsing");

        assert_eq!(f.bit_length(), 16);
        assert_eq!(
            f.fields,
            vec![
                Field::new('p', NonZeroU32::new(1).unwrap()),
                Field::new('a', NonZeroU32::new(3).unwrap()),
                Field::new('b', NonZeroU32::new(4).unwrap()),
                Field::new('c', NonZeroU32::new(4).unwrap()),
                Field::new('d', NonZeroU32::new(4).unwrap()),
            ]
        );
    }

    #[test]
    fn ignored_bits() {
        let f: Fields = "aaa--bb_".parse().expect("successful parsing");

        assert_eq!(f.bit_length(), 8);
        assert_eq!(
            f.fields,
            vec![
                Field::new('a', NonZeroU32::new(3).unwrap()),
                Field::new('_', NonZeroU32::new(2).unwrap()),
                Field::new('b', NonZeroU32::new(2).unwrap()),
                Field::new('_', NonZeroU32::new(1).unwrap()),
            ]
        );
    }
}
