//! Module which implements all "jump" instructions:
//! instructions which inconditionnaly make the execution
//! address (i.e. PC:PB) jump to another location

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

    cpu.addr_bus = snes_addr!(0:cpu.registers.PC); // read from the fetched addr
    meta FETCH16_INTO cpu.registers.PC;
});

// JMP absolute indirect X-indexed: same as above, except we add X to the
// absolute address (read the jump addr at A+X instead of A), and read it
// in bank PB instead of bank 0
cpu_instr_no_inc_pc!(jmp_abs_ind_indx {
    meta FETCH16_IMM_INTO cpu.registers.PC; // use PC as a buffer

    // artificially add an internal cycle to replicate hardware behaviour
    // this cycle was probably necessary to perform the X-indexing
    meta END_CYCLE Internal;

    // cpu.addr_bus.bank = PB; // bank is already PB since last fetch was immediate
    cpu.addr_bus.addr = cpu.registers.PC.wrapping_add(cpu.registers.X); // read at A+X
    meta FETCH16_INTO cpu.registers.PC;
});

// JML: jump long
// similar to JMP absolute indirect except we also read a new PB
cpu_instr_no_inc_pc!(jml {
    meta FETCH16_IMM_INTO cpu.registers.PC; // use PC as a buffer

    cpu.addr_bus = snes_addr!(0:cpu.registers.PC); // read from the fetched addr
    meta FETCH16_INTO cpu.registers.PC;

    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(1);
    meta FETCH8_INTO cpu.registers.PB;
});

#[cfg(test)]
mod tests {
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
            snes_addr!(0x12:0x3457),
            0xcd,
            "jump address (low)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3458),
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
            snes_addr!(0x12:0x3457),
            0xef,
            "jump address (PC low)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3458),
            0xcd,
            "jump address (PC high)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3459),
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
            snes_addr!(0x12:0x3457),
            0x00,
            "operand address (low)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3458),
            0x22,
            "operand address (high)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0x2200),
            0x89,
            "jump address (PC low)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0x2201),
            0x67,
            "jump address (PC high)",
        );

        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x6789;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn test_jmp_abs_ind_indx() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.X = 0x1000;
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x7c);
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3457),
            0xa0,
            "operand address (low)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3458),
            0xb0,
            "operand address (high)",
        );
        expect_internal_cycle(&mut cpu, "internal cycle for X-indexing");
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0xc0a0), // PB:(addr+X)
            0x89,
            "jump address (PC low)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0xc0a1), // PB:(addr+X+1)
            0x67,
            "jump address (PC high)",
        );

        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x6789;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn test_jmp_abs_ind_indx_wraparound() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.X = 0xf000; // We set a large X so that the X-indexing wraps around
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x7c);
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3457),
            0x30,
            "operand address (low)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3458),
            0x20,
            "operand address (high)",
        );
        expect_internal_cycle(&mut cpu, "internal cycle for X-indexing");
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x1030), // PB:(addr+X)
            0x89,
            "jump address (PC low)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x1031), // PB:(addr+X+1)
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
            snes_addr!(0x12:0x3457),
            0x77,
            "operand address (low)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3458),
            0x88,
            "operand address (high)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0x8877),
            0xef,
            "jump address (PC low)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0x8878),
            0xcd,
            "jump address (PC high)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0x8879),
            0xab,
            "jump address (PB)",
        );

        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PB = 0xab;
        expected_regs.PC = 0xcdef;
        assert_eq!(*cpu.regs(), expected_regs);
    }
}
