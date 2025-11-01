use pm2::{Ident, TokenStream, TokenTree};
use proc_macro2 as pm2;
use quote::quote;

/// Enum for all the meta instructions implemented for the CPU
/// meta-language.
pub(crate) enum MetaInstruction {
    /// Manually delimit the end of a cycle,
    /// with the CycleResult (cycle type) produced by the token stream
    EndCycle(TokenStream),

    /// Sets the address bus to point at an immediate operand
    /// (right after the opcode)
    SetAddrModeImmediate,

    /// Creates a read cycle at the current address bus
    /// and assigns the value set in the data into the token
    /// stream passed as parameter in the next cycle.
    ///
    /// `meta FETCH8_INTO <tokstream>;` is strictly equivalent to
    /// `meta END_CYCLE Read; <tokstream> = cpu.data_bus;`
    Fetch8Into(TokenStream),

    /// Fetch a byte from the address at PB:PC
    Fetch8ImmNoIncPC,

    /// Fetch a byte from the address at PB:PC, and increment PC
    Fetch8Imm,

    /// Fetch a byte from the address at PB:PC,
    /// and assign into <tokstream>, similar to [`Fetch8Into`]
    Fetch8ImmNoIncPCInto(TokenStream),

    /// Fetch a byte from the address at PB:PC, and increment PC,
    /// and assign into <tokstream>, similar to [`Fetch8Into`]
    Fetch8ImmInto(TokenStream),

    /// Fetch two bytes from the current address bus
    /// (and the current address bus + 1) into the u16
    /// contained in <tokstream>
    Fetch16Into(TokenStream),

    /// Fetch two bytes at PB:PC (and PB:PC + 1) into
    /// the u16 contained in <tokstream>
    Fetch16ImmNoIncPCInto(TokenStream),

