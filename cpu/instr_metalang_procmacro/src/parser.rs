use pm2::{Ident, TokenStream, TokenTree};
use proc_macro2 as pm2;
use quote::quote;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub(crate) enum OpSize {
    /// Constant operand size
    Constant,

    /// Operand size is either 8 or 16 bits, depending
    /// on the state of the M flag (width of the A register)
    AccMem,

    /// Operand size is either 8 or 16 bits, depending
    /// on the state of the X flag (width of X and Y registers)
    Index,
}

/// Data describing the status of the parser at any point in parsing
pub(crate) struct ParserStatus {
    /// Whether PC should be automatically incremented
    pub inc_pc: bool,

    /// Current position of the address bus. In some cases this can help
    /// avoiding some recalculations when switch from common addressing modes,
    /// for example: setting the addr mode to immediate at the start of an
    /// instr should only require increment the addr bus by 1 since it must
    /// point at the opcode (which is 1 before) initially.
    pub addrmode: AddrBusPosition,

    /// Offset from the PC to the next immediate operand.
    /// Each time an immediate byte is read, increment this so that independent
    /// calls to FetchImm can avoid reading the same byte multiple times.
    ///
    /// Also used to calculate the final PC increment to generate at the end
    /// of the instruction.
    pub imm_offset: VarWidth<u16>,

    /// Size of read/written operands of the instruction
    pub operand_size: OpSize,
}

#[derive(PartialEq, Eq)]
pub(crate) enum AddrBusPosition {
    /// The addr bus might point at any location in memory
    Unaligned,

    /// The addr bus points at the opcode of the current instruction
    Opcode,

    /// The addr bus points at the next immediate operand
    Immediate,
}

