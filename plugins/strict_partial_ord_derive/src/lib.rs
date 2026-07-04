use {
    proc_macro::TokenStream,
    quote::{
        ToTokens,
        format_ident,
        quote,
    },
    syn::{
        ItemStruct,
        parse::{self, Parse, ParseStream},
    }
};

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
    derive_partial_ord(input.into()).into()
}

fn derive_partial_ord(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    // Parse the annotated item.
    let ast: StrictPartialOrd = match syn::parse2(input) {
        Ok(parsed) => parsed,
        Err(e) => return e.into_compile_error()
    };

    // Return the macro's expanded form (the main logic is in `Pod::to_tokens`).
    let mut ts = proc_macro2::TokenStream::new();
    ast.to_tokens(&mut ts);
    ts
}

struct StrictPartialOrd {
    item: ItemStruct,
}

impl Parse for StrictPartialOrd {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        Ok(Self { item: input.call(ItemStruct::parse)? })
    }
}

impl ToTokens for StrictPartialOrd {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = &self.item.ident;
        let members = self.item.fields.members();
        let (impl_generics, ty_generics, where_clause) = self.item.generics.split_for_impl();

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

        tokens.extend(quote!(
            impl #impl_generics std::cmp::PartialOrd for #ident #ty_generics #where_clause {
                fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                    let mut #acc_varname = std::cmp::Ordering::Equal;

                    #(#members_match_blocks)*

                    Some(#acc_varname)
                }
            }
        ));
    }
}

#[cfg(test)]
mod tests {
    use runtime_macros::emulate_derive_macro_expansion;
    use super::derive_partial_ord;
    use std::{env, fs};

    #[test]
    fn code_coverage() {
        // This code doesn't check much. Instead, it does macro expansion at run time to let
        // tarpaulin measure code coverage for the macro.
        let mut path = env::current_dir().unwrap();
        path.push("tests");
        path.push("lib.rs");
        let file = fs::File::open(path).unwrap();
        emulate_derive_macro_expansion(file, &[("strict::PartialOrd", derive_partial_ord)]).unwrap();
    }
}
