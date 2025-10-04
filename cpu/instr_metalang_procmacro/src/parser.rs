use pm2::{Ident, TokenStream, TokenTree};
use proc_macro2 as pm2;

/// Enum for all the meta instructions implemented for the CPU
/// meta-language.
pub(crate) enum MetaInstruction {
    EndCycle(Ident),
    EndInstr(Ident),
}

impl MetaInstruction {
    /// Method which checks if this instr is a `EndInstr`
    fn is_end_instr(&self) -> bool {
        match self {
            Self::EndInstr(_) => true,
            _ => false,
        }
    }
}

impl MetaInstruction {
    /// Conversion from a Token iterator
    ///
    /// For some reason can't be implemented as a TryFrom trait
    fn try_from<I: IntoIterator<Item = TokenTree>>(value: I) -> Result<Self, &'static str> {
        let mut it = value.into_iter();

        let Some(TokenTree::Ident(meta_kw)) = it.next() else {
            Err("Expecting a meta-keyword")?
        };
        let ret = match meta_kw.to_string().as_str() {
            "END_CYCLE" => {
                let Some(TokenTree::Ident(cyc_type)) = it.next() else {
                    Err("END_CYCLE expects an identifier operand (cycle type)")?
                };
                MetaInstruction::EndCycle(cyc_type)
            }

            "END_INSTR" => {
                let Some(TokenTree::Ident(cyc_type)) = it.next() else {
                    Err("INSTR expects an identifier operand (cycle type)")?
                };
                MetaInstruction::EndInstr(cyc_type)
            }

            _ => Err("Unknown meta-keyword")?,
        };
        if it.next().is_some() {
            Err("Unexpected token after end of meta-instruction")?
        }
        Ok(ret)
    }
}

/// Type resulting from the parsing of the token stream passed
/// as parameter to the [`cpu_instr`] proc macro.
pub(crate) struct Instr {
    pub name: Ident,
    pub cycles: Vec<Cycle>,
}

impl Instr {
    fn new(name: Ident) -> Self {
        Self {
            name,
            cycles: vec![],
        }
    }
}

impl TryFrom<TokenStream> for Instr {
    fn try_from(stream: TokenStream) -> Result<Self, Self::Error> {
        let mut it = stream.into_iter();
        let Some(TokenTree::Ident(name)) = it.next() else {
            Err("Expecting the instruction name")?
        };
        let Some(TokenTree::Group(body)) = it.next() else {
            Err("Expecting the instruction body")?
        };
        if it.next().is_some() {
            Err("Unexpected token after the instruction body")?
        }

        let mut it = body.stream().into_iter().peekable();
        let mut ret = Instr::new(name);
        loop {
            let it = it.by_ref();
            let cycle_body = it.take_while(|token| token.to_string() != "meta");
            let body_tok_stream = TokenStream::from_iter(cycle_body);
            let meta_instr = MetaInstruction::try_from(it.take_while(|token| {
                let TokenTree::Punct(p) = token else {
                    return true;
                };
                return p.as_char() != ';';
            }))?;

            if it.peek().is_none() && !meta_instr.is_end_instr() {
                Err("Instructions must end by an END_INSTR meta instruction")?
            }

            match meta_instr {
                MetaInstruction::EndCycle(c) => ret.cycles.push(Cycle::new(body_tok_stream, c)),
                MetaInstruction::EndInstr(c) => ret.cycles.push(Cycle::new(body_tok_stream, c)),
            }

            if it.peek().is_none() {
                break;
            }
        }
        Ok(ret)
    }
    type Error = &'static str;
}

/// Data structure which contains the info required to build
/// a cycle function body
pub(crate) struct Cycle {
    /// Raw function body
    pub body: TokenStream,
    /// Cycle type (part of the function return value; should evaluate
    /// to something of type `CycleResult`)
    pub cyc_type: Ident,
}

impl Cycle {
    fn new(body: TokenStream, cyc_type: Ident) -> Self {
        Self { body, cyc_type }
    }
}
