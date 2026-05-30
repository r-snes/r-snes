use instr_metalang_procmacro::cpu_instr_no_inc_pc;
use duplicate::duplicate;

cpu_instr_no_inc_pc!(brk {
    meta FETCH8_IMM; // ignored imm read

    if cpu.registers.E {
        // skip the PB push if in emu mode
        return brk_cyc3(cpu);
    }
    meta PUSH8 cpu.registers.PB;
    meta PUSH16 cpu.registers.PC.wrapping_add(2);
    meta PUSH8 cpu.registers.P.into();

    cpu.registers.P.I = true;
    cpu.registers.P.D = false;

    let addr = if cpu.registers.E {
        0xFFFE
    } else {
        0xFFE6
    };
    cpu.addr_bus = snes_addr!(0:addr);
    meta FETCH16_INTO cpu.registers.PC;
    cpu.registers.PB = 0;
});

cpu_instr_no_inc_pc!(cop {
    meta FETCH8_IMM; // ignored imm read

    if cpu.registers.E {
        // skip the PB push if in emu mode
        return cop_cyc3(cpu);
    }
    meta PUSH8 cpu.registers.PB;
    meta PUSH16 cpu.registers.PC.wrapping_add(2);
    meta PUSH8 cpu.registers.P.into();

    cpu.registers.P.I = true;
    cpu.registers.P.D = false;

    let addr = if cpu.registers.E {
        0xFFF4
    } else {
        0xFFE4
    };
    cpu.addr_bus = snes_addr!(0:addr);
    meta FETCH16_INTO cpu.registers.PC;
    cpu.registers.PB = 0;
});

cpu_instr_no_inc_pc!(rti {
    meta END_CYCLE Internal;
    meta END_CYCLE Internal;

    meta PULL8;
    cpu.registers.P = cpu.data_bus.into();

    meta PULL16_INTO cpu.registers.PC;

    // emu mode doesn't pull PB, goes straight to opcode fetch
    if cpu.registers.E {
        return opcode_fetch(cpu);
    }

    meta PULL8_INTO cpu.registers.PB;
});

#[cfg(test)]
mod test {
    use super::super::test_prelude::*;

    #[test]
    fn brk_emu() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.S = 0x0180;
        regs.P = 0b10101010.into();
        regs.E = true;

        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x00);
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3457),
            0x33,
            "signature byte (ignored)",
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x0180),
            0x34,
            "PCH",
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x017f),
            0x56 + 2, // pushes PC + 2
            "PCL",
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x017e),
            0b10101010,
            "P",
        );

        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0xFFFE),
            0x66,
            "interrupt routine addr low",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0xFFFF),
            0x33,
            "interrupt routine addr hi",
        );

        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3366;
        expected_regs.PB = 0;
        expected_regs.S = 0x017d;
        expected_regs.P.D = false;
        expected_regs.P.I = true;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn brk_nat() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.S = 0x0180;
        regs.P = 0b10101010.into();
        regs.E = false;

        let mut expected_regs = regs.clone();

        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x00);
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0x12:0x3457),
            0x33,
            "signature byte (ignored)",
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x0180),
            0x12,
            "PB",
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x017f),
            0x34,
            "PCH",
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x017e),
            0x56 + 2, // pushes PC + 2
            "PCL",
        );
        expect_write_cycle(
            &mut cpu,
            snes_addr!(0:0x017d),
            0b10101010,
            "P",
        );

        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0xFFE6),
            0x66,
            "interrupt routine addr low",
        );
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:0xFFE7),
            0x33,
            "interrupt routine addr hi",
        );

        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3366;
        expected_regs.PB = 0;
        expected_regs.S = 0x017c;
        expected_regs.P.D = false;
        expected_regs.P.I = true;
        assert_eq!(*cpu.regs(), expected_regs);
    }
}
