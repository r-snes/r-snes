use {
    proc_macro::TokenStream,
    quote::{
        ToTokens,
        quote,
    },
    syn::{
        ItemStruct,
        parse::{self, Parse, ParseStream},
    }
};

#[proc_macro_derive(Permission)]
pub fn derive_permission(input: TokenStream) -> TokenStream {
    derive_permission_impl(input.into()).into()
}

fn derive_permission_impl(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    // Parse the annotated item.
    let ast: PermDerive = match syn::parse2(input) {
        Ok(parsed) => parsed,
        Err(e) => return e.into_compile_error()
    };

    // Return the macro's expanded form (the main logic is in `Pod::to_tokens`).
    let mut ts = proc_macro2::TokenStream::new();
    ast.to_tokens(&mut ts);
    ts
}

struct PermDerive {
    item: ItemStruct,
}

impl Parse for PermDerive {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        Ok(Self { item: input.call(ItemStruct::parse)? })
    }
}

impl ToTokens for PermDerive {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = &self.item.ident;
        let (impl_generics, ty_generics, where_clause) = self.item.generics.split_for_impl();

        let members_all_call = self.item.fields.iter().map(|f| {
            let ident = &f.ident;
            let typ = &f.ty;

            quote! { #ident: #typ::all() }
        });
        let members_none_call = self.item.fields.iter().map(|f| {
            let ident = &f.ident;
            let typ = &f.ty;

            quote! { #ident: #typ::none() }
        });

        tokens.extend(quote!(
            impl #impl_generics Permission for #ident #ty_generics #where_clause {
                fn all() -> Self {
                    Self {
                        #(#members_all_call),*
                    }
                }

                fn none() -> Self {
                    Self {
                        #(#members_none_call),*
                    }
                }
            }
        ));
    }
}

#[cfg(test)]
mod tests {
    use runtime_macros::emulate_derive_macro_expansion;
    use super::derive_permission_impl;
    use std::{env, fs};

    #[test]
    fn code_coverage() {
        // This code doesn't check much. Instead, it does macro expansion at run time to let
        // tarpaulin measure code coverage for the macro.
        let mut path = env::current_dir().unwrap();
        path.push("tests");
        path.push("lib.rs");
        let file = fs::File::open(path).unwrap();
        emulate_derive_macro_expansion(file, &[("Permission", derive_permission_impl)]).unwrap();
    }
}
