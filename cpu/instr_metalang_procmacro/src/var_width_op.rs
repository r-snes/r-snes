use quote::{quote, format_ident};
use proc_macro2::{TokenTree, TokenStream, Ident};
use syn::{ItemFn};

pub struct Binding {
    pub name: Ident,
    pub value: TokenStream,
}

impl Binding {
    pub fn parse(ts: TokenStream) -> Self {
        let mut it = ts.into_iter();

        let Some(TokenTree::Ident(name)) = it.next() else {
            panic!("Expecting identifier name in binding");
        };
        match it.next() {
            Some(TokenTree::Punct(p)) if p.as_char() == '=' => (),
            _ => panic!("Expecting '=' for binding a value"),
        }
        let value = it.collect();

        Self { name, value }
    }

    pub fn as16(&self) -> TokenStream {
        let &Self { ref name, ref value } = self;
        quote! {
            let #name: &mut u16 = &mut #value;
        }
    }

    pub fn as8(&self) -> TokenStream {
        let &Self { ref name, ref value } = self;
        quote! {
            let #name: &mut u8 = (#value).lo_mut();
        }
    }
}

pub fn var_width_op_impl(attr: TokenStream, func: ItemFn) -> TokenStream {
    let mut func16 = func.clone();
    let mut func8 = func;
    let funcname = &func8.sig.ident;

    func16.sig.ident = format_ident!("{funcname}16");
    func8.sig.ident = format_ident!("{funcname}8");

    let mut attr_it = attr.into_iter().peekable();
    let mut bindings: Vec<Binding> = Vec::new();
    while attr_it.peek().is_some() {
        let binding_ts = attr_it
            .by_ref()
            .take_while(|token| token.to_string() != ",").collect::<TokenStream>();
        bindings.push(Binding::parse(binding_ts));
    }

    func16.block.stmts = bindings
        .iter()
        .map(|b| syn::parse2::<syn::Stmt>(b.as16()).unwrap())
        .chain(func16.block.stmts.into_iter())
        .collect::<Vec<syn::Stmt>>();
    func8.block.stmts = bindings
        .iter()
        .map(|b| syn::parse2::<syn::Stmt>(b.as8()).unwrap())
        .chain(func8.block.stmts.into_iter())
        .collect::<Vec<syn::Stmt>>();

    quote! {
        #func16
        #func8
    }
}
