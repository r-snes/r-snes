//! Module which defines all instructions which mainly affect CPU flags

use crate::instrs::prelude::*;

// `CLV`: clear overflow flag
cpu_instr!(clv {
    cpu.registers.P.V = false;
    meta END_INSTR Internal;
});

// `CLC`: clear carry flag
cpu_instr!(clc {
    cpu.registers.P.C = false;
    meta END_INSTR Internal;
});

// `CLI`: clear Interrupt Disable bit
cpu_instr!(cli {
    cpu.registers.P.I = false;
    meta END_INSTR Internal;
});

// `CLD`: clear decimal flag
cpu_instr!(cld {
    cpu.registers.P.D = false;
    meta END_INSTR Internal;
});
