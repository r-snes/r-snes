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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instrs::test_prelude::*;

    use duplicate::duplicate_item;

    #[duplicate_item(
        DUP_instr_name DUP_set_flag DUP_opcode;
        [clv] [V] [0xb8];
        [cld] [D] [0xd8];
        [cli] [I] [0x58];
        [clc] [C] [0x18];
    )]
    #[test]
    fn DUP_instr_name() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        let mut expected_regs = regs.clone();

        regs.P.DUP_set_flag = true;
        let mut cpu = CPU::new(regs);

        // Check for opcode fetch cycle and inject the clear opcode into the CPU
        expect_opcode_fetch(&mut cpu, DUP_opcode);
        expect_internal_cycle(&mut cpu, "clear flag");

        expected_regs.PC = expected_regs.PC + 1; // We expect PC to be incremented
        expected_regs.P.DUP_set_flag = false;    // and the flag to be cleared
        assert_eq!(cpu.registers, expected_regs, "Flag should be cleared");

        // Execute the instruction once more to check the flag stays clear
        expect_opcode_fetch(&mut cpu, DUP_opcode);
        expect_internal_cycle(&mut cpu, "clearing the flag again");

        expected_regs.PC = expected_regs.PC + 1; // PC should be incremented once again
        assert_eq!(cpu.registers, expected_regs, "Flag should stay cleared");
    }
}
