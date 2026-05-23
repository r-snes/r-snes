use instr_metalang_procmacro::cpu_instr;
use duplicate::duplicate;

// Push variable width registers
duplicate! {
    [
        DUP_name    DUP_reg     DUP_size;
        [pha]       [A]         [AccMem];
        [phx]       [X]         [Index];
        [phy]       [Y]         [Index];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE DUP_size;

        meta END_CYCLE Internal;

        meta PUSH_OP cpu.registers.DUP_reg;
    });
}

// Pull variable width registers
duplicate! {
    [
        DUP_name    DUP_reg     DUP_size;
        [pla]       [A]         [AccMem];
        [plx]       [X]         [Index];
        [ply]       [Y]         [Index];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE DUP_size;

        meta END_CYCLE Internal;
        meta END_CYCLE Internal;

        meta PULL_OP_INTO cpu.registers.DUP_reg;
        meta SET_NZ_OP cpu.registers.DUP_reg;
    });
}

// Push P register
cpu_instr!(php {
    meta END_CYCLE Internal;
    meta PUSH8 cpu.registers.P.into();
});

// Push D register
cpu_instr!(phd {
    meta END_CYCLE Internal;
    meta PUSHN16 cpu.registers.D;
});

// Push 8-bit registers
duplicate! {
    [
        DUP_name    DUP_reg;
        [phb]       [DB];
        [phk]       [PB];
    ]
    cpu_instr!(DUP_name {
        meta END_CYCLE Internal;
        meta PUSHN8 cpu.registers.DUP_reg;
    });
}

// Pull DB register
cpu_instr!(plb {
    meta END_CYCLE Internal;
    meta END_CYCLE Internal;

    meta PULLN8_INTO cpu.registers.DB;
    meta SET_NZ8 cpu.registers.DB;
});

// Pull D register
cpu_instr!(pld {
    meta END_CYCLE Internal;
    meta END_CYCLE Internal;

    meta PULLN16_INTO cpu.registers.D;
    meta SET_NZ16 cpu.registers.D;
});

// Pull P register
cpu_instr!(plp {
    meta END_CYCLE Internal;
    meta END_CYCLE Internal;

    meta PULL8;
    cpu.registers.P = cpu.data_bus.into();

    if cpu.registers.E {
        cpu.registers.P.M = true;
        cpu.registers.P.X = true;
    } else {
        if cpu.registers.P.X {
            *cpu.registers.X.hi_mut() = 0;
            *cpu.registers.Y.hi_mut() = 0;
        }
    }
});

// Push effective address: reads an immediat 16 bit operand and pushes it
cpu_instr!(pea {
    meta FETCH16_IMM_INTO cpu.internal_data_bus;
    meta PUSHN16 cpu.internal_data_bus;
});

// Push effective relative address: push an address relative to PC
cpu_instr!(per {
    meta FETCH16_IMM_INTO cpu.internal_data_bus;

    cpu.internal_data_bus = cpu.registers.PC
        .wrapping_add(3)
        .wrapping_add(cpu.internal_data_bus);

    meta END_CYCLE Internal;

    meta PUSHN16 cpu.internal_data_bus;
});

// Push effective indirect address: push a 16-bit direct operand
cpu_instr!(pei {
    meta SET_ADDRMODE_DIRECT;

    meta FETCH16_INTO cpu.internal_data_bus;
    meta PUSHN16 cpu.internal_data_bus;
});

#[cfg(test)]
mod tests {
    use super::super::test_prelude::*;

