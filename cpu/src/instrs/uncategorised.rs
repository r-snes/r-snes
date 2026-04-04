//! Module which defines many instructions which don't quite fit into
//! a bigger category, at least don't *yet* fit into a category with other
//! currently implemented instructions.

use instr_metalang_procmacro::cpu_instr;

// `INX` instruction: increment register X
//
// Flags set:
// - `Z`: whether the result is zero
// - `N`: whether the result is negative (if it were interpreted as signed)
cpu_instr!(inx {
    cpu.registers.X = cpu.registers.X.wrapping_add(1);
    cpu.registers.P.Z = cpu.registers.X == 0;
    cpu.registers.P.N = cpu.registers.X > 0x7fff;

    meta END_CYCLE Internal;
});

// `NOP`: "no-op" (no operation). Literally does nothing
cpu_instr!(nop {
    meta END_CYCLE Internal;
});

#[cfg(test)]
mod tests {
    use crate::instrs::test_prelude::*;

    #[test]
    fn test_1_plus_1_is_2_cycle_api() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;

        regs.X = 1;
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xe8);
        expect_internal_cycle(&mut cpu, "register increment");

        assert_eq!(cpu.regs().X, 2, "Expecting value 2 in X register");

        expect_opcode_fetch_cycle(&mut cpu);
    }

    #[test]
    fn nop_does_nothing() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xea);
        expect_internal_cycle(&mut cpu, "no-op");

        expected_regs.PC = expected_regs.PC + 1;
        assert_eq!(cpu.registers, expected_regs, "Only PC should have been touched");

        expect_opcode_fetch_cycle(&mut cpu);
    }
}
