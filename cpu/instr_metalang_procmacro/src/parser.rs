use pm2::{Ident, TokenStream, TokenTree};
use proc_macro2 as pm2;
use quote::quote;

/// Data describing the status of the parser at any point in parsing
#[derive(Copy, Clone, Debug)]
pub(crate) struct ParserStatus {
    /// Whether PC should be automatically incremented
    pub inc_pc: bool,
}

impl Default for ParserStatus {
    fn default() -> Self {
        Self {
            inc_pc: true,
        }
    }
}

impl ParserStatus {
    fn conditionally_inc_pc(&self, inc: u16) -> TokenStream {
        if self.inc_pc {
            quote! {
                cpu.registers.PC = cpu.registers.PC.wrapping_add(#inc);
            }
        } else {
            quote! {}
        }
    }
}

/// Enum for all the meta instructions implemented for the CPU
/// meta-language.
///
/// Notes shared by several meta-instructions:
/// 1. Meta-instrs that have "8" or "16" in their name indicate that
///    they are a variant of an operation on 8-bits/16-bits operands
/// 2. Meta-instructions that read or write to the stack (Pull and Push
///    meta-instructions) all have an "N" (native) variant. This is to
///    account for a weird quirk of pushing/pulling to/from the stack with
///    the 65816: by default (in native mode) the S (stack pointer, points
///    to the current top of the stack) register is 16-bits wide, but in
///    emulation mode it's "generally" only 8-bits wide, with the high-order
///    byte "forced" to 0x01. The high order byte is only set to 0x01 when
///    switching to emulation mode, it is not constantly (nor repeatedly)
///    reset to 0x01. Most instructions that interact with the stack will thus
///    only increment/decrement the low-order byte of S when in emulation
///    mode to "preserve" the forced high byte (note that they would just as
///    well preserve the high byte even it isn't currently 0x01).
///    This is what Pull/Push meta-instructions do by default.
///    However, a few instructions may increment or decrement S past its
///    "forced" 0x01. PullN and PushN meta-instructions allow this behaviour:
///    they will always push/pull as in native mode, (without preserving the
///    high order byte of S in emulation mode)
pub(crate) enum MetaInstruction {
    /// Manually delimit the end of a cycle,
    /// with the CycleResult (cycle type) produced by the token stream
    EndCycle(TokenStream),

    /// Sets the address bus to point at an immediate operand
    /// (right after the opcode)
    SetAddrModeImmediate,

    /// Sets the address bus to point at an absolute operand
    /// (read an address after the opcode and set the addr bus
    /// to point at that address in DB)
    SetAddrModeAbsolute,

    /// Sets the address bus to point at the top of the stack
    SetAddrModeStack,

    /// Creates a read cycle at the current address bus
    /// and assigns the value set in the data into the token
    /// stream passed as parameter in the next cycle.
    ///
    /// `meta FETCH8_INTO <tokstream>;` is strictly equivalent to
    /// `meta END_CYCLE Read; <tokstream> = cpu.data_bus;`
    Fetch8Into(TokenStream),

    /// Fetch two bytes from the current address bus
    /// (and the current address bus + 1) into the u16
    /// contained in <tokstream>
    Fetch16Into(TokenStream),

    /// Fetch a byte from the address at PB:PC+1, and conditionally increment PC
    Fetch8Imm,

    /// Fetch a byte from the address at PB:PC+1, and conditionally increment PC,
    /// and assign into <tokstream>, similar to [`Fetch8Into`]
    Fetch8ImmInto(TokenStream),

    /// Fetch two bytes at PB:PC+1 (and PB:PC+2) into the u16 contained
    /// in <tokstream>, and conditionally increment PC by two
    Fetch16ImmInto(TokenStream),

    /// Read a byte from the top of the stack, and update the stack pointer
    Pull8,

    /// Read a byte from the top of the stack, and update the stack pointer
    ///
    /// See note 2 for differences with Pull8
    PullN8,

    /// Read a u8 from the top of the stack, store it in the token
    /// stream pointed to by <tokstream>, and update the stack pointer
    Pull8Into(TokenStream),

    /// Read a u8 from the top of the stack, store it in the token
    /// stream pointed to by <tokstream>, and update the stack pointer
    ///
    /// See note 2 for differences with Pull8Into
    PullN8Into(TokenStream),

    /// Read a u16 from the top of the stack, store it in the token
    /// stream pointed to by <tokstream>, and update the stack pointer
    Pull16Into(TokenStream),

    /// Read a u16 from the top of the stack, store it in the token
    /// stream pointed to by <tokstream>, and update the stack pointer
    ///
    /// See note 2 for differences with Pull16Into
    PullN16Into(TokenStream),

    /// Write the u8 stored in <tokstream> at the current address bus
    Write8(TokenStream),

    /// Write the u16 stored in <tokstream> at the current address bus
    Write16(TokenStream),

    /// Write the u8 stored in <tokenstream> at the top of the stack,
    /// and update the stack pointer
    Push8(TokenStream),

