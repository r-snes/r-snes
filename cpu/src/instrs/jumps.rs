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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instrs::test_prelude::*;

    #[test]
    fn test_jump_absolute() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x4c);
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x12,
                addr: 0x3457,
            },
            0xcd,
            "jump address (low)",
        );
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x12,
                addr: 0x3458,
            },
            0xab,
            "jump address (high)",
        );

        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0xabcd;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn test_jump_absolute_long() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x5c);
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x12,
                addr: 0x3457,
            },
            0xef,
            "jump address (PC low)",
        );
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x12,
                addr: 0x3458,
            },
            0xcd,
            "jump address (PC high)",
        );
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x12,
                addr: 0x3459,
            },
            0xab,
            "jump address (PB)",
        );

        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PB = 0xab;
        expected_regs.PC = 0xcdef;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn test_jmp_abs_ind() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x6c);
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x12,
                addr: 0x3457,
            },
            0x00,
            "operand address (low)",
        );
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x12,
                addr: 0x3458,
            },
            0x22,
            "operand address (high)",
        );
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x00,
                addr: 0x2200,
            },
            0x89,
            "jump address (PC low)",
        );
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x00,
                addr: 0x2201,
            },
            0x67,
            "jump address (PC high)",
        );

        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x6789;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn test_jump_long() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xdc);
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x12,
                addr: 0x3457,
            },
            0x77,
            "operand address (low)",
        );
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x12,
                addr: 0x3458,
            },
            0x88,
            "operand address (high)",
        );
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x00,
                addr: 0x8877,
            },
            0xef,
            "jump address (PC low)",
        );
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x00,
                addr: 0x8878,
            },
            0xcd,
            "jump address (PC high)",
        );
        expect_read_cycle(
            &mut cpu,
            SnesAddress {
                bank: 0x00,
                addr: 0x8879,
            },
            0xab,
            "jump address (PB)",
        );

        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PB = 0xab;
        expected_regs.PC = 0xcdef;
        assert_eq!(*cpu.regs(), expected_regs);
    }
}
