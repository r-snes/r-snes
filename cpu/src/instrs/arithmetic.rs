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

// duplicate over all addrmodes of BIT
// we have to include the algo because BIT immediate has a slightly
// different algorithm (doesn't set N and V flags)
duplicate! {
    [
        DUP_name    DUP_algo    DUP_addrmode;
        [bit_d]     [bit]       [SET_ADDRMODE_DIRECT];
        [bit_abs]   [bit]       [SET_ADDRMODE_ABS];
        [bit_dx]    [bit]       [SET_ADDRMODE_DIRECTX];
        [bit_absx]  [bit]       [SET_ADDRMODE_ABSX];
        [bit_imm]   [bit_imm]   [SET_ADDRMODE_IMM];
    ]
    cpu_instr!(DUP_name {
            meta SET_OP_SIZE AccMem;
            meta DUP_addrmode;

            meta FETCH_OP_INTO cpu.internal_data_bus;

            meta LET_VARWIDTH_MUT a = cpu.registers.A;
            meta LET_VARWIDTH idb = cpu.internal_data_bus;

            algorithms::DUP_algo(a, idb, &mut cpu.registers.P);
    });
}

// duplicate over all 8 RMW (read-modify-write) instructions which
// share the same cycle layout, and overall logic
//
// TRB and TSB are available for fewer addr modes, and also read the
// accumulator, so we do some weird duplication for things to work
duplicate! {
    [
        DUP_name    DUP_algo    DUP_addrmode            DUP_trb_tsb_arg;
        [asl_abs]   [asl]      [SET_ADDRMODE_ABS]       [];
        [asl_absx]  [asl]      [SET_ADDRMODE_ABSX]      [];
        [asl_d]     [asl]      [SET_ADDRMODE_DIRECT]    [];
        [asl_dx]    [asl]      [SET_ADDRMODE_DIRECTX]   [];

        [lsr_abs]   [lsr]      [SET_ADDRMODE_ABS]       [];
        [lsr_absx]  [lsr]      [SET_ADDRMODE_ABSX]      [];
        [lsr_d]     [lsr]      [SET_ADDRMODE_DIRECT]    [];
        [lsr_dx]    [lsr]      [SET_ADDRMODE_DIRECTX]   [];

        [inc_abs]   [inc]      [SET_ADDRMODE_ABS]       [];
        [inc_absx]  [inc]      [SET_ADDRMODE_ABSX]      [];
        [inc_d]     [inc]      [SET_ADDRMODE_DIRECT]    [];
        [inc_dx]    [inc]      [SET_ADDRMODE_DIRECTX]   [];

        [dec_abs]   [dec]      [SET_ADDRMODE_ABS]       [];
        [dec_absx]  [dec]      [SET_ADDRMODE_ABSX]      [];
        [dec_d]     [dec]      [SET_ADDRMODE_DIRECT]    [];
        [dec_dx]    [dec]      [SET_ADDRMODE_DIRECTX]   [];

        [rol_abs]   [rol]      [SET_ADDRMODE_ABS]       [];
        [rol_absx]  [rol]      [SET_ADDRMODE_ABSX]      [];
        [rol_d]     [rol]      [SET_ADDRMODE_DIRECT]    [];
        [rol_dx]    [rol]      [SET_ADDRMODE_DIRECTX]   [];

        [ror_abs]   [ror]      [SET_ADDRMODE_ABS]       [];
        [ror_absx]  [ror]      [SET_ADDRMODE_ABSX]      [];
        [ror_d]     [ror]      [SET_ADDRMODE_DIRECT]    [];
        [ror_dx]    [ror]      [SET_ADDRMODE_DIRECTX]   [];

        [tsb_abs]   [tsb]      [SET_ADDRMODE_ABS]       [_a, ];
        [tsb_d]     [tsb]      [SET_ADDRMODE_DIRECT]    [_a, ];

        [trb_abs]   [trb]      [SET_ADDRMODE_ABS]       [_a, ];
        [trb_d]     [trb]      [SET_ADDRMODE_DIRECT]    [_a, ];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE AccMem;
        meta DUP_addrmode;

        meta IF_16 {
            // copy the original addrbus to restore it in the 16-bit write
            cpu.addr_bus2 = cpu.addr_bus;
        };

        meta FETCH_OP_INTO cpu.internal_data_bus;
        meta LET_VARWIDTH _a = cpu.registers.A;
        meta LET_VARWIDTH_MUT idb = cpu.internal_data_bus;

        algorithms::DUP_algo(idb, DUP_trb_tsb_arg &mut cpu.registers.P);
        meta END_CYCLE Internal;

        meta IF_16 {
            // if 16-bit, write the high byte first, at the current
            // addrbus, which is where the high byte was read.
            meta WRITE8 *cpu.internal_data_bus.hi();

            // then reset addrbus to where the low byte was read
            // so that it ends up being written at the correct addr
            cpu.addr_bus = cpu.addr_bus2;
        };

        meta WRITE8 *cpu.internal_data_bus.lo();
    });
}

