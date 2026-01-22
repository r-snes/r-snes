use instr_metalang_procmacro::cpu_instr;
use duplicate::duplicate;

duplicate! {
    [
        DUP_name    DUP_src             DUP_opsize  DUP_addrmode;
        // stx: write X register to memory
        [stx_abs]   [cpu.registers.X]   [Index]     [SET_ADDRMODE_ABS];
        [stx_d]     [cpu.registers.X]   [Index]     [SET_ADDRMODE_DIRECT];
        [stx_dy]    [cpu.registers.X]   [Index]     [SET_ADDRMODE_DIRECTY];

        // sty: write Y register to memory
        [sty_abs]   [cpu.registers.Y]   [Index]     [SET_ADDRMODE_ABS];
        [sty_d]     [cpu.registers.Y]   [Index]     [SET_ADDRMODE_DIRECT];
        [sty_dx]    [cpu.registers.Y]   [Index]     [SET_ADDRMODE_DIRECTX];

        // sta: write A register to memory
        [sta_abs]   [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_ABS];
        [sta_absl]  [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_ABSL];
        [sta_abslx] [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_ABSLX];
        [sta_absx]  [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_ABSX];
        [sta_absy]  [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_ABSY];
        [sta_d]     [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECT];
        [sta_dxind] [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECTX_IND];
        [sta_dind]  [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECT_IND];
        [sta_dindy] [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECT_INDY];
        [sta_dindly][cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECT_INDLY];
        [sta_dindl] [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECT_INDL];
        [sta_dx]    [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECTX];
        [sta_sr]    [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_STACKREL];
        [sta_sry]   [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_STACKREL_INDY];

        // stz: write 0 to memory
        [stz_abs]   [0]                 [AccMem]    [SET_ADDRMODE_ABS];
        [stz_absx]  [0]                 [AccMem]    [SET_ADDRMODE_ABSX];
        [stz_d]     [0]                 [AccMem]    [SET_ADDRMODE_DIRECT];
        [stz_dx]    [0]                 [AccMem]    [SET_ADDRMODE_DIRECTX];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE DUP_opsize;

        meta DUP_addrmode;
        meta WRITE_OP DUP_src;
    });
}

#[cfg(test)]
mod tests {
    use super::super::test_prelude::*;
    use duplicate::duplicate_item;

    // since we already have extensive tests for all load instructions,
    // we know all addressing modes work properly, so we don't need to
    // them all here.
    // Essentially all we want to test is the `meta WRITE_OP` thing:
    // byte write order, correct 16-/8-bit mode handling, and write a value
    // from the right register.

    #[duplicate_item(
        DUP_name    DUP_opcode  DUP_reg;
        [sta8_abs]  [0x8d]      [A];
        [stx8_abs]  [0x8e]      [X];
        [sty8_abs]  [0x8c]      [Y];
    )]
    #[test]
    fn DUP_name() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.DUP_reg = 0x5544;
        regs.P.E = true; // 8-bit mode
        regs.DB = 0xdb;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, DUP_opcode);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x22, "AAL");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0x11, "AAH");
        expect_write_cycle(&mut cpu, snes_addr!(0xdb:0x1122), 0x44, "write reg");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3459;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn stz_abs() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.E = true; // 8-bit mode
        regs.DB = 0xdb;

        // set all other registers which can be written, to check stz
        // doesn't read from a register
        regs.A = 0xabcd;
        regs.X = 0xabcd;
        regs.Y = 0xabcd;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x9c);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x22, "AAL");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0x11, "AAH");
        expect_write_cycle(&mut cpu, snes_addr!(0xdb:0x1122), 0, "write zero");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3459;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    // test 16-bit writes in the right order
    #[test]
    fn sta16_abs() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.A = 0x5544;
        regs.P.E = false;
        regs.P.M = false;
        regs.DB = 0xdb;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x8d);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x22, "AAL");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0x11, "AAH");
        expect_write_cycle(&mut cpu, snes_addr!(0xdb:0x1122), 0x44, "AL");
        expect_write_cycle(&mut cpu, snes_addr!(0xdb:0x1123), 0x55, "AH");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3459;
        assert_eq!(*cpu.regs(), expected_regs);
    }
}
