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

// JSR absolute: jump stack relative absolute
// same as JMP absolute, but pushes PC to the stack before jumping
cpu_instr_no_inc_pc!(jsr_abs {
    meta FETCH16_IMM_INTO cpu.internal_data_bus;

    // artificial internal cycle to reproduce hardware behaviour
    meta END_CYCLE Internal;

    // push a PC that is 1 byte before the next opcode
    meta PUSH16 cpu.registers.PC.wrapping_add(2);

    cpu.registers.PC = cpu.internal_data_bus;
});

// JSR absolute indirect X-indexed: same as JMP (a,x) but also push PC
// cycle layout exceptionally weird: R AAL; W PC; R AAH; I; R PC
cpu_instr_no_inc_pc!(jsr_abs_ind_xind {
    meta FETCH8_IMM_INTO *cpu.internal_data_bus.lo_mut();

    // adjust pushed PC to next opcode - 1
    meta PUSHN16 cpu.registers.PC.wrapping_add(2);

    // we need to readjust the addr bus manually to read immediate values again
    cpu.addr_bus = SnesAddress {
        bank: cpu.registers.PB,
        addr: cpu.registers.PC.wrapping_add(2),
    };
    meta FETCH8_INTO *cpu.internal_data_bus.hi_mut();
    // now cpu.internal_bus finally has the address at which we'll read PC

    meta END_CYCLE Internal; // internal cycle to reproduce hardware behaviour

    // set the addr bus to read PC.
    // We intentionally don't set the bank, keep reading in PB
    cpu.addr_bus.addr = cpu.internal_data_bus.wrapping_add(cpu.registers.X);

    meta FETCH16_INTO cpu.registers.PC;

    // *cpu.registers.S.hi_mut() = 0x01;
    // We may or may not to uncomment the above, I don't know for now
    // See https://github.com/bsnes-emu/bsnes/issues/374 for info
});

// JSL: jump stack relative long
// same as JSR abs, but also read/write PB
// cycle layout is a bit weird: R PC; W PB; I; R PB; W PC
cpu_instr_no_inc_pc!(jsl {
    meta FETCH16_IMM_INTO cpu.internal_data_bus;
    meta PUSHN8 cpu.registers.PB;

    meta END_CYCLE Internal;

    meta FETCH8_IMM_INTO cpu.registers.PB;
    // adjust pushed PC to next opcode - 1
    meta PUSHN16 cpu.registers.PC.wrapping_add(3);

    cpu.registers.PC = cpu.internal_data_bus;

    // *cpu.registers.S.hi_mut() = 0x01;
    // see https://github.com/bsnes-emu/bsnes/issues/374
});

// RTS: return from subroutine (return from a JSR).
// reads PC from the stack and jumps to that address
cpu_instr_no_inc_pc!(rts {
    // RTS spends two internal cycles doing nothing
    meta END_CYCLE Internal;
    meta END_CYCLE Internal;

    meta PULL16_INTO cpu.registers.PC;

    // readjust PC: the pushed value is 1 byte before the next opcode
    cpu.registers.PC = cpu.registers.PC.wrapping_add(1);

    meta END_CYCLE Internal; // and one more internal cycle for nothing
});

// RTL: return from subroutine long (return from a JSL).
// reads PC and PB from the stack and jumps to that address
cpu_instr_no_inc_pc!(rtl {
    // RTL spends two internal cycles doing nothing
    meta END_CYCLE Internal;
    meta END_CYCLE Internal;

    meta PULLN16_INTO cpu.registers.PC;
    meta PULLN8_INTO cpu.registers.PB;

    // readjust PC: the pushed value is 1 byte before the next opcode
    cpu.registers.PC = cpu.registers.PC.wrapping_add(1);

    // *cpu.registers.S.hi_mut() = 0x01;
    // see https://github.com/bsnes-emu/bsnes/issues/374
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

    #[test]
    fn test_jsr_abs() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.S = 0x0188; // set the stack pointer in page 1
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x20);
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3457),
            0xcd,
            "jump address (PC high)",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3458),
            0xab,
            "jump address (PC high)",
        );
        expect_internal_cycle(&mut cpu, "stall between fetch and push");
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x0188),
            0x34,
            "push PC high",
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x0187),
            0x58, // PC points to the last byte of this instruction
            "push PC high",
        );
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0xabcd; // PC has been read
        expected_regs.S = 0x0186; // stack pointer decreased by 2
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn test_jsr_abs_ind_xind() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.S = 0x0122;
        regs.X = 0x0120;
        regs.PC = 0x3456;
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xfc);
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3457),
            0x77,
            "operand address high",
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x0122),
            0x34,
            "push PCH"
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x0121),
            0x58, // PC points the last byte of this instruction
            "push PCL"
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3458),
            0x88,
            "operand address low",
        );
        expect_internal_cycle(&mut cpu, "X-indexing");
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x8997), // PB:AAH+X
            0xbb,
            "PCL",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x8998), // PB:AAH+X+1
            0xaa,
            "PCH",
        );
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.S = 0x0120;
        expected_regs.PC = 0xaabb;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn test_jsl() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.S = 0x0199;
        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x22);
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3457),
            0xef,
            "PCL",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3458),
            0xcd,
            "PCH",
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x0199),
            0x12,
            "push PB",
        );
        expect_internal_cycle(&mut cpu, "stall between push PB and read PB");
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3459),
            0xab,
            "PB",
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x0198),
            0x34,
            "push PCH",
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x0197),
            0x59, // pushed PC points the last byte of the instr
            "push PCL",
        );
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PB = 0xab;
        expected_regs.PC = 0xcdef;
        expected_regs.S = 0x0196;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn test_rts() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.S = 0x0155;
        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x60);
        expect_internal_cycle(&mut cpu, "first stall cycle");
        expect_internal_cycle(&mut cpu, "second stall cycle");
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0x0156),
            0xbb,
            "pull PCL",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0x0157),
            0xaa,
            "pull PCH",
        );
        expect_internal_cycle(&mut cpu, "second stall cycle");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.S = 0x0157;
        // [the pulled value + 1]: jsr/jsl push [address of the next opcode - 1]
        expected_regs.PC = 0xaabc;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn test_rtl() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.S = 0x0155;
        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x6b);
        expect_internal_cycle(&mut cpu, "first stall cycle");
        expect_internal_cycle(&mut cpu, "second stall cycle");
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0x0156),
            0xbb,
            "pull PCL",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0x0157),
            0xaa,
            "pull PCH",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0x0158),
            0xee,
            "pull PB",
        );
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.S = 0x0158;
        // [the pulled value + 1]: jsr/jsl push [address of the next opcode - 1]
        expected_regs.PB = 0xee;
        expected_regs.PC = 0xaabc;
        assert_eq!(*cpu.regs(), expected_regs);
    }
}
