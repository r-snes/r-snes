//! Module which implements all "jump" instructions:
//! instructions which inconditionnaly make the execution
//! address (i.e. PC:PB) jump to another location

use crate::instrs::prelude::*;
use common::u16_split::*;
use instr_metalang_procmacro::cpu_instr_no_inc_pc;

// JMP absolute: jump program execution to the
// 16-bits PC written after the opcode
cpu_instr_no_inc_pc!(jmp_abs {
    // We need to use NoIncPC because we're assigning into PC;
    // we don't want it to increment at the end of the instruction
    meta FETCH16_IMM_INTO cpu.registers.PC;
});

// JMP absolute long: jump program execution to the
// 24-bits address written after PC
cpu_instr_no_inc_pc!(jmp_absl {
    // We need to use NoIncPC because we're assigning into PC;
    // we don't want it to increment at the end of the instruction
    meta FETCH16_IMM_INTO cpu.registers.PC;

    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(1);
    meta FETCH8_INTO cpu.registers.PB;
});

// JMP absolute indirect: read a 16-bits address (call it A) right after the opcode,
// then read the 16-bits jump address at address A in bank 0
cpu_instr_no_inc_pc!(jmp_abs_ind {
    meta FETCH16_IMM_INTO cpu.registers.PC; // use PC as a buffer

    cpu.addr_bus.bank = 0;
    cpu.addr_bus.addr = cpu.registers.PC; // read from the fetched address
    meta FETCH16_INTO cpu.registers.PC;
});

// JML: jump long
// similar to JMP absolute indirect except we also read a new PB
cpu_instr_no_inc_pc!(jml {
    meta FETCH16_IMM_INTO cpu.registers.PC; // use PC as a buffer

    cpu.addr_bus.bank = 0;
    cpu.addr_bus.addr = cpu.registers.PC; // read from the fetched address
    meta FETCH16_INTO cpu.registers.PC;

    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(1);
    meta FETCH8_INTO cpu.registers.PB;
});