    /// Write the u8 stored in <tokenstream> at the top of the stack,
    /// and update the stack pointer
    ///
    /// See note 2 for differences with Push8
    PushN8(TokenStream),

    /// Write the u16 stored in <tokenstream> at the top of the stack,
    /// and update the stack pointer
    Push16(TokenStream),

    /// Write the u16 stored in <tokenstream> at the top of the stack,
    /// and update the stack pointer
    ///
    /// See note 2 for differences with Push16
    PushN16(TokenStream),
}

impl MetaInstruction {
    /// Conversion from a Token iterator  
    ///
    /// The input [`value`] contains all tokens between (excluding)
    /// the `meta` identifier (which indicates the start of a meta-instruction)
    /// and the semicolon (which indicates the end of the meta-instruction)
    ///
    /// For some reason can't be implemented as a TryFrom trait
    #[cfg(not(tarpaulin_include))]
    fn try_from<I: IntoIterator<Item = TokenTree>>(value: I) -> Result<Self, &'static str> {
        let mut it = value.into_iter();

        let Some(TokenTree::Ident(meta_kw)) = it.next() else {
            Err("Expecting a meta-keyword")?
        };
        let ret = match meta_kw.to_string().as_str() {
            "END_CYCLE" => MetaInstruction::EndCycle(it.by_ref().collect()),

            "SET_ADDRMODE_IMM" => MetaInstruction::SetAddrModeImmediate,
            "SET_ADDRMODE_ABS" => MetaInstruction::SetAddrModeAbsolute,
            "SET_ADDRMODE_STACK" => MetaInstruction::SetAddrModeStack,

            "FETCH8_INTO" => MetaInstruction::Fetch8Into(it.by_ref().collect()),
            "FETCH16_INTO" => MetaInstruction::Fetch16Into(it.by_ref().collect()),

            "FETCH8_IMM" => MetaInstruction::Fetch8Imm,
            "FETCH8_IMM_INTO" => MetaInstruction::Fetch8ImmInto(it.by_ref().collect()),
            "FETCH16_IMM_INTO" => MetaInstruction::Fetch16ImmInto(it.by_ref().collect()),

            "PULL8" => MetaInstruction::Pull8,
            "PULLN8" => MetaInstruction::PullN8,
            "PULL8_INTO" => MetaInstruction::Pull8Into(it.by_ref().collect()),
            "PULLN8_INTO" => MetaInstruction::PullN8Into(it.by_ref().collect()),
            "PULL16_INTO" => MetaInstruction::Pull16Into(it.by_ref().collect()),
            "PULLN16_INTO" => MetaInstruction::PullN16Into(it.by_ref().collect()),

            "WRITE8" => MetaInstruction::Write8(it.by_ref().collect()),
            "WRITE16" => MetaInstruction::Write16(it.by_ref().collect()),

            "PUSH8" => MetaInstruction::Push8(it.by_ref().collect()),
            "PUSHN8" => MetaInstruction::PushN8(it.by_ref().collect()),
            "PUSH16" => MetaInstruction::Push16(it.by_ref().collect()),
            "PUSHN16" => MetaInstruction::PushN16(it.by_ref().collect()),

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
    #[cfg(not(tarpaulin_include))]
    fn expand(self, pstatus: &mut ParserStatus) -> InstrBody {
        let mut ret = InstrBody::default();

        match self {
            Self::EndCycle(cyctype) => {
                ret.cycles = vec![Cycle::new(TokenStream::new(), cyctype)];
            }

            Self::SetAddrModeImmediate => {
                let conditional_incpc = if !pstatus.inc_pc {
                    // we need to look at PC+1 if it wasn't auto-incremented
                    // to go to the operand
                    quote! { .wrapping_add(1) }
                } else {
                    quote! {}
                };

                ret.post_instr = quote! {
                    cpu.addr_bus.bank = cpu.registers.PB;
                    cpu.addr_bus.addr = cpu.registers.PC #conditional_incpc;
                }
            }
            Self::SetAddrModeAbsolute => {
                // start by fetching the address at which we'll be reading/writing
                ret = Self::Fetch16ImmInto(quote! { cpu.internal_data_bus }).expand(pstatus);
                // then set the addr bus accordingly
                ret += InstrBody::post(quote! {
                    cpu.addr_bus.addr = cpu.internal_data_bus;
                    cpu.addr_bus.bank = cpu.registers.DB;
                });
            }
            Self::SetAddrModeStack => {
                ret = InstrBody::post(quote! {
                    cpu.addr_bus.addr = cpu.registers.S;
                    cpu.addr_bus.bank = 0;
                });
            }

            Self::Fetch8Into(dest) => {
                ret = Self::EndCycle(quote! { Read }).expand(pstatus);
                ret.post_instr = quote! { #dest = cpu.data_bus; };
            }
            Self::Fetch16Into(into) => {
                ret = Self::Fetch8Into(quote! { *#into.lo_mut() }).expand(pstatus);
                ret += InstrBody::post(quote! {
                    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(1);
                });
                ret += Self::Fetch8Into(quote! { *#into.hi_mut() }).expand(pstatus);
            }

            Self::Fetch8Imm => {
                ret = Self::SetAddrModeImmediate.expand(pstatus);
                ret += InstrBody::cycles(vec![Cycle::new(
                    pstatus.conditionally_inc_pc(1),
                    quote! { Read },
                )]);
            }
            Self::Fetch8ImmInto(into) => {
                ret = Self::Fetch8Imm.expand(pstatus);
                ret += InstrBody::post(quote! { #into = cpu.data_bus; });
            }
            Self::Fetch16ImmInto(into) => {
                ret = Self::SetAddrModeImmediate.expand(pstatus);
                ret += InstrBody::post(pstatus.conditionally_inc_pc(2));
                ret += Self::Fetch16Into(into).expand(pstatus);
            }

            Self::Pull8 => {
                ret = InstrBody::post(quote! {
                    // stack grows downwards; only set low byte in emu mode
                    if cpu.registers.P.E {
                        *cpu.registers.S.lo_mut() = cpu.registers.S.lo().wrapping_add(1);
                    } else {
                        cpu.registers.S = cpu.registers.S.wrapping_add(1);
                    }
                });
                ret += Self::SetAddrModeStack.expand(pstatus);
                ret += Self::EndCycle(quote! { Read }).expand(pstatus);
            }
            Self::PullN8 => {
                ret = InstrBody::post(quote! {
                    // stack grows downwards
                    cpu.registers.S = cpu.registers.S.wrapping_add(1);
                });
                ret += Self::SetAddrModeStack.expand(pstatus);
                ret += Self::EndCycle(quote! { Read }).expand(pstatus);
            }
            Self::Pull8Into(into) => {
                ret = Self::Pull8.expand(pstatus);
                ret += InstrBody::post(quote! { #into = cpu.data_bus; });
            }
            Self::PullN8Into(into) => {
                ret = Self::PullN8.expand(pstatus);
                ret += InstrBody::post(quote! { #into = cpu.data_bus; });
            }
            Self::Pull16Into(into) => {
                // pulls read the low byte first
                ret = Self::Pull8Into(quote! { *#into.lo_mut() }).expand(pstatus);
                ret += Self::Pull8Into(quote! { *#into.hi_mut() }).expand(pstatus);
            }
            Self::PullN16Into(into) => {
                // pulls read the low byte first
                ret = Self::PullN8Into(quote! { *#into.lo_mut() }).expand(pstatus);
                ret += Self::PullN8Into(quote! { *#into.hi_mut() }).expand(pstatus);
            }

            Self::Write8(data) => {
                ret.cycles = vec![Cycle::new(
                    quote! {
                        cpu.data_bus = #data;
                    },
                    quote! { Write }
                )];
            }
            Self::Write16(data) => {
                ret += Self::Write8(quote! { *#data.lo() }).expand(pstatus);
                ret += InstrBody::post(quote! {
                    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(1);
                });
                ret += Self::Write8(quote! { *#data.hi() }).expand(pstatus);
            }

            Self::Push8(data) => {
                ret = Self::SetAddrModeStack.expand(pstatus);
                ret += InstrBody::post(quote! {
                    // stack grows downwards; only set low byte in emu mode
                    if cpu.registers.P.E {
                        *cpu.registers.S.lo_mut() = cpu.registers.S.lo().wrapping_sub(1);
                    } else {
                        cpu.registers.S = cpu.registers.S.wrapping_sub(1);
                    }
                });
                ret += Self::Write8(data).expand(pstatus);
            }
            Self::PushN8(data) => {
                ret = Self::SetAddrModeStack.expand(pstatus);
                // stack grows downwards
                ret += InstrBody::post(quote! {
                    cpu.registers.S = cpu.registers.S.wrapping_sub(1);
                });

                ret += Self::Write8(data).expand(pstatus);
            }
            Self::Push16(data) => {
                // pushes write the high byte first
                ret = Self::Push8(quote! { *#data.hi() }).expand(pstatus);
                ret += Self::Push8(quote! { *#data.lo() }).expand(pstatus);
            }
            Self::PushN16(data) => {
                // pushes write the high byte first
                ret = Self::PushN8(quote! { *#data.hi() }).expand(pstatus);
                ret += Self::PushN8(quote! { *#data.lo() }).expand(pstatus);
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
        Self {
            name,
            body: InstrBody::default(),
        }
    }

    pub fn parse(stream: TokenStream, inc_pc: bool) -> Result<Self, &'static str> {
        let mut pstatus = ParserStatus::default();
        pstatus.inc_pc = inc_pc;

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

        ret.body += InstrBody::post(pstatus.conditionally_inc_pc(1));
        loop {
            let it = it.by_ref();

            ret.body
                .post_instr
                .extend(it.take_while(|token| token.to_string() != "meta"));

            if it.peek().is_none() {
                break;
            }

            let meta_instr = MetaInstruction::try_from(it.take_while(|token| {
                let TokenTree::Punct(p) = token else {
                    return true;
                };
                return p.as_char() != ';';
            }))?;

            ret.body += meta_instr.expand(&mut pstatus);
        }
        Ok(ret)
    }
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
