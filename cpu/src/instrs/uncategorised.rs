//! Module which defines many instructions which don't quite fit into
//! a bigger category, at least don't *yet* fit into a category with other
//! currently implemented instructions.

use crate::instrs::prelude::*;

// `INX` instruction: increment register X
//
// Flags set:
// - `Z`: whether the result is zero
// - `N`: whether the result is negative (if it were interpreted as signed)
cpu_instr!(inx {
    cpu.registers.X = cpu.registers.X.wrapping_add(1);
    cpu.registers.P.Z = cpu.registers.X == 0;
    cpu.registers.P.N = cpu.registers.X > 0x7fff;

    meta END_INSTR Internal;
});

// `NOP`: "no-op" (no operation). Literally does nothing
cpu_instr!(nop {
    meta END_INSTR Internal;
});
