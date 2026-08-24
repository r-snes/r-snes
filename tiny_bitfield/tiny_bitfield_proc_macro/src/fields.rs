use std::{fmt::Display, num::NonZeroU32};

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{ToTokens, quote};

#[derive(PartialEq, Eq, Debug)]
pub struct Field {
    pub name: Ident,
    pub width: NonZeroU32,
    pub retype: Option<BitsType>,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum BitsType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl Display for BitsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Bool => write!(f, "bool"),
            Self::U8 => write!(f, "u8"),
            Self::U16 => write!(f, "u16"),
            Self::U32 => write!(f, "u32"),
            Self::U64 => write!(f, "u64"),
            Self::U128 => write!(f, "u128"),
        }
    }
}

impl ToTokens for BitsType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            Self::Bool => tokens.extend(quote!(bool)),
            Self::U8 => tokens.extend(quote!(u8)),
            Self::U16 => tokens.extend(quote!(u16)),
            Self::U32 => tokens.extend(quote!(u32)),
            Self::U64 => tokens.extend(quote!(u64)),
            Self::U128 => tokens.extend(quote!(u128)),
        }
    }
}

impl TryFrom<u32> for BitsType {
    type Error = ();

    fn try_from(bit_length: u32) -> Result<Self, Self::Error> {
        match bit_length {
            1 => Ok(Self::Bool),
            8 => Ok(Self::U8),
            16 => Ok(Self::U16),
            32 => Ok(Self::U32),
            64 => Ok(Self::U64),
            128 => Ok(Self::U128),
            _ => Err(()),
        }
    }
}

impl TryFrom<Ident> for BitsType {
    type Error = ();

    fn try_from(id: Ident) -> Result<Self, Self::Error> {
        match id.to_string().as_ref() {
            "u8" => Ok(Self::U8),
            "u16" => Ok(Self::U16),
            "u32" => Ok(Self::U32),
            "u64" => Ok(Self::U64),
            "u128" => Ok(Self::U128),
            "bool" => Ok(Self::Bool),
            _ => Err(()),
        }
    }
}

impl BitsType {
    pub fn bits(self) -> u32 {
        match self {
            Self::Bool => 1,
            Self::U8 => 8,
            Self::U16 => 16,
            Self::U32 => 32,
            Self::U64 => 64,
            Self::U128 => 128,
        }
    }

    pub fn cast(self) -> TokenStream {
        match self {
            Self::Bool => quote!(!= 0),
            Self::U8 => quote!(as u8),
            Self::U16 => quote!(as u16),
            Self::U32 => quote!(as u32),
            Self::U64 => quote!(as u64),
            Self::U128 => quote!(as u128),
        }
    }
}

#[derive(PartialEq, Eq, Default)]
pub struct Fields(pub Vec<Field>);

impl Fields {
    pub fn bit_length(&self) -> u32 {
        self.0.iter().fold(0, |acc, f| acc + f.width.get())
    }

    pub fn has_field(&self, id: &Ident) -> bool {
        self.0.iter().any(|f| &f.name == id)
    }

    pub fn has_char(&self, c: char) -> bool {
        self.0.iter().any(|f| f.name == c.to_string())
    }

    pub fn generate_bindings(&self, expr: &TokenStream, ty: BitsType) -> TokenStream {
        let mut rshift = 0;
        let mut decls = quote!();
        let mut assigns = quote!();

        for Field {
            name,
            width,
            retype,
        } in self.0.iter().rev()
        {
            let width = width.get();
            let mask = Literal::u32_unsuffixed(1_u32.strict_shl(width).strict_sub(1));
            let ty = retype.unwrap_or(ty);
            let as_clause = retype.map(BitsType::cast).unwrap_or_default();

            decls.extend(quote!(let #name: #ty;));
            assigns.extend(quote!(
                #name = (((expr) >> #rshift) & #mask) #as_clause;
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

    pub fn extend_one_char(&mut self, c: char) -> Result<(), ()> {
        if let Some(last) = self.0.last_mut()
            && last.name == c.to_string()
        {
            last.width = last.width.saturating_add(1);
        } else {
            if c != '_' && self.has_char(c) {
                return Err(());
            }
            self.0.push(Field {
                name: Ident::new(&c.to_string(), Span::call_site()),
                width: NonZeroU32::new(1).unwrap(),
                retype: None,
            });
        }

        Ok(())
    }
}
