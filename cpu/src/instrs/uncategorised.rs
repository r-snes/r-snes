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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instrs::test_prelude::*;

    #[test]
    fn test_1_plus_1_is_2_cycle_api() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;

        regs.X = 1;
        let mut cpu = CPU::new(regs);

        assert_eq!(
            cpu.cycle(),
            CycleResult::Read,
            "Expecting a read cycle for opcode fetch"
        );
        assert_eq!(
            *cpu.addr_bus(),
            SnesAddress {
                bank: 0x12,
                addr: 0x3456
            },
            "Read query should be from address at PB:PC"
        );
        cpu.data_bus = 0xe8; // Inject the INX opcode into the CPU

        assert_eq!(
            cpu.cycle(),
            CycleResult::Internal,
            "Expecting internal cycle for register increment"
        );
        assert_eq!(cpu.regs().X, 2, "Expecting value 2 in X register");
    }

    #[test]
    fn nop_does_nothing() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        let mut regs_copy = regs.clone();

        let mut cpu = CPU::new(regs);

        assert_eq!(
            cpu.cycle(),
            CycleResult::Read,
            "Expecting a read cycle for opcode fetch"
        );
        cpu.data_bus = 0xea; // Inject the NOP opcode into the CPU

        assert_eq!(
            cpu.cycle(),
            CycleResult::Internal,
            "Expecting internal cycle for register increment"
        );

        regs_copy.PC = regs_copy.PC + 1;
        assert_eq!(cpu.registers, regs_copy, "Only PC should have been touched");
    }
}
