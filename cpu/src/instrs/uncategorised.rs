//! Module which defines many instructions which don't quite fit into
//! a bigger category, at least don't *yet* fit into a category with other
//! currently implemented instructions.

use instr_metalang_procmacro::{
    cpu_instr,
    cpu_instr_no_inc_pc,
};
use duplicate::duplicate;

// `NOP`: "no-op" (no operation). Literally does nothing
cpu_instr!(nop {
    meta END_CYCLE Internal;
});

// `WDM`: reserved for future use, does nothing
// Actually takes the same number of cycles as a NOP, but with
// a read cycle instead of an internal cycle.
// The read is immediate, making the instruction two bytes long
cpu_instr!(wdm {
    meta FETCH8_IMM;
});

// `XCE`: eXchange Carry and Emulation
// Swaps the carry bit with the emulation bit.
// This is the only instruction which can toggle emulation on and off
cpu_instr!(xce {
    std::mem::swap(&mut cpu.registers.P.C, &mut cpu.registers.E);

    // switching to (or already in) emulation mode
    if cpu.registers.E {
        *cpu.registers.X.hi_mut() = 0;
        *cpu.registers.Y.hi_mut() = 0;
        *cpu.registers.S.hi_mut() = 1;
        cpu.registers.P.X = true;
        cpu.registers.P.M = true;
    }

    meta END_CYCLE Internal;
});

// REset P: resets (clears) flags of the P register
cpu_instr!(rep {
    meta FETCH8_IMM;

    let mut p: u8 = cpu.registers.P.into();
    p &= !cpu.data_bus;

    cpu.registers.P = p.into();

    // re-force X and M to 1 if in emulation mode: they can't be set to 0
    if cpu.registers.E {
        cpu.registers.P.X = true;
        cpu.registers.P.M = true;
    }

    meta END_CYCLE Internal;
});

// SEt P: sets flags of the P register
cpu_instr!(sep {
    meta FETCH8_IMM;

    let mut p: u8 = cpu.registers.P.into();
    p |= cpu.data_bus;

    cpu.registers.P = p.into();

    // reset XH and YH if we just set the X flag
    if cpu.registers.P.X {
        *cpu.registers.X.hi_mut() = 0;
        *cpu.registers.Y.hi_mut() = 0;
    }

    meta END_CYCLE Internal;
});

// eXchange B A: swaps the A and B accumulators
// (the high byte of the accumulator is also referred to as B,
// in which case the name A refers to the low byte)
cpu_instr!(xba {
    meta END_CYCLE Internal;

    cpu.registers.A = cpu.registers.A.swap_bytes();
    meta SET_NZ8 *cpu.registers.A.lo();
    meta END_CYCLE Internal;
});

// `MVN`/`MVP`: MoVe Positive/Negative
// These "block move" instructions can move blocks of memory across banks
// and at different addresses. The source and destination bank are immediate
// operands, whereas the addresses are specified in X and Y registers.
// The A register specifies the number of bytes to move (minus 1: A == 0 means
// we move 1 byte).
// DB is overwritten with the destination bank number.
//
// Interestingly, these instructions actually loop themselves (they don't
// increment PC until the last iteration, so that their opcode is read again
// and again), executing several times until A wraps around from 0 to 0xFFFF.
// So actually, these instructions only move byte per "call", taking their full
// 7 seven cycles of execution to move each byte.
// X and Y are also adjusted between calls to continue to the next byte to move
//
// The difference between these instructions is that MVN copies the block from
// start to end (increasing addresses), whereas MVP copies from end to start.
// Thus, for MVP, the initial values in X and Y are the addresses of the end of
// the source and destination.
duplicate! {
    [
        DUP_name    DUP_inc;
        [mvn]       [wrapping_add];
        [mvp]       [wrapping_sub];
    ]
    cpu_instr_no_inc_pc!(DUP_name {
        meta FETCH8_IMM_INTO cpu.registers.DB; // fetch destination bank
        meta FETCH8_IMM_INTO cpu.addr_bus.bank; // fetch source bank

        cpu.addr_bus.addr = cpu.registers.X;
        meta END_CYCLE Read;

        cpu.addr_bus.bank = cpu.registers.DB;
        cpu.addr_bus.addr = cpu.registers.Y;
        meta END_CYCLE Write;

        meta END_CYCLE Internal;

        if cpu.registers.P.X {
            *cpu.registers.X.lo_mut() = cpu.registers.X.lo().DUP_inc(1);
            *cpu.registers.Y.lo_mut() = cpu.registers.Y.lo().DUP_inc(1);
        } else {
            cpu.registers.X = cpu.registers.X.DUP_inc(1);
            cpu.registers.Y = cpu.registers.Y.DUP_inc(1);
        }

        cpu.registers.A = cpu.registers.A.wrapping_sub(1);
        if cpu.registers.A == 0xFFFF {
            cpu.registers.PC = cpu.registers.PC.wrapping_add(3);
        }

        meta END_CYCLE Internal;
    });
}

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

    #[test]
    fn wdm() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x42);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x00, "idle (ignored read)");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3458;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn xce_to_emu() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.C = true; // C will move to E
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xfb);
        expect_internal_cycle(&mut cpu, "switch emu bit");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3457;

        expected_regs.E = true;
        expected_regs.P.C = false;
        expected_regs.P.X = true; // M and X are set to 1
        expected_regs.P.M = true; // since we switched to emu mode
        *expected_regs.S.hi_mut() = 1; // and SH is set to 1

        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn xce_to_nat() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.C = false; // C will move to E
        regs.E = true;
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xfb);
        expect_internal_cycle(&mut cpu, "switch emu bit");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3457;

        expected_regs.E = false;
        expected_regs.P.C = true;

        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn sep() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.Z = true; // set Z now and set it again with SEP, shouldn't change
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xe2);
        //                                                     -V----ZC
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0b01000011, "bits to set in P");
        expect_internal_cycle(&mut cpu, "idle after setting flags");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3458;
        expected_regs.P.C = true;
        expected_regs.P.V = true;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn rep() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.Z = true; // set Z for it to be reset by REP
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xc2);
        //                                                     N-----Z-
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0b10000010, "bits to clear in P");
        expect_internal_cycle(&mut cpu, "idle after clearing flags");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3458;
        expected_regs.P.Z = false;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn xba() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.A = 0xbbaa;
        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xeb);
        expect_internal_cycle(&mut cpu, "swap");
        expect_internal_cycle(&mut cpu, "swap (2)");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3457;
        expected_regs.A = 0xaabb;
        expected_regs.P.N = true; // because 0xbb is negative
        assert_eq!(*cpu.regs(), expected_regs);
    }
}