    /// Fetch two bytes at PB:PC (and PB:PC + 1) into
    /// the u16 contained in <tokstream>, and increment PC by two
    Fetch16ImmInto(TokenStream),
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
            "SET_ADDRMODE_IMM" => MetaInstruction::SetAddrModeImmediate,
            "FETCH8_INTO" => MetaInstruction::Fetch8Into(it.by_ref().collect()),
            "FETCH8_IMM_NOINCPC" => MetaInstruction::Fetch8ImmNoIncPC,
            "FETCH8_IMM" => MetaInstruction::Fetch8Imm,
            "FETCH8_IMM_NOINCPC_INTO" => MetaInstruction::Fetch8ImmNoIncPCInto(it.by_ref().collect()),
            "FETCH8_IMM_INTO" => MetaInstruction::Fetch8ImmInto(it.by_ref().collect()),
            "FETCH16_INTO" => MetaInstruction::Fetch16Into(it.by_ref().collect()),
            "FETCH16_IMM_NOINCPC_INTO" => MetaInstruction::Fetch16ImmNoIncPCInto(it.by_ref().collect()),
            "FETCH16_IMM_INTO" => MetaInstruction::Fetch16ImmInto(it.by_ref().collect()),

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
    fn expand(self) -> InstrBody {
        let mut ret = InstrBody::default();

        match self {
            Self::EndCycle(cyctype) => {
                ret.cycles = vec![Cycle::new(TokenStream::new(), cyctype)];
            }
            Self::SetAddrModeImmediate => {
                ret.post_instr = quote!{
                    cpu.addr_bus.bank = cpu.registers.PB;
                    cpu.addr_bus.addr = cpu.registers.PC;
                }
            }
            Self::Fetch8Into(dest) => {
                ret.cycles = vec![Cycle::new(TokenStream::new(), quote! { Read })];
                ret.post_instr = quote!{ #dest = cpu.data_bus; };
            }
            Self::Fetch8ImmNoIncPC => {
                ret = Self::SetAddrModeImmediate.expand();
                ret += InstrBody::cycles(vec![Cycle::new( quote! {}, quote! { Read })]);
            }
            Self::Fetch8Imm => {
                ret = Self::Fetch8ImmNoIncPC.expand();
                ret.cycles[0].body.extend(quote! {
                    cpu.registers.PC = cpu.registers.wrapping_add(1);
                });
            }
            Self::Fetch8ImmNoIncPCInto(into) => {
                ret = Self::Fetch8ImmNoIncPC.expand();
                ret += InstrBody::post(quote! {
                    #into = cpu.data_bus;
                });
            }
            Self::Fetch8ImmInto(into) => {
                ret = Self::Fetch8Imm.expand();
                ret += InstrBody::post(quote! {
                    #into = cpu.data_bus;
                });
            }
            Self::Fetch16Into(into) => {
                ret = Self::Fetch8Into(quote! { *#into.lo_mut() }).expand();
                ret += InstrBody::post(quote! {
                    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(1);
                });
                ret += Self::Fetch8Into(quote! { *#into.hi_mut() }).expand();
            }
            Self::Fetch16ImmNoIncPCInto(into) => {
                ret = Self::SetAddrModeImmediate.expand();
                ret += Self::Fetch16Into(into).expand();
            }
            Self::Fetch16ImmInto(into) => {
                ret = Self::Fetch16ImmNoIncPCInto(into).expand();
                ret += InstrBody::post(quote! {
                    cpu.registers.PC = cpu.registers.PC.wrapping_add(2);
                });
            }
        }
        ret
    }
}

/// Type resulting from the parsing of the token stream passed
/// as parameter to the [`cpu_instr`] proc macro.
pub(crate) struct Instr {
    /// Name of the instruction (e.g. clv, inx, ldx_imm, jmp_abs)
    pub name: Ident,

    /// Body of the instruction, incuding potential post-instr code
    pub body: InstrBody,
}

impl Instr {
    fn new(name: Ident) -> Self {
        Self { name, body: InstrBody::default() }
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

            ret.body.post_instr.extend(it.take_while(|token| token.to_string() != "meta"));

            if it.peek().is_none() {
                break;
            }

            let meta_instr = MetaInstruction::try_from(it.take_while(|token| {
                let TokenTree::Punct(p) = token else {
                    return true;
                };
                return p.as_char() != ';';
            }))?;

            ret.body += meta_instr.expand();
        }
        Ok(ret)
    }

    type Error = &'static str;
}

/// Body of an instruction: one or more cycles and potential
/// post-instruction code
#[derive(Default)]
pub(crate) struct InstrBody {
    /// Cycles of the instruction (does not include the opcode fetch cycle)
    pub cycles: Vec<Cycle>,

    /// "Post-instruction" code: some code related to this instruction
    /// which needs to be run at the start of the next instruction.
    ///
    /// This is typically required when the last cycle of an
    /// instruction is a Read cycle, but the instruction needs to do
    /// something with the read value (e.g. placing it in a register).
    /// The problem is that the value will be injected between cycles, and
    /// will only be available at the start of the next cycle. So this code
    /// will be run at the beginning of the next opcode fetch cycle.
    pub post_instr: TokenStream,
}

// impl which allows us to use +=
impl std::ops::AddAssign for InstrBody {
    /// Concatenates an InstrBody to [`self`].
    /// [`self`] remains first after concatenation. [`self.post_instr`] will be
    /// prepended to the RHS's first cycle, or to the RHS's post_instr
    /// if it doesn't have any cycles.
    fn add_assign(&mut self, mut other: Self) {
        if other.cycles.is_empty() {
            // simple case: no cycle vec to merge, just join the post_instrs
            self.post_instr.extend(other.post_instr);
            return;
        }

        // swap out self.post_instr with other.post_instr, such that
        // self.post_instr can be worked with later, and placing
        // other.post_instr already where it needs to be in the end
        let old_postinstr = std::mem::replace(&mut self.post_instr, other.post_instr);

        // swap out the first cycle body of the other InstrBody for our current
        // post_instr, and then glue it back *after* our current post_instr
        let second_firstcycle = std::mem::replace(&mut other.cycles[0].body, old_postinstr);
        other.cycles[0].body.extend(second_firstcycle);

        // finally merge the two cycle vectors
        self.cycles.extend(other.cycles);
    }
}

impl InstrBody {
    pub fn new(cycles: Vec<Cycle>, post_instr: TokenStream) -> Self {
        Self { cycles, post_instr }
    }

    pub fn cycles(cycles: Vec<Cycle>) -> Self {
        Self::new(cycles, TokenStream::new())
    }

    pub fn post(post_instr: TokenStream) -> Self {
        Self::new(vec![], post_instr)
    }
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
