use {
    proc_macro2 as pm2,
    proc_macro::{
        TokenStream,
    },
    quote::{
        ToTokens,
        quote,
    },
    syn::{
        ItemStruct,
        parse::{self, Parse, ParseStream},
    }
};

#[proc_macro_derive(PermTreeNode)]
pub fn derive_perm_tree_node(input: TokenStream) -> TokenStream {
    derive_perm_tree_node_impl(input.into()).into()
}

fn derive_perm_tree_node_impl(input: pm2::TokenStream) -> pm2::TokenStream {
    let ast: PermTreeNodeDerive = match syn::parse2(input) {
        Ok(parsed) => parsed,
        Err(e) => return e.into_compile_error()
    };

    let mut ts = pm2::TokenStream::new();
    ast.to_tokens(&mut ts);
    ts
}

struct PermTreeNodeDerive {
    item: ItemStruct,
}

impl Parse for PermTreeNodeDerive {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        Ok(Self { item: input.call(ItemStruct::parse)? })
    }
}

impl ToTokens for PermTreeNodeDerive {
    fn to_tokens(&self, tokens: &mut pm2::TokenStream) {
        let ident = &self.item.ident;
        let (impl_generics, ty_generics, where_clause) = self.item.generics.split_for_impl();

        let match_arms = self.item.fields.iter().map(|f| {
            let ident = &f.ident.as_ref().expect("named struct field (no tuple struct)");
            let typ = &f.ty;
            let bytestring = pm2::Literal::byte_string(ident.to_string().as_bytes());

            quote! {
                // match arms for named fields:
                // `<root> = { internal = { ... }, external = { ... } }
                (piccolo::Value::String(s), _) if s.as_bytes() == #bytestring => {
                    // recurse in the named field's `from_lua` implementation
                    ret.#ident = <#typ>::from_lua(ctx, val)?;
                }

                // match arms for indexed fields (implicit = "all"):
                // `<root> = { "internal", "external" }
                (_, piccolo::Value::String(s)) if s.as_bytes() == #bytestring => {
                    ret.#ident = <#typ>::all();
                }
            }
        });

        tokens.extend(quote!(
            impl #impl_generics PermTreeNode for #ident #ty_generics #where_clause {
                fn from_lua<'gc>(ctx: piccolo::Context<'gc>, value: piccolo::Value<'gc>) -> Option<Self> {
                    match value {
                        piccolo::Value::String(s) if s.as_bytes() == b"all" => Some(Self::all()),
                        piccolo::Value::String(s) if s.as_bytes() == b"none" => Some(Self::none()),
                        piccolo::Value::Table(tab) => {
                            let mut ret = Self::none();

                            for (key, val) in tab {
                                match (key, val) {
                                    #(#match_arms),*
                                    _ => None?
                                }
                            }

                            Some(ret)
                        }
                        _ => None,
                    }
                }
            }
        ));
    }
}

#[cfg(test)]
mod tests {
    use runtime_macros::emulate_derive_macro_expansion;
    use super::derive_perm_tree_node_impl;
    use std::{env, fs};

    #[test]
    fn code_coverage() {
        // This code doesn't check much. Instead, it does macro expansion at run time to let
        // tarpaulin measure code coverage for the macro.
        let mut path = env::current_dir().unwrap();
        path.push("tests");
        path.push("lib.rs");
        let file = fs::File::open(path).unwrap();
        emulate_derive_macro_expansion(file, &[("Permission", derive_perm_tree_node_impl)]).unwrap();
    }
}