impl Default for ParserStatus {
    fn default() -> Self {
        Self {
            inc_pc: true,
            addrmode: AddrBusPosition::Opcode, // at instr start, addrbus is on PC
            imm_offset: VarWidth::constw(1), // at instr start, the first imm value is 1 after PC
            operand_size: OpSize::Constant,
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

    /// Sets the operand size for variable width instructions
    SetOperandSize(TokenTree),

    /// Spend an internal cycle idling if the tokenstream evaluates to true
    IdleIf(TokenStream),

    /// Sets the address bus to point at an immediate operand
    /// (right after the opcode)
    SetAddrModeImmediate,

    /// Sets the address bus to point at an absolute operand
    /// (read an address after the opcode and set the addr bus
    /// to point at that address in DB)
    SetAddrModeAbsolute,

    /// Sets the address bus to point at an absolute long operand
    /// (read and address and bank, set addr bus to this)
    SetAddrModeAbsoluteLong,

    /// Sets the address bus to point at an absolute long X-indexed operand
    /// (same as abs long, but add X to the address)
    SetAddrModeAbsLongX,

    /// Sets the address bus to point at an absolute X-indexed operand
    /// (same as absolute, but also add X to the address)
    SetAddrModeAbsoluteX,

    /// Sets the address bus to point at an absolute Y-indexed operand
    /// (same as absolute, but also add Y to the address)
    SetAddrModeAbsoluteY,

    /// Sets the address bus to point at a direct operand
    /// (read an offset, then set addr bus to 0:D+offset
    SetAddrModeDirect,

    /// Sets the address bus to point at a direct X-indexed indirect operand
    /// (read direct offset, then read addr from 0:D+DO+X, then addr bus is DB:AA)
    SetAddrModeDirectXIndirect,

    /// Sets the address bus to point at a direct indirect operand
    /// (read direct offset, then read addr from 0:D+DO; then addr bus is DB:AA)
    SetAddrModeDirectIndirect,

    /// Sets the address bus to point at a direct indirect Y-indexed operand
    /// (read direct offset, then read addr from 0:D+DO; then addr bus is DB:AA+Y)
    /// Contrary to X-indexing, the indexing is done on the final address, not on
    /// the indirect one
    SetAddrModeDirectIndirectY,

    /// Same as DirectIndirectY, but also read a bank after the final address, which
    /// is the the bank number in the final addr bus
    SetAddrModeDirectIndirectLongY,

    /// Same as DirectIndirect, but also read a bank number after the
    /// final address
    SetAddrModeDirectIndirectLong,

    /// Same as SetAddrModeDirect, but add X to the address
    SetAddrModeDirectX,

    /// Same as SetAddrModeDirect, but add Y to the address
    SetAddrModeDirectY,

    /// Sets the address bus to point at the top of the stack
    SetAddrModeStack,

    /// Sets the address bus to point at a stack-relative operand
    /// (read an immediate stack offset to add to S in bank 0)
    SetAddrModeStackRelative,

    /// Sets the address bus to point at a stack-relative operand
    /// (same as stack relative, then deref this operand in DB, Y-indexed)
    SetAddrModeStackRelativeIndirectY,

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

    /// Fetches the operand of the instruction
    /// (variable width must be set with SetOperandSize)
    FetchOperandInto(TokenStream),

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

    /// Writes the operand of the instruction
    /// (variable width must be set with SetOperandSize)
    WriteOperand(TokenStream),

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

    /// Sets the CPU flags N and Z for an 8-bit value
    SetNZ8(TokenStream),

    /// Sets the CPU flags N and Z for an 16bit value
    SetNZ16(TokenStream),

    /// Sets the CPU flags N and Z for a variable width value
    SetNZOperand(TokenStream),
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

            "SET_OP_SIZE" => MetaInstruction::SetOperandSize(it.next().expect("size")),

            "IDLE_IF" => MetaInstruction::IdleIf(it.by_ref().collect()),

            "SET_ADDRMODE_IMM" => MetaInstruction::SetAddrModeImmediate,
            "SET_ADDRMODE_ABS" => MetaInstruction::SetAddrModeAbsolute,
            "SET_ADDRMODE_ABSL" => MetaInstruction::SetAddrModeAbsoluteLong,
            "SET_ADDRMODE_ABSLX" => MetaInstruction::SetAddrModeAbsLongX,
            "SET_ADDRMODE_ABSX" => MetaInstruction::SetAddrModeAbsoluteX,
            "SET_ADDRMODE_ABSY" => MetaInstruction::SetAddrModeAbsoluteY,
            "SET_ADDRMODE_DIRECT" => MetaInstruction::SetAddrModeDirect,
            "SET_ADDRMODE_DIRECTX_IND" => MetaInstruction::SetAddrModeDirectXIndirect,
            "SET_ADDRMODE_DIRECT_IND" => MetaInstruction::SetAddrModeDirectIndirect,
            "SET_ADDRMODE_DIRECT_INDY" => MetaInstruction::SetAddrModeDirectIndirectY,
            "SET_ADDRMODE_DIRECT_INDLY" => MetaInstruction::SetAddrModeDirectIndirectLongY,
            "SET_ADDRMODE_DIRECT_INDL" => MetaInstruction::SetAddrModeDirectIndirectLong,
            "SET_ADDRMODE_DIRECTX" => MetaInstruction::SetAddrModeDirectX,
            "SET_ADDRMODE_DIRECTY" => MetaInstruction::SetAddrModeDirectY,
            "SET_ADDRMODE_STACK" => MetaInstruction::SetAddrModeStack,
            "SET_ADDRMODE_STACKREL" => MetaInstruction::SetAddrModeStackRelative,
            "SET_ADDRMODE_STACKREL_INDY" => MetaInstruction::SetAddrModeStackRelativeIndirectY,

            "FETCH8_INTO" => MetaInstruction::Fetch8Into(it.by_ref().collect()),
            "FETCH16_INTO" => MetaInstruction::Fetch16Into(it.by_ref().collect()),

            "FETCH_OP_INTO" => MetaInstruction::FetchOperandInto(it.by_ref().collect()),

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

            "WRITE_OP" => MetaInstruction::WriteOperand(it.by_ref().collect()),

            "PUSH8" => MetaInstruction::Push8(it.by_ref().collect()),
            "PUSHN8" => MetaInstruction::PushN8(it.by_ref().collect()),
            "PUSH16" => MetaInstruction::Push16(it.by_ref().collect()),
            "PUSHN16" => MetaInstruction::PushN16(it.by_ref().collect()),

            "SET_NZ8" => MetaInstruction::SetNZ8(it.by_ref().collect()),
            "SET_NZ16" => MetaInstruction::SetNZ16(it.by_ref().collect()),
            "SET_NZ_OP" => MetaInstruction::SetNZOperand(it.by_ref().collect()),

            kw => panic!("Unknown meta-keyword: {}", kw),
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
    fn expand(self, pstatus: &mut ParserStatus) -> MetaInstrExpansion {
        let mut ret = MetaInstrExpansion::default();

        match self {
            Self::EndCycle(cyctype) => {
                ret += InstrBody::cycles(vec![Cycle::new(TokenStream::new(), cyctype)]);
            }

            Self::SetOperandSize(arg) => {
                if pstatus.operand_size != OpSize::Constant {
                    panic!("Operand size can only be set once!");
                }
                match arg.to_string().as_str() {
                    "AccMem" => pstatus.operand_size = OpSize::AccMem,
                    "Index" => pstatus.operand_size = OpSize::Index,
                    _ => panic!("Only valid operand sizes are AccMem and Index")
                }
            }

            Self::IdleIf(condition) => {
                ret += InstrBody::cycles(vec![Cycle::conditional(condition)]);
            }

            Self::SetAddrModeImmediate => {
                match pstatus.addrmode {
                    AddrBusPosition::Immediate => {} // already imm, nothing to do
                    AddrBusPosition::Opcode => { // addrbus is already at PB:PC
                        ret += pstatus.imm_offset.map_into(|increment| quote! {
                            // in practice the increment is always 1
                            cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(#increment);
                        });
                    }
                    _ => { // default case, reset entire addrbus from scratch
                        ret += pstatus.imm_offset.map_into(|increment| quote! {
                            cpu.addr_bus.bank = cpu.registers.PB;
                            cpu.addr_bus.addr = cpu.registers.PC.wrapping_add(#increment);
                        });
                    }
                }
                pstatus.addrmode = AddrBusPosition::Immediate;
            }
            Self::SetAddrModeAbsolute => {
                // start by fetching the address at which we'll be reading/writing
                ret += Self::Fetch16ImmInto(quote! { cpu.internal_data_bus }).expand(pstatus);
                // then set the addr bus accordingly
                ret += InstrBody::post(quote! {
                    cpu.addr_bus.addr = cpu.internal_data_bus;
                    cpu.addr_bus.bank = cpu.registers.DB;
                });
                pstatus.addrmode = AddrBusPosition::Unaligned;
            }
            Self::SetAddrModeAbsoluteLong => {
                ret += Self::Fetch16ImmInto(quote!(cpu.internal_data_bus)).expand(pstatus);
                ret += Self::Fetch8Imm.expand(pstatus);

                ret += quote! {
                    cpu.addr_bus.addr = cpu.internal_data_bus;
                    cpu.addr_bus.bank = cpu.data_bus;
                };
                pstatus.addrmode = AddrBusPosition::Unaligned;
            }
            Self::SetAddrModeAbsLongX => {
                ret += Self::SetAddrModeAbsoluteLong.expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(cpu.registers.X);
                }
            }
            Self::SetAddrModeAbsoluteX => {
                ret += Self::SetAddrModeAbsolute.expand(pstatus);

                let new_addr = quote!(cpu.addr_bus.addr.wrapping_add(cpu.registers.X));
                // spend an additional cycle if indexing across page boundaries (cpu doc note 4)
                ret += Self::IdleIf(
                    quote!(*cpu.addr_bus.addr.hi() != *#new_addr.hi())
                ).expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.addr = #new_addr;
                }
            }
            Self::SetAddrModeAbsoluteY => {
                ret += Self::SetAddrModeAbsolute.expand(pstatus);

                let new_addr = quote!(cpu.addr_bus.addr.wrapping_add(cpu.registers.Y));
                // spend an additional cycle if indexing across page boundaries (cpu doc note 4)
                ret += Self::IdleIf(
                    quote!(*cpu.addr_bus.addr.hi() != *#new_addr.hi())
                ).expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.addr = #new_addr;
                }
            }
            Self::SetAddrModeDirect => {
                ret += Self::Fetch8Imm.expand(pstatus);
                // direct indexing stalls one cycle when DL != 0 (cpu doc note 2)
                ret += Self::IdleIf(quote!(*cpu.registers.D.lo() != 0)).expand(pstatus);
                ret += quote! {
                    cpu.addr_bus = snes_addr!(0:cpu.registers.D.wrapping_add(cpu.data_bus as u16));
                };
                pstatus.addrmode = AddrBusPosition::Unaligned;
            }
            Self::SetAddrModeDirectXIndirect => {
                ret += Self::SetAddrModeDirect.expand(pstatus);
                ret += Self::EndCycle(quote!(Internal)).expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(cpu.registers.X);
                };
                ret += Self::Fetch16Into(quote!(cpu.internal_data_bus)).expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.bank = cpu.registers.DB;
                    cpu.addr_bus.addr = cpu.internal_data_bus;
                };
            }
            Self::SetAddrModeDirectIndirect => {
                ret += Self::SetAddrModeDirect.expand(pstatus);
                ret += Self::Fetch16Into(quote!(cpu.internal_data_bus)).expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.bank = cpu.registers.DB;
                    cpu.addr_bus.addr = cpu.internal_data_bus;
                };
            }
            Self::SetAddrModeDirectIndirectY => {
                ret += Self::SetAddrModeDirectIndirect.expand(pstatus);

                let new_addr = quote!(cpu.addr_bus.addr.wrapping_add(cpu.registers.Y));
                // spend an additional cycle if indexing across page boundaries (cpu doc note 4)
                ret += Self::IdleIf(
                    quote!(*cpu.addr_bus.addr.hi() != *#new_addr.hi())
                ).expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.bank = cpu.registers.DB;
                    cpu.addr_bus.addr = #new_addr;
                }
            }
            Self::SetAddrModeDirectIndirectLongY => {
                ret += Self::SetAddrModeDirectIndirectLong.expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(cpu.registers.Y);
                }
            }
            Self::SetAddrModeDirectIndirectLong => {
                ret += Self::SetAddrModeDirect.expand(pstatus);
                ret += Self::Fetch16Into(quote!(cpu.internal_data_bus)).expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(1);
                };
                ret += Self::EndCycle(quote!(Read)).expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.bank = cpu.data_bus;
                    cpu.addr_bus.addr = cpu.internal_data_bus;
                }
            }
            Self::SetAddrModeDirectX => {
                ret += Self::SetAddrModeDirect.expand(pstatus);
                ret += Self::EndCycle(quote!(Internal)).expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(cpu.registers.X);
                }
            }
            Self::SetAddrModeDirectY => {
                ret += Self::SetAddrModeDirect.expand(pstatus);
                ret += Self::EndCycle(quote!(Internal)).expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(cpu.registers.Y);
                }
            }
            Self::SetAddrModeStack => {
                ret += InstrBody::post(quote! {
                    cpu.addr_bus.addr = cpu.registers.S;
                    cpu.addr_bus.bank = 0;
                });
                pstatus.addrmode = AddrBusPosition::Unaligned;
            }
            Self::SetAddrModeStackRelative => {
                ret += Self::Fetch8Imm.expand(pstatus); // read stack offset
                ret += Self::EndCycle(quote!(Internal)).expand(pstatus); // idle 1 cycle
                ret += quote! {
                    // set the addr bus to 0:S+SO
                    cpu.addr_bus = snes_addr!(0:cpu.registers.S.wrapping_add(cpu.data_bus as u16));
                };
                pstatus.addrmode = AddrBusPosition::Unaligned;
            }
            Self::SetAddrModeStackRelativeIndirectY => {
                ret += Self::SetAddrModeStackRelative.expand(pstatus);
                ret += Self::Fetch16Into(quote!(cpu.internal_data_bus)).expand(pstatus);
                ret += Self::EndCycle(quote!(Internal)).expand(pstatus);
                ret += quote! {
                    cpu.addr_bus.bank = cpu.registers.DB;
                    cpu.addr_bus.addr = cpu.internal_data_bus.wrapping_add(cpu.registers.Y);
                }
            }

            Self::Fetch8Into(dest) => {
                ret += Self::EndCycle(quote! { Read }).expand(pstatus);
                ret += quote! { #dest = cpu.data_bus; };
                if pstatus.addrmode == AddrBusPosition::Immediate {
                    pstatus.imm_offset += 1;
                }
                pstatus.addrmode = AddrBusPosition::Unaligned; // next imm is 1 further
            }
            Self::Fetch16Into(into) => {
                let is_imm = pstatus.addrmode == AddrBusPosition::Immediate;

                ret += Self::Fetch8Into(quote! { *#into.lo_mut() }).expand(pstatus);
                ret += InstrBody::post(quote! {
                    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(1);
                });
                if is_imm { // if we started as imm, now we are imm again
                    pstatus.addrmode = AddrBusPosition::Immediate;
                }
                ret += Self::Fetch8Into(quote! { *#into.hi_mut() }).expand(pstatus);
            }

            Self::FetchOperandInto(into) => {
                let is_imm = pstatus.addrmode == AddrBusPosition::Immediate;

                if is_imm {
                    // we "hide" the fact that we're reading imm when we are, otherwise
                    // the next two following meta-instr expansions would mess
                    // up trying to update the imm_offset, since they are expanded
                    // *sequentially*, when in practice they end up in split
                    // instr bodies
                    pstatus.addrmode = AddrBusPosition::Unaligned;
                }
                ret += MetaInstrExpansion::VarWidth{
                    short: Self::Fetch8Into(quote! { *#into.lo_mut() }).expand(pstatus).expect_const(),
                    long: Self::Fetch16Into(into).expand(pstatus).expect_const(),
                    data: (),
                };
                if is_imm {
                    // and now add the imm_offset of either 1 or 2
                    pstatus.imm_offset += VarWidth::varw(1, 2);
                }
            }

            Self::Fetch8Imm => {
                ret += Self::SetAddrModeImmediate.expand(pstatus);
                ret += Self::EndCycle(quote! { Read }).expand(pstatus);
                pstatus.imm_offset += 1;
                pstatus.addrmode = AddrBusPosition::Unaligned; // 1 off from next imm
            }
            Self::Fetch8ImmInto(into) => {
                ret += Self::Fetch8Imm.expand(pstatus);
                ret += InstrBody::post(quote! { #into = cpu.data_bus; });
            }
            Self::Fetch16ImmInto(into) => {
                ret += Self::SetAddrModeImmediate.expand(pstatus);
                ret += Self::Fetch16Into(into).expand(pstatus);
            }

            Self::Pull8 => {
                ret += InstrBody::post(quote! {
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
                ret += InstrBody::post(quote! {
                    // stack grows downwards
                    cpu.registers.S = cpu.registers.S.wrapping_add(1);
                });
                ret += Self::SetAddrModeStack.expand(pstatus);
                ret += Self::EndCycle(quote! { Read }).expand(pstatus);
            }
            Self::Pull8Into(into) => {
                ret += Self::Pull8.expand(pstatus);
                ret += InstrBody::post(quote! { #into = cpu.data_bus; });
            }
            Self::PullN8Into(into) => {
                ret += Self::PullN8.expand(pstatus);
                ret += InstrBody::post(quote! { #into = cpu.data_bus; });
            }
            Self::Pull16Into(into) => {
                // pulls read the low byte first
                ret += Self::Pull8Into(quote! { *#into.lo_mut() }).expand(pstatus);
                ret += Self::Pull8Into(quote! { *#into.hi_mut() }).expand(pstatus);
            }
            Self::PullN16Into(into) => {
                // pulls read the low byte first
                ret += Self::PullN8Into(quote! { *#into.lo_mut() }).expand(pstatus);
                ret += Self::PullN8Into(quote! { *#into.hi_mut() }).expand(pstatus);
            }

            Self::Write8(data) => {
                ret += InstrBody::cycles(vec![Cycle::new(
                    quote! {
                        cpu.data_bus = #data;
                    },
                    quote! { Write }
                )]);
            }
            Self::Write16(data) => {
                ret += Self::Write8(quote! { *#data.lo() }).expand(pstatus);
                ret += InstrBody::post(quote! {
                    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(1);
                });
                ret += Self::Write8(quote! { *#data.hi() }).expand(pstatus);
            }

            Self::WriteOperand(op) => {
                ret += MetaInstrExpansion::VarWidth{
                    short: Self::Write8(quote! { *#op.lo() }).expand(pstatus).expect_const(),
                    long: Self::Write16(op).expand(pstatus).expect_const(),
                    data: (),
                };
            }

            Self::Push8(data) => {
                ret += Self::SetAddrModeStack.expand(pstatus);
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
                ret += Self::SetAddrModeStack.expand(pstatus);
                // stack grows downwards
                ret += InstrBody::post(quote! {
                    cpu.registers.S = cpu.registers.S.wrapping_sub(1);
                });

                ret += Self::Write8(data).expand(pstatus);
            }
            Self::Push16(data) => {
                // pushes write the high byte first
                ret += Self::Push8(quote! { *#data.hi() }).expand(pstatus);
                ret += Self::Push8(quote! { *#data.lo() }).expand(pstatus);
            }
            Self::PushN16(data) => {
                // pushes write the high byte first
                ret += Self::PushN8(quote! { *#data.hi() }).expand(pstatus);
                ret += Self::PushN8(quote! { *#data.lo() }).expand(pstatus);
            }
            Self::SetNZ8(data) => {
                ret += quote! {
                    cpu.registers.P.Z = (#data) == 0;
                    cpu.registers.P.N = (#data) > 0x7f;
                }
            }
            Self::SetNZ16(data) => {
                ret += quote! {
                    cpu.registers.P.Z = (#data) == 0;
                    cpu.registers.P.N = (#data) > 0x7fff;
                }
            }
            Self::SetNZOperand(op) => {
                ret += MetaInstrExpansion::VarWidth{
                    short: Self::SetNZ8(quote!(*#op.lo())).expand(pstatus).expect_const(),
                    long: Self::SetNZ16(op).expand(pstatus).expect_const(),
                    data: (),
                }
            }
        }
        ret
    }
}

pub(crate) enum VarWidth<T, U = ()> {
    ConstWidth(T),
    VarWidth{short: T, long: T, data: U},
}

impl<T, U> VarWidth<T, U> {
    pub fn constw(from: T) -> Self {
        Self::ConstWidth(from)
    }

    pub fn expect_const(self) -> T {
        match self {
            Self::ConstWidth(x) => x,
            _ => panic!("Unexpected variable width"),
        }
    }

    pub fn map_mut(&mut self, mapfunc: impl Fn(&mut T)) {
        match self {
            Self::ConstWidth(x) => mapfunc(x),
            Self::VarWidth{short, long, ..} => {
                mapfunc(short);
                mapfunc(long);
            }
        }
    }
}

impl<T: Clone, U> VarWidth<T, U> {
    /// Splits this variable width data into the actual variable width
    /// variant from the constant width variant
    fn split(&mut self, data: U) {
        let Self::ConstWidth(x) = self else {
            return; // if we're already var width, early return
        };

        *self = Self::VarWidth {
            short: x.clone(),
            long: x.clone(),
            data,
        };
    }
}

impl<T, U: Clone> VarWidth<T, U> {
    pub fn map_into<T2>(&self, mut mapfunc: impl FnMut(&T) -> T2) -> VarWidth<T2, U> {
        match self {
            Self::ConstWidth(x) => VarWidth::ConstWidth(mapfunc(x)),
            Self::VarWidth{short, long, data} => VarWidth::VarWidth {
                short: mapfunc(short),
                long: mapfunc(long),
                data: data.clone(),
            },
        }
    }
}

impl<T: Clone, U: Default> VarWidth<T, U> {
    /// Same as `split`, but default-constructs the associated data
    fn split_default(&mut self) {
        self.split(U::default())
    }
}

impl<T, U: Default> VarWidth<T, U> {
    pub fn varw(short: T, long: T) -> Self {
        Self::VarWidth{short, long, data: U::default()}
    }
}

// "unofficial" clone cause otherwise we get conflicting AddAssign implementations
impl<T: Clone, U: Clone> VarWidth<T, U> {
    fn clone(&self) -> Self {
        match self {
            Self::ConstWidth(d) => Self::ConstWidth(d.clone()),
            Self::VarWidth{short, long, data} => Self::VarWidth {
                short: short.clone(),
                long: long.clone(),
                data: data.clone(),
            },
        }
    }
}


impl<T: Default, U> Default for VarWidth<T, U> {
    fn default() -> Self {
        Self::ConstWidth(T::default())
    }
}

impl<T: std::ops::AddAssign<V>, U, V: Clone> std::ops::AddAssign<V> for VarWidth<T, U> {
    fn add_assign(&mut self, x: V) {
        match *self {
            Self::ConstWidth(ref mut body) => *body += x,
            Self::VarWidth{ref mut short, ref mut long, ..} => {
                *short += x.clone();
                *long += x;
            }
        }
    }
}

impl<T1: Clone + std::ops::AddAssign<T2>, U1: Default, T2: Clone> std::ops::AddAssign<VarWidth<T2, ()>> for VarWidth<T1, U1> {
    fn add_assign(&mut self, other: VarWidth<T2, ()>) {
        match (self, other) {
            // simple case: other is constant width
            (self_, VarWidth::ConstWidth(x)) => *self_ += x,

            // both self and other are var width, add each respective part together
            (
                Self::VarWidth{short: s_short, long: s_long, ..},
                VarWidth::VarWidth{short: o_short, long: o_long, ..},
            ) => {
                *s_short += o_short;
                *s_long += o_long;
            }

            // self is constant, other is variable: split self in half, then call the case above
            (self_@Self::ConstWidth(_), other@VarWidth::VarWidth{..}) => {
                let Self::ConstWidth(b) = self_ else { unreachable!(); };
                *self_ = Self::VarWidth{
                    short: b.clone(),
                    long: b.clone(),
                    data: U1::default(),
                };
                *self_ += other;
            }
        }

    }
}

type MetaInstrExpansion = VarWidth<InstrBody>;

/// Type resulting from the parsing of the token stream passed
/// as parameter to the [`cpu_instr`] proc macro.
pub(crate) struct Instr {
    /// Name of the instruction (e.g. clv, inx, ldx_imm, jmp_abs)
    pub name: Ident,

    /// Body of the instruction, including potential post-instr code,
    /// and 16-/8-bit disjunction
    pub body: VarWidth<InstrBody, TokenStream>,
}

impl Instr {
    fn new(name: Ident) -> Self {
        Self {
            name,
            body: VarWidth::default(),
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

        loop {
            let it = it.by_ref();

            ret.body += it.take_while(|token| token.to_string() != "meta").collect::<TokenStream>();

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

        // Set PC to point at the next opcode
        match (&mut ret.body, pstatus.imm_offset.map_into(|i| pstatus.conditionally_inc_pc(*i))) {
            (VarWidth::ConstWidth(ib), VarWidth::ConstWidth(offs)) => {
                *ib.cycles.last_mut().expect("at least 1 cycle") += offs;
            }
            (VarWidth::ConstWidth(_), VarWidth::VarWidth{..}) => panic!("var-width offset with const width instr"),
            (VarWidth::VarWidth{short, long, ..}, VarWidth::ConstWidth(offs)) => {
                *short.cycles.last_mut().expect("at least 1 cycle") += offs.clone();
                *long.cycles.last_mut().expect("at least 1 cycle") += offs;
            }
            (
                VarWidth::VarWidth{short: i_s, long: i_l, ..},
                VarWidth::VarWidth{short: o_s, long: o_l, ..}
            ) => {
                *i_s.cycles.last_mut().expect("at least 1 cycle") += o_s;
                *i_l.cycles.last_mut().expect("at least 1 cycle") += o_l;
            }
        }

        // here `data` is the condition in which the instr is in 16-bit mode
        if let VarWidth::VarWidth{ref mut data, .. } = ret.body {
            match pstatus.operand_size {
                OpSize::Constant => panic!(
                    "Variable width instructions used without setting the operand size"
                ),
                OpSize::Index => *data = quote!(!cpu.registers.P.E && !cpu.registers.P.X),
                OpSize::AccMem => *data = quote!(!cpu.registers.P.E && !cpu.registers.P.M),
            }
        }
        Ok(ret)
    }
}

/// Body of an instruction: one or more cycles and potential
/// post-instruction code
#[derive(Default, Clone)]
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

impl std::ops::AddAssign<TokenStream> for InstrBody {
    /// Appends a token stream to the end of this InstrBody
    fn add_assign(&mut self, ts: TokenStream) {
        self.post_instr.extend(ts)
    }
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
            *self += other.post_instr;
            return;
        }

        // swap out self.post_instr with other.post_instr, such that
        // self.post_instr can be worked with later, and placing
        // other.post_instr already where it needs to be in the end
        let old_postinstr = std::mem::replace(&mut self.post_instr, other.post_instr);

        // swap out the first cycle body of the other InstrBody for our current
        // post_instr, and then glue it back *after* our current post_instr
        let second_firstcycle = std::mem::replace(other.cycles[0].body_mut(), old_postinstr);
        other.cycles[0].body_mut().extend(second_firstcycle);

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
#[derive(Clone)]
pub(crate) enum Cycle {
    /// An unconditional cycle: always executes
    Unconditional{
        /// Raw function body
        body: TokenStream,
        /// Cycle type (part of the function return value; should evaluate
        /// to something of type `CycleResult`)
        cyc_type: TokenStream,
    },

    /// A cycle during which the CPU might idle if a condition is met,
    /// or skip to the next cycle otherwise
    ConditionalIdle{
        /// The condition for which the conditional cycle is idled
        condition: TokenStream,

        /// The body of this cycle's function
        /// Usually should be empty, but may contain post-cycle code
        /// from a previous unconditional cycle
        body: TokenStream,
    },
}

impl Cycle {
    fn new(body: TokenStream, cyc_type: TokenStream) -> Self {
        Self::Unconditional { body, cyc_type }
    }

    fn conditional(condition: TokenStream) -> Self {
        Self::ConditionalIdle{condition, body: quote!()}
    }

    fn body_ref(&self) -> &TokenStream {
        match self {
            Self::Unconditional{body, ..} => body,
            Self::ConditionalIdle{body, ..} => body,
        }
    }

    fn body_mut(&mut self) -> &mut TokenStream {
        match self {
            Self::Unconditional{body, ..} => body,
            Self::ConditionalIdle{body, ..} => body,
        }
    }
}

impl std::ops::AddAssign<TokenStream> for Cycle {
    fn add_assign(&mut self, ts: TokenStream) {
        self.body_mut().extend(ts);
    }
}
