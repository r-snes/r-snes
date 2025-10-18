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
        let mut regs_copy = regs.clone();

        regs.P.DUP_set_flag = true;
        let mut cpu = CPU::new(regs);

        assert_eq!(
            cpu.cycle(),
            CycleResult::Read,
            "Expecting a read cycle for opcode fetch"
        );
        cpu.data_bus = DUP_opcode; // Inject the clear opcode into the CPU

        assert_eq!(
            cpu.cycle(),
            CycleResult::Internal,
            "Expecting internal cycle for clear flag"
        );

        regs_copy.PC = regs_copy.PC + 1;
        regs_copy.P.DUP_set_flag = false;

        assert_eq!(cpu.registers, regs_copy, "Flag should be cleared");

        // Execute the instruction once more to check the flag stays clear
        assert_eq!(
            cpu.cycle(),
            CycleResult::Read,
            "Expecting a read cycle for opcode fetch"
        );
        assert_eq!(
            cpu.addr_bus,
            SnesAddress {
                bank: 0x12,
                addr: 0x3457
            },
            "Opcode fetch should be from the next byte",
        );
        regs_copy.PC = regs_copy.PC + 1;
        assert_eq!(cpu.registers, regs_copy, "Flag should stay cleared");
    }
}