    // we only test pha for pha, phx, and phy since they are duplicated
    #[test]
    fn pha() {
        let mut regs = Registers::default();
        regs.A = 0x5566;
        regs.S = 0x0477;
        regs.PC = 0;
        regs.PB = 0;
        let mut expected_regs = regs;
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x48);
        expect_internal_cycle(&mut cpu, "stack alignment");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0477), 0x55, "push hi");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0476), 0x66, "push lo");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 1;
        expected_regs.S = 0x0475;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    // we only test pla for pla, plx, and ply since they are duplicated
    #[test]
    fn pla() {
        let mut regs = Registers::default();
        regs.S = 0x0475;
        regs.PC = 0;
        regs.PB = 0;
        let mut expected_regs = regs;
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x68);
        expect_internal_cycle(&mut cpu, "stack alignment (1)");
        expect_internal_cycle(&mut cpu, "stack alignment (2)");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x0476), 0x66, "pull lo");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x0477), 0x55, "pull hi");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 1;
        expected_regs.A = 0x5566;
        expected_regs.S = 0x0477;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn php() {
        let mut regs = Registers::default();
        regs.P = 0x42.into();
        regs.S = 0x0477;
        regs.PC = 0;
        regs.PB = 0;
        let mut expected_regs = regs;
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x08);
        expect_internal_cycle(&mut cpu, "stack alignment");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0477), 0x42, "push");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 1;
        expected_regs.S = 0x0476;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    // we only test phb for phb and phk (8 bit pushes)
    #[test]
    fn phb() {
        let mut regs = Registers::default();
        regs.DB = 0x42;
        regs.S = 0x0477;
        regs.PC = 0;
        regs.PB = 0;
        let mut expected_regs = regs;
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x8b);
        expect_internal_cycle(&mut cpu, "stack alignment");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0477), 0x42, "push");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 1;
        expected_regs.S = 0x0476;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn plb() {
        let mut regs = Registers::default();
        regs.S = 0x0476;
        regs.PC = 0;
        regs.PB = 0;
        let mut expected_regs = regs;
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xab);
        expect_internal_cycle(&mut cpu, "stack alignment (1)");
        expect_internal_cycle(&mut cpu, "stack alignment (2)");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x0477), 0x42, "pull");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 1;
        expected_regs.DB = 0x42;
        expected_regs.S = 0x0477;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn pld() {
        let mut regs = Registers::default();
        regs.S = 0x0475;
        regs.PC = 0;
        regs.PB = 0;
        let mut expected_regs = regs;
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x2b);
        expect_internal_cycle(&mut cpu, "stack alignment (1)");
        expect_internal_cycle(&mut cpu, "stack alignment (2)");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x0476), 0x11, "pull lo");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x0477), 0x99, "pull hi");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 1;
        expected_regs.D = 0x9911;
        expected_regs.S = 0x0477;
        expected_regs.P.N = true; // 0x9911 is negative
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn plp() {
        let mut regs = Registers::default();
        regs.S = 0x0476;
        regs.PC = 0;
        regs.PB = 0;
        let mut expected_regs = regs;
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x28);
        expect_internal_cycle(&mut cpu, "stack alignment (1)");
        expect_internal_cycle(&mut cpu, "stack alignment (2)");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x0477), 0x42, "pull");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 1;
        expected_regs.P = 0x42.into();
        expected_regs.S = 0x0477;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn pea() {
        let mut regs = Registers::default();
        regs.S = 0x0866;
        regs.PC = 0;
        regs.PB = 0;
        let mut expected_regs = regs;
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xf4);
        expect_read_cycle(&mut cpu, snes_addr!(0:1), 0x44, "address lo");
        expect_read_cycle(&mut cpu, snes_addr!(0:2), 0x33, "address hi");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0866), 0x33, "address hi");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0865), 0x44, "address lo");

        expected_regs.PC = 3;
        expected_regs.S = 0x0864;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn per() {
        let mut regs = Registers::default();
        regs.S = 0x0866;
        regs.PC = 0x100;
        regs.PB = 0;
        let mut expected_regs = regs;
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x62);
        expect_read_cycle(&mut cpu, snes_addr!(0:0x101), 0x44, "offset lo");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x102), 0x33, "offset hi");
        expect_internal_cycle(&mut cpu, "stack alignment");
        // then we have a 16 bit push of PC + offset, taking the PC as it would be
        // for the next opcode (0x103 in this case)
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0866), 0x34, "address hi");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0865), 0x47, "address lo");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x103;
        expected_regs.S = 0x0864;
        assert_eq!(*cpu.regs(), expected_regs);
    }
    
    #[test]
    fn pei() {
        let mut regs = Registers::default();
        regs.S = 0x0866;
        regs.PC = 0x100;
        regs.PB = 0;
        regs.D = 0x1234;
        let mut expected_regs = regs;
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xd4);
        expect_read_cycle(&mut cpu, snes_addr!(0:0x101), 0x11, "direct offset");
        expect_internal_cycle(&mut cpu, "DL != 0");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x1245), 0x88, "address lo");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x1246), 0x99, "address hi");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0866), 0x99, "address hi");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0865), 0x88, "address lo");

        expected_regs.PC = 0x102;
        expected_regs.S = 0x0864;

        assert_eq!(*cpu.regs(), expected_regs);
    }
}
