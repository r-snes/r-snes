use proc_macro2::{Punct, Spacing, TokenStream, TokenTree};
use quote::quote;

use crate::fields::Fields;

mod fields;

#[proc_macro]
pub fn bitfield_read(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    bitfield_read_impl(ts.into()).into()
}

fn get_bits_type_from_bitlength(bit_length: u32) -> TokenStream {
    match bit_length {
        1 => quote!(bool),
        8 => quote!(u8),
        16 => quote!(u16),
        32 => quote!(u32),
        64 => quote!(u64),
        128 => quote!(u128),
        _ => quote!(compile_error!("invalid bit pattern length")),
    }
}

fn bitfield_read_impl(ts: TokenStream) -> TokenStream {
    let mut tokens = ts.into_iter();
    let expr = {
        let take_while = tokens.by_ref().take_while(|t| {
            if let TokenTree::Punct(p) = t
                && p.as_char() == ':'
            {
                false
            } else {
                true
            }
        });

        take_while.collect::<TokenStream>()
    };
    let mut fields = {
        let bit_pattern = tokens.next().expect("unexpected end of stream");
        let TokenTree::Ident(id) = bit_pattern else {
            panic!("bit pattern should be an identifier, like `aabbcc_d`");
        };
        let Ok(fields) = id.to_string().parse::<Fields>() else {
            panic!("invalid bit pattern format");
        };
        fields
    };

    if let Some(rename_block) = tokens.next() {
        let TokenTree::Group(rename_block) = rename_block else {
            panic!("expected rename block or end of stream");
        };
        let mut rename_tokens = rename_block.stream().into_iter();
        while let Some(token) = rename_tokens.next() {
            let TokenTree::Ident(to) = token else {
                panic!("rename field shoud be an identifier");
            };
            if let Some(TokenTree::Punct(p)) = rename_tokens.next()
                && p.as_char() == '='
                && p.spacing() == Spacing::Alone
            {
            } else {
                panic!("expected equals sign");
            };
            let Some(TokenTree::Ident(from)) = rename_tokens.next() else {
                panic!(
                    "expected rename field from {:?}",
                    fields.fields.iter().map(|f| &f.name).collect::<Vec<_>>()
                );
            };
            if let Err(()) = fields.rename_field(&from.to_string(), to.to_string()) {
                panic!("field `{}` doesn't exist", from.to_string());
            }
            if let Some(TokenTree::Punct(p)) = rename_tokens.next()
                && p.as_char() == ';'
                && p.spacing() == Spacing::Alone
            {
            } else {
                panic!("expected semicolon");
            };
        }

        assert!(
            tokens.next().is_none(),
            "unexpected tokens after rename block"
        );
    }

    let ty = get_bits_type_from_bitlength(fields.bit_length());
    fields.generate_bindings(&expr, &ty)
}
