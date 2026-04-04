//! Module which defines "transfer" instructions: simple instructions
//! which copy the content of one register to another
//!
//! For all of those instructions, the size (8 bit vs 16 bit) of the
//! transfer is determined by the size of the destination register.
//! The name of the instruction mnenomics means 'Transfer'-'Source'-'Dest',
//! for example TXS means 'Transfer X to S'.

use super::algorithms;
use instr_metalang_procmacro::cpu_instr;
use duplicate::duplicate;

// duplicate over all transfers to S, because S becomes 8 bit
// in emu mode
duplicate! {
    [
        DUP_name    DUP_src;
        [txs]       [X];
        [tcs]       [A];
    ]
    cpu_instr!(DUP_name {
        if cpu.registers.E {
            *cpu.registers.S.lo_mut() = *cpu.registers.DUP_src.lo();
        } else {
            cpu.registers.S = cpu.registers.DUP_src;
        }

        meta END_CYCLE Internal;
    });
}

// duplicate over variable width registers (as destination --
// since the size of the transfer comes from the size of the source)
duplicate! {
    [
        DUP_name    DUP_src     DUP_dest    DUP_size;
        [tax]       [A]         [X]         [Index];
        [tay]       [A]         [Y]         [Index];
        [tsx]       [S]         [X]         [Index];
        [txa]       [X]         [A]         [AccMem];
        [txy]       [X]         [Y]         [Index];
        [tya]       [Y]         [A]         [AccMem];
        [tyx]       [Y]         [X]         [Index];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE DUP_size;

        meta LET_VARWIDTH_MUT dest = cpu.registers.DUP_dest;
        meta LET_VARWIDTH src = cpu.registers.DUP_src;

        *dest = src;

        algorithms::set_nz(*dest, &mut cpu.registers.P);

        meta END_CYCLE Internal;
    });
}

// duplicate over all 16-bit transfers
duplicate! {
    [
        DUP_name    DUP_src     DUP_dest;
        [tcd]       [A]         [D];
        [tdc]       [D]         [A];
        [tsc]       [S]         [A];
    ]
    cpu_instr!(DUP_name {
        cpu.registers.DUP_dest = cpu.registers.DUP_src;
        algorithms::set_nz(cpu.registers.DUP_dest, &mut cpu.registers.P);

        meta END_CYCLE Internal;
    });
}

#[cfg(test)]
mod tests {
    use super::super::test_prelude::*;

    // we only test txs for txs and tcs since they do the exact same
    #[test]
    fn txs_8() {
        let mut regs = Registers::default();
        regs.PB = 0x55;
        regs.PC = 0x7777;
        regs.X = 0xff44;
        regs.S = 0x0133;
        regs.E = true; // for 8-bit transfer

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x9a);
        expect_internal_cycle(&mut cpu, "transfer");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x7778;
        expected_regs.S = 0x0144; // only low 8 bits changed
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn txs_16() {
        let mut regs = Registers::default();
        regs.PB = 0x55;
        regs.PC = 0x7777;
        regs.X = 0xff44;
        regs.S = 0x0133;
        regs.E = false; // for 16-bit transfer

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x9a);
        expect_internal_cycle(&mut cpu, "transfer");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x7778;
        expected_regs.S = 0xff44; // all 16 bits
        assert_eq!(*cpu.regs(), expected_regs);
    }

    // we only test tax for all transfers to X, Y, and A since they do the same
    #[test]
    fn tax_8() {
        let mut regs = Registers::default();
        regs.PB = 0x55;
        regs.PC = 0x7777;
        regs.A = 0xff44;
        regs.X = 0x0033;
        regs.P.X = true; // for 8-bit transfer

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xaa);
        expect_internal_cycle(&mut cpu, "transfer");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x7778;
        expected_regs.X = 0x0044; // only low 8 bits are changed
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn tax_16() {
        let mut regs = Registers::default();
        regs.PB = 0x55;
        regs.PC = 0x7777;
        regs.A = 0xff44;
        regs.X = 0x0133;
        regs.E = false; // for 16-bit transfer
        regs.P.X = false; // for 16-bit transfer

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xaa);
        expect_internal_cycle(&mut cpu, "transfer");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x7778;
        expected_regs.X = 0xff44; // all 16 bits
        expected_regs.P.N = true; // 0xff44 is negative
        assert_eq!(*cpu.regs(), expected_regs);
    }

    // we only test tcd for tcd, tsc and tdc, since they are duplicated
    #[test]
    fn tcd() {
        let mut regs = Registers::default();
        regs.PB = 0x55;
        regs.PC = 0x7777;
        regs.A = 0xff44;
        regs.D = 0x0133;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x5b);
        expect_internal_cycle(&mut cpu, "transfer");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x7778;
        expected_regs.D = 0xff44;
        expected_regs.P.N = true; // 0xff44 is negative
        assert_eq!(*cpu.regs(), expected_regs);
    }
}
