use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::DeriveInput;

#[proc_macro_derive(Permission)]
pub fn derive_permission(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let item: syn::Item = input.into();
    let syn::Item::Struct(item_struct) = item else {
        panic!("Permission derive macro only supports structs for now");
    };

    derive_permission_fn(&item_struct).into()
}

fn derive_permission_fn(input: &syn::ItemStruct) -> proc_macro2::TokenStream {
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let members_all_call = input.fields.iter().map(|f| {
        let ident = &f.ident;
        let typ = &f.ty;

        quote! { #ident: #typ::all() }
    });
    let members_none_call = input.fields.iter().map(|f| {
        let ident = &f.ident;
        let typ = &f.ty;

        quote! { #ident: #typ::none() }
    });

    quote! {
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
    }
}
