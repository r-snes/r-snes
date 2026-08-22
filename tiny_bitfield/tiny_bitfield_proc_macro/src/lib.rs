use proc_macro2::{Spacing, TokenStream, TokenTree};

use crate::fields::{BitsType, Fields};

mod fields;

#[proc_macro]
pub fn bitfield_read(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    bitfield_read_impl(ts.into()).into()
}

fn bitfield_read_impl(ts: TokenStream) -> TokenStream {
    let mut tokens = ts.into_iter().peekable();
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

    let mut fields = Fields::default();
    while let Some(TokenTree::Ident(id)) = tokens.next_if(|t| matches!(t, TokenTree::Ident(_))) {
        for c in id.to_string().chars() {
            fields
                .extend_one_char(c)
                .expect("invalid character in bit pattern: {c:?}");
        }
    }

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
                panic!("field `{}` doesn't exist", from);
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

    let ty = BitsType::try_from(fields.bit_length()).expect("invalid bit pattern length");
    fields.generate_bindings(&expr, ty)
}