// duplicate over the 8 rmw instructions on registers
duplicate! {
    [
        DUP_name    DUP_algo    DUP_reg     DUP_opsize;
        [asl_acc]   [asl]       [A]         [AccMem];
        [lsr_acc]   [lsr]       [A]         [AccMem];
        [rol_acc]   [rol]       [A]         [AccMem];
        [ror_acc]   [ror]       [A]         [AccMem];

        [inc_acc]   [inc]       [A]         [AccMem];
        [inx]       [inc]       [X]         [Index];
        [iny]       [inc]       [Y]         [Index];

        [dec_acc]   [dec]       [A]         [AccMem];
        [dex]       [dec]       [X]         [Index];
        [dey]       [dec]       [Y]         [Index];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE DUP_opsize;
        meta LET_VARWIDTH_MUT reg = cpu.registers.DUP_reg;

        algorithms::DUP_algo(reg, &mut cpu.registers.P);

        meta END_CYCLE Internal;
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

    #[test]
    fn bit_imm16() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.A = 0x00ff;
        regs.P.E = false;
        regs.P.M = false;
        regs.P.Z = false;
        regs.P.N = false;
        regs.P.V = false;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x89);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x00, "operand lo");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0xff, "operand hi");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3459;
        expected_regs.P.Z = true; // immediate BIT only touches Z
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn bit_abs16() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.A = 0x00ff;
        regs.P.E = false;
        regs.P.M = false;
        regs.P.Z = false;
        regs.P.N = false;
        regs.P.V = false;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x2c);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0xcd, "operand address lo");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0xab, "operand address hi");
        expect_read_cycle(&mut cpu, snes_addr!(0:0xabcd), 0x00, "operand lo");
        expect_read_cycle(&mut cpu, snes_addr!(0:0xabce), 0xff, "operand hi");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3459;
        expected_regs.P.Z = true;
        expected_regs.P.N = true;
        expected_regs.P.V = true;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    #[test]
    fn asl_abs16() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.E = false;
        regs.P.M = false;

        regs.P.Z = true;
        regs.P.N = true;
        regs.P.V = true;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x0e);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x89, "AAL");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0x67, "AAH");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x6789), 0xf0, "operand lo");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x678a), 0x0f, "operand hi");
        expect_internal_cycle(&mut cpu, "modify");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x678a), 0x1f, "operand hi");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x6789), 0xe0, "operand lo");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3459;
        expected_regs.P.Z = false;
        expected_regs.P.N = false;
        expected_regs.P.V = false;
    }

    #[test]
    fn asl_abs8() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.E = true;

        regs.P.Z = true;
        regs.P.N = true;
        regs.P.V = true;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x0e);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x89, "AAL");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0x67, "AAH");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x6789), 0x0f, "operand");
        expect_internal_cycle(&mut cpu, "modify");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x6789), 0x1e, "operand");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3459;
        expected_regs.P.Z = false;
        expected_regs.P.N = false;
        expected_regs.P.V = false;
    }

    #[test]
    fn asl_acc8() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.E = true;

        regs.A = 0x0f;

        regs.P.Z = true;
        regs.P.N = true;
        regs.P.V = true;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x0a);
        expect_internal_cycle(&mut cpu, "modify");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.A = 0x1e;
        expected_regs.PC = 0x3457;
        expected_regs.P.Z = false;
        expected_regs.P.N = false;
        expected_regs.P.V = false;
    }
}
