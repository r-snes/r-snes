use proc_macro::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::parse_macro_input;
use syn::DeriveInput;

/// Custom derive macro which implements [`std::cmp::PartialOrd`]
///
/// Currently only applies to structs, requires that all struct
/// members implement [`std::cmp::PartialOrd`] already.
///
/// The implementation differs quite much from the standard derive macro:
/// the standard derive macro represents a lexicographical order, whereas
/// this macro represents what we call a *strict order*.
///
/// ---
///
/// ## Ordering logic
///
/// Base principles:
/// - All fields will be compared with their counterpart, relying on the
/// [`PartialOrd`] implementation of the field.
/// - All orderings will be taken into account to determine the resulting ordering.
///
/// Actual logic:
/// - If any of the fields cannot be ordered (e.g. [`partial_cmp`] returned [`None`]
/// at least once), then [`None`] will be returned.
/// - If a struct is greater than the other in all of its fields,
/// it will evaluate greater overall. The same applies the other way around.
/// - If some fields are equal, and at least one is greater, the struct will
/// evaluate greater overall. The same applies the other way around.
/// - If there is any contradiction (at least one greater **and** one lesser), the two
/// structs cannot be ordered; [`None`] will be returned.
/// - Only in the occassion all fields evaluate equal will the structs be recognised equal.
#[proc_macro_derive(PartialOrd)]
pub fn strict_partial_ord(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let item: syn::Item = input.into();
    let syn::Item::Struct(item_struct) = item else {
        panic!("Strict PartialOrd only support struct inputs");
    };

    derive_partial_ord(&item_struct).into()
}

fn derive_partial_ord(input: &syn::ItemStruct) -> proc_macro2::TokenStream {
    let ident = &input.ident;
    let members = input.fields.members();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let acc_varname = format_ident!("{}", "acc");
    let members_match_blocks = members.map(|member| {
        quote! {
            let member_ord = self.#member.partial_cmp(&other.#member)?;
            match (#acc_varname, member_ord) {
                (std::cmp::Ordering::Equal, x) => #acc_varname = x,
                (std::cmp::Ordering::Less, std::cmp::Ordering::Less) => (),
                (std::cmp::Ordering::Greater, std::cmp::Ordering::Greater) => (),
                _ => return None,
            };
        }
    });
    quote! {
        impl #impl_generics std::cmp::PartialOrd for #ident #ty_generics #where_clause {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                let mut #acc_varname = std::cmp::Ordering::Equal;

                #(#members_match_blocks)*

                Some(#acc_varname)
            }
        }
    }
}
