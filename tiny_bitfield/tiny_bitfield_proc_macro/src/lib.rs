use proc_macro2::{TokenStream, TokenTree};

use crate::{
    fields::{BitsType, Fields},
    rename_fields::RenameRetypeGroup,
};

mod fields;
mod rename_fields;

#[proc_macro]
pub fn bitfield_read(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    bitfield_read_impl(ts.into()).into()
}

fn bitfield_read_impl(ts: TokenStream) -> TokenStream {
    let mut tokens = ts.into_iter().peekable();
    let expr = tokens
        .by_ref()
        .take_while(|t| !matches!(t, TokenTree::Punct(p) if p.as_char() == ':'))
        .collect::<TokenStream>();

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
        let rename_tokens = rename_block.stream().into_iter();
        let renames = RenameRetypeGroup::from_tokens(rename_tokens, &fields);

        renames.apply_to(&mut fields);

        assert!(
            tokens.next().is_none(),
            "unexpected tokens after rename block"
        );
    }

    let ty = BitsType::try_from(fields.bit_length()).expect("invalid bit pattern length");
    fields.generate_bindings(&expr, ty)
}
