use duplicate::{duplicate_item, duplicate};
use instr_metalang_procmacro::cpu_instr;
use super::algorithms;

// duplicate over these 6 instructions which share the same 15 addressing modes
#[duplicate_item(
    DUP1_name;
    [eor];
    [ora];
    [and];
    [cmp];
    [adc];
    [sbc];
)]
pub(crate) mod DUP1_name {
    use super::*;

    // we write a `use` instead of using DUP1_algo directly because it
    // doesn't substitute properly in the nested `duplicate!` call below
    use algorithms::DUP1_name as algo;

    // duplicate for all addressing modes of these instructions
    duplicate! {
        [
            DUP2_name   DUP2_addrmode;
            [abs]       [SET_ADDRMODE_ABS];
            [absl]      [SET_ADDRMODE_ABSL];
            [abslx]     [SET_ADDRMODE_ABSLX];
            [absx]      [SET_ADDRMODE_ABSX];
            [absy]      [SET_ADDRMODE_ABSY];
            [d]         [SET_ADDRMODE_DIRECT];
            [dxind]     [SET_ADDRMODE_DIRECTX_IND];
            [dind]      [SET_ADDRMODE_DIRECT_IND];
            [dindy]     [SET_ADDRMODE_DIRECT_INDY];
            [dindly]    [SET_ADDRMODE_DIRECT_INDLY];
            [dindl]     [SET_ADDRMODE_DIRECT_INDL];
            [dx]        [SET_ADDRMODE_DIRECTX];
            [imm]       [SET_ADDRMODE_IMM];
            [sr]        [SET_ADDRMODE_STACKREL];
            [sry]       [SET_ADDRMODE_STACKREL_INDY];
        ]
        cpu_instr!(DUP2_name {
            meta SET_OP_SIZE AccMem;
            meta DUP2_addrmode;

            meta FETCH_OP_INTO cpu.internal_data_bus;

            meta LET_VARWIDTH_MUT a = cpu.registers.A;
            meta LET_VARWIDTH idb = cpu.internal_data_bus;

            algo(a, idb, &mut cpu.registers.P);
        });
    }
}

// duplicate for CPX and CPY, which are similar to the above except
// they exist for fewer addrmodes and their width depend on the X flag
duplicate! {
    [
        DUP_name    DUP_reg DUP_addrmode;
        [cpx_imm]   [X]     [SET_ADDRMODE_IMM];
        [cpx_abs]   [X]     [SET_ADDRMODE_ABS];
        [cpx_d]     [X]     [SET_ADDRMODE_DIRECT];

        [cpy_imm]   [Y]     [SET_ADDRMODE_IMM];
        [cpy_abs]   [Y]     [SET_ADDRMODE_ABS];
        [cpy_d]     [Y]     [SET_ADDRMODE_DIRECT];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE Index;
        meta DUP_addrmode;

        meta FETCH_OP_INTO cpu.internal_data_bus;

        meta LET_VARWIDTH_MUT reg = cpu.registers.DUP_reg;
        meta LET_VARWIDTH idb = cpu.internal_data_bus;

        algorithms::cmp(reg, idb, &mut cpu.registers.P);
    });
}

#[cfg(test)]
mod tests {
    use crate::instrs::test_prelude::*;

    #[test]
    fn adc_imm8() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.A = 0x44;
        regs.P.E = true;
        regs.P.Z = true;
        regs.P.N = false;
        regs.P.V = false;
        regs.P.C = false;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x69);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x44, "operand");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.A = 0x88;
        expected_regs.PC = 0x3458;
        expected_regs.P.Z = false;
        expected_regs.P.N = true;
        expected_regs.P.V = true;
        expected_regs.P.C = false;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn adc_imm16() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.A = 0x5588;
        regs.P.E = false;
        regs.P.M = false;
        regs.P.Z = true;
        regs.P.N = true;
        regs.P.C = false;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x69);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x43, "operand lo");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0x21, "operand hi");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.A = 0x76cb;
        expected_regs.PC = 0x3459;
        expected_regs.P.Z = false;
        expected_regs.P.N = false;
        expected_regs.P.C = false;
        assert_eq!(*cpu.regs(), expected_regs);
    }
}
