use std::fmt::Display;

use proc_macro2::{Ident, TokenTree};

use crate::{BitsType, Fields};

#[derive(Default)]
pub struct RenameRetypeGroup(pub Vec<RenameRetypeField>);

impl RenameRetypeGroup {
    pub fn from_tokens(toks: impl Iterator<Item = TokenTree>, fields: &Fields) -> Self {
        let mut toks = toks.peekable();
        let mut ret = Self::default();

        while toks.peek().is_some() {
            let one_rename = toks
                .by_ref()
                .take_while(|t| !matches!(t, TokenTree::Punct(t) if t.as_char() == ';'));
            let one_rename = RenameRetypeField::from_tokens(one_rename);
            let target = &one_rename.target;

            if !fields.has_field(target) {
                panic!("rename/retype `{one_rename}` targets {target} which is not a field");
            }
            if ret.has_target(target) {
                panic!(
                    "rename/retype `{one_rename}` targets {target} which is already renamed/retyped"
                );
            }
            ret.0.push(one_rename);
        }
        ret
    }

    pub fn has_target(&self, target: &Ident) -> bool {
        self.0.iter().any(|f| &f.target == target)
    }

    pub fn apply_to(&self, fields: &mut Fields) {
        for rename in &self.0 {
            let target = &rename.target;

            for field in &mut fields.0 {
                if &field.name != target {
                    continue;
                }

                if let Some(retype) = rename.retype {
                    if retype.bits() < field.width.get() {
                        panic!(
                            "retype for {} is too small ({} bits, but bitfield is {} bits)",
                            field.name,
                            retype.bits(),
                            field.width
                        )
                    }
                    field.retype = Some(retype);
                }
                if let Some(rename) = &rename.rename {
                    field.name = rename.clone();
                }
            }
        }
    }
}

pub struct RenameRetypeField {
    /// The thing to rename/retype
    pub target: Ident,

    pub retype: Option<BitsType>,
    pub rename: Option<Ident>,
}

impl Display for RenameRetypeField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let target = &self.target;
        match (&self.rename, &self.retype) {
            (Some(rename), Some(retype)) => {
                write!(f, "{rename}: {retype} = {target}")
            }
            (None, Some(retype)) => {
                write!(f, "{target}: {retype}")
            }
            (Some(rename), None) => {
                write!(f, "{rename} = {target}")
            }
            (None, None) => {
                write!(f, "{target}")
            }
        }
    }
}

impl RenameRetypeField {
    pub fn from_tokens(mut toks: impl Iterator<Item = TokenTree>) -> Self {
        match (
            toks.next(),
            toks.next(),
            toks.next(),
            toks.next(),
            toks.next(),
        ) {
            // rename only (foo = f)
            (
                Some(TokenTree::Ident(to)),
                Some(TokenTree::Punct(eq)),
                Some(TokenTree::Ident(from)),
                None,
                _,
            ) if eq.as_char() == '=' => Self {
                target: from,
                rename: Some(to),
                retype: None,
            },

            // retype only (f: u8)
            (
                Some(TokenTree::Ident(field)),
                Some(TokenTree::Punct(colon)),
                Some(TokenTree::Ident(retype)),
                None,
                _,
            ) if colon.as_char() == ':' => Self {
                target: field,
                rename: None,
                retype: Some(BitsType::try_from(retype).expect("Invalid type")),
            },

            // retype and rename (f: u8 = foo)
            (
                Some(TokenTree::Ident(field)),
                Some(TokenTree::Punct(colon)),
                Some(TokenTree::Ident(retype)),
                Some(TokenTree::Punct(eq)),
                Some(TokenTree::Ident(from)),
            ) if colon.as_char() == ':' && eq.as_char() == '=' => {
                let None = toks.next() else {
                    panic!(
                        "expected end of stream in rename/retype `{}: {} = {}`",
                        field, retype, from
                    );
                };
                Self {
                    target: from,
                    rename: Some(field),
                    retype: Some(BitsType::try_from(retype).expect("Invalid type")),
                }
            }

            (t1, t2, t3, t4, t5) => {
                panic!("can't read rename [{t1:?} {t2:?} {t3:?} {t4:?} {t5:?}]");
            }
        }
    }
}
