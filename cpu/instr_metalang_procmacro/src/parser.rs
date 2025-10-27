use pm2::{Ident, TokenStream, TokenTree};
use proc_macro2 as pm2;

/// Enum for all the meta instructions implemented for the CPU
/// meta-language.
pub(crate) enum MetaInstruction {
    /// Manually delimit the end of a cycle,
    /// with the CycleResult (cycle type) produced by the token stream
    EndCycle(TokenStream),
}

impl MetaInstruction {
    /// Conversion from a Token iterator  
    ///
    /// The input [`value`] contains all tokens between (excluding)
    /// the `meta` identifier (which indicates the start of a meta-instruction)
    /// and the semicolon (which indicates the end of the meta-instruction)
    ///
    /// For some reason can't be implemented as a TryFrom trait
    fn try_from<I: IntoIterator<Item = TokenTree>>(value: I) -> Result<Self, &'static str> {
        let mut it = value.into_iter();

        let Some(TokenTree::Ident(meta_kw)) = it.next() else {
            Err("Expecting a meta-keyword")?
        };
        let ret = match meta_kw.to_string().as_str() {
            "END_CYCLE" => MetaInstruction::EndCycle(it.by_ref().collect()),

            _ => Err("Unknown meta-keyword")?,
        };
        if it.next().is_some() {
            Err("Unexpected token after end of meta-instruction")?
        }
        Ok(ret)
    }

    /// Expands this meta-instruction: given the current cycle body,
    /// the meta-instruction may return 0 to many cycles and the next
    /// "current" instruction body.
    ///
    /// Some meta-instrucions may not return complete cycles, and simply append to
    /// the token stream passed as input. Others may consume the current token stream
    /// and return it in a new cycle, and optionnally add more cycles.
    /// It is also possible that a meta-instruction both expands to 1 or more cycles
    /// and returns the body of the following (net yet complete) cycle
    fn expand(self, current_cyc_body: TokenStream) -> (Vec<Cycle>, TokenStream) {
        let cycles;
        let ts;

        match self {
            Self::EndCycle(cyctype) => {
                cycles = vec![Cycle::new(current_cyc_body, cyctype)];
                ts = TokenStream::new();
            }
        }
        (cycles, ts)
    }
}

/// Type resulting from the parsing of the token stream passed
/// as parameter to the [`cpu_instr`] proc macro.
pub(crate) struct Instr {
    /// Name of the instruction (e.g. clv, inx, ldx_imm, jmp_abs)
    pub name: Ident,

    /// Cycles of the instruction (does not include the opcode fetch cycle)
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
        let mut current_cyc_body = TokenStream::new();
        loop {
            let it = it.by_ref();

            current_cyc_body.extend(it.take_while(|token| token.to_string() != "meta"));

            let meta_instr = MetaInstruction::try_from(it.take_while(|token| {
                let TokenTree::Punct(p) = token else {
                    return true;
                };
                return p.as_char() != ';';
            }))?;

            let (cycs, new_body) = meta_instr.expand(current_cyc_body);

            ret.cycles.extend(cycs);
            current_cyc_body = new_body;

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
    pub cyc_type: TokenStream,
}

impl Cycle {
    fn new(body: TokenStream, cyc_type: TokenStream) -> Self {
        Self { body, cyc_type }
    }
}
