use duplicate::duplicate;
use instr_metalang_procmacro::cpu_instr;

duplicate! {
    [
        DUP_name    DUP_dest    DUP_opsize  DUP_addrmode;
        [ldx_imm]   [X]         [Index]     [SET_ADDRMODE_IMM];
        [ldx_abs]   [X]         [Index]     [SET_ADDRMODE_ABS];
        [ldx_absy]  [X]         [Index]     [SET_ADDRMODE_ABSY];
        [ldx_d]     [X]         [Index]     [SET_ADDRMODE_DIRECT];
        [ldx_dy]    [X]         [Index]     [SET_ADDRMODE_DIRECTY];

        [ldy_imm]   [Y]         [Index]     [SET_ADDRMODE_IMM];
        [ldy_abs]   [Y]         [Index]     [SET_ADDRMODE_ABS];
        [ldy_absx]  [Y]         [Index]     [SET_ADDRMODE_ABSX];
        [ldy_d]     [Y]         [Index]     [SET_ADDRMODE_DIRECT];
        [ldy_dx]    [Y]         [Index]     [SET_ADDRMODE_DIRECTX];

        [lda_imm]   [A]         [AccMem]    [SET_ADDRMODE_IMM];
        [lda_abs]   [A]         [AccMem]    [SET_ADDRMODE_ABS];
        [lda_absl]  [A]         [AccMem]    [SET_ADDRMODE_ABSL];
        [lda_abslx] [A]         [AccMem]    [SET_ADDRMODE_ABSLX];
        [lda_absx]  [A]         [AccMem]    [SET_ADDRMODE_ABSX];
        [lda_absy]  [A]         [AccMem]    [SET_ADDRMODE_ABSY];
        [lda_d]     [A]         [AccMem]    [SET_ADDRMODE_DIRECT];
        [lda_dxind] [A]         [AccMem]    [SET_ADDRMODE_DIRECTX_IND];
        [lda_dind]  [A]         [AccMem]    [SET_ADDRMODE_DIRECT_IND];
        [lda_dindy] [A]         [AccMem]    [SET_ADDRMODE_DIRECT_INDY];
        [lda_dindly][A]         [AccMem]    [SET_ADDRMODE_DIRECT_INDLY];
        [lda_dindl] [A]         [AccMem]    [SET_ADDRMODE_DIRECT_INDL];
        [lda_dx]    [A]         [AccMem]    [SET_ADDRMODE_DIRECTX];
        [lda_sr]    [A]         [AccMem]    [SET_ADDRMODE_STACKREL];
        [lda_sry]   [A]         [AccMem]    [SET_ADDRMODE_STACKREL_INDY];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE DUP_opsize;

        meta DUP_addrmode;
        meta FETCH_OP_INTO cpu.registers.DUP_dest;
        meta SET_NZ_OP cpu.registers.DUP_dest;
    });
}

#[cfg(test)]
mod tests {
    use crate::instrs::test_prelude::*;
    use duplicate::duplicate_item;

    fn expect_load16_read(cpu: &mut CPU, mut from: SnesAddress, value: u16) {
        expect_read_cycle(cpu, from, *value.lo(), "first byte of a 16-bit load");
        from.addr += 1;
        expect_read_cycle(cpu, from, *value.hi(), "second byte of a 16-bit load");
    }

    // nested duplicated to test immediate loads for all regs
    #[duplicate_item(
        DUP1_mod    DUP1_reg    DUP1_flag   DUP1_opcode;
        [lda_imm]   [A]         [M]         [0xa9];
        [ldx_imm]   [X]         [X]         [0xa2];
        [ldy_imm]   [Y]         [X]         [0xa0];
    )]
    mod DUP1_mod {
        use super::*;

        // duplicate for all output combinations of N and Z flags
        // for 16-bit loads
        #[duplicate_item(
            DUP2_name       DUP2_val  DUP2_N    DUP2_Z;
            [imm16_pos]     [0x4321]  [false]   [false]; // positive nonzero
            [imm16_neg]     [0xabcd]  [true]    [false]; // negative as i16
            [imm16_zero]    [0]       [false]   [true];
        )]
        #[test]
        fn DUP2_name() {
            let mut regs = Registers::default();
            regs.PB = 0x12;
            regs.PC = 0x3456;
            regs.P.E = false; // need to disable emu mode for 16-bit load
            regs.P.DUP1_flag = false; // and turn off M flag too
            regs.DUP1_reg = 0x9999; // value which will be overwritten

            // start with flags set to the opposite of the expected
            regs.P.Z = !DUP2_Z;
            regs.P.N = !DUP2_N;

            let mut expected_regs = regs.clone();
            let mut cpu = CPU::new(regs);

            expect_opcode_fetch(&mut cpu, DUP1_opcode);
            expect_load16_read(&mut cpu, snes_addr!(0x12:0x3457), DUP2_val);
            expect_opcode_fetch_cycle(&mut cpu);

            expected_regs.DUP1_reg = DUP2_val;
            expected_regs.PC = 0x3459;
            expected_regs.P.N = DUP2_N;
            expected_regs.P.Z = DUP2_Z;
            assert_eq!(*cpu.regs(), expected_regs);
        }

        // duplicate for all output combinations of N and Z flags
        // for 8-bit loads
        #[duplicate_item(
            DUP2_name       DUP2_val  DUP2_N    DUP2_Z;
            [imm8_pos]      [0x43]    [false]   [false]; // positive nonzero
            [imm8_neg]      [0xab]    [true]    [false]; // negative as i8
            [imm8_zero]     [0]       [false]   [true];
        )]
        mod DUP2_name {
            use super::*;

            // duplicate for all reasons why the instr can be in 8-bit mode
            #[duplicate_item(
                DUP3_name   DUP3_emu    DUP3_index;
                [emu_no_ind][true]      [false]; // When emu mode ON, always 8-bit mode
                [emu_ind]   [true]      [true]; // When emu mode ON, always 8-bit mode
                [no_emu_ind][false]     [true]; // When emu mode OFF, 8-bit mode when the index
                                                // flag is set
            )]
            #[test]
            fn DUP3_name() {
                let mut regs = Registers::default();
                regs.PB = 0x12;
                regs.PC = 0x3456;
                regs.P.E = DUP3_emu;
                regs.P.DUP1_flag = DUP3_index;
                regs.DUP1_reg = 0x9999; // value which will be overwritten

                // start with flags set to the opposite of the expected
                regs.P.Z = !DUP2_Z;
                regs.P.N = !DUP2_N;

                let mut expected_regs = regs.clone();
                let mut cpu = CPU::new(regs);

                expect_opcode_fetch(&mut cpu, DUP1_opcode);
                expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), DUP2_val, "8-bit load");
                expect_opcode_fetch_cycle(&mut cpu);

                *expected_regs.DUP1_reg.lo_mut() = DUP2_val;
                expected_regs.PC = 0x3458;
                expected_regs.P.N = DUP2_N;
                expected_regs.P.Z = DUP2_Z;
                assert_eq!(*cpu.regs(), expected_regs);
            }
        }
    }

    // now we only test 16-bit positive nonzero case, since we already
    // tested all combinations in immediate, and the implementation is
    // duplicated for all addressing modes

    // duplicate for all regs
    #[duplicate_item(
        DUP_name    DUP_reg     DUP_opcode;
        [lda_abs]   [A]         [0xad];
        [ldx_abs]   [X]         [0xae];
        [ldy_abs]   [Y]         [0xac];
    )]
    #[test]
    fn DUP_name() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.E = false; // non-emu mode for 16-bit instr
        regs.P.X = false; // unset both X and M
        regs.P.M = false; // so that all instrs are 16-bit
        regs.DB = 0xdb;
        regs.DUP_reg = 0x9999; // value which will be overwritten

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, DUP_opcode);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0xaa, "address low");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0xbb, "address high");
        expect_read_cycle(&mut cpu, snes_addr!(0xdb:0xbbaa), 0x44, "value low");
        expect_read_cycle(&mut cpu, snes_addr!(0xdb:0xbbab), 0x33, "value high");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.DUP_reg = 0x3344;
        expected_regs.PC = 0x3459;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    // absl addrmode only exists for LDA
    #[test]
    fn lda_absl() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.E = false; // non-emu mode for 16-bit instr
        regs.P.X = false; // unset both X and M
        regs.P.M = false; // so that all instrs are 16-bit
        regs.A = 0x9999; // value which will be overwritten

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xaf);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0xaa, "address low");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0xbb, "address high");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3459), 0xee, "address bank");
        expect_read_cycle(&mut cpu, snes_addr!(0xee:0xbbaa), 0x44, "value low");
        expect_read_cycle(&mut cpu, snes_addr!(0xee:0xbbab), 0x33, "value high");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.A = 0x3344;
        expected_regs.PC = 0x345a;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    // abslx addrmode only exists for LDA
    #[test]
    fn lda_abslx() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.E = false; // non-emu mode for 16-bit instr
        regs.P.X = false; // unset both X and M
        regs.P.M = false; // so that all instrs are 16-bit
        regs.A = 0x9999; // value which will be overwritten
        regs.X = 0x0102;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xbf);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0xaa, "address low");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0xbb, "address high");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3459), 0xee, "address bank");
        expect_read_cycle(&mut cpu, snes_addr!(0xee:0xbcac), 0x44, "value low");
        expect_read_cycle(&mut cpu, snes_addr!(0xee:0xbcad), 0x33, "value high");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.A = 0x3344;
        expected_regs.PC = 0x345a;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    // ALL absolute indexed instructions
    // in each block of 4, there is:
    //   1. X flag off; index within page boundary => only case where we don't idle
    //   2. X flag off; index across page boundary => idle cycle for indexing
    //   3. X flag on; index within page boundary => idle cycle for indexing
    //   4. X flag on; index across page boundary => idle cycle for indexing
    //
    // Interestingly, for LDX and LDY, the X flag controls both the indexing register
    // and the register that will be loaded (since it controls X and Y), so turning it
    // on make the instruction load only 1 byte
    #[duplicate_item(
        //        loaded reg           X flag  index reg
        DUP_name    DUP_reg DUP_opcode  DUP_xf  DUP_idx DUP_idx_val DUP_idle    DUP_16;
        [lda_absx1] [A]     [0xbd]      [true]  [X]     [0x0020]    [false]     [true];
        [lda_absx2] [A]     [0xbd]      [true]  [X]     [0x00ff]    [true]      [true];
        [lda_absx3] [A]     [0xbd]      [false] [X]     [0x0020]    [true]      [true];
        [lda_absx4] [A]     [0xbd]      [false] [X]     [0x0220]    [true]      [true];

        [lda_absy1] [A]     [0xb9]      [true]  [Y]     [0x0020]    [false]     [true];
        [lda_absy2] [A]     [0xb9]      [true]  [Y]     [0x00ff]    [true]      [true];
        [lda_absy3] [A]     [0xb9]      [false] [Y]     [0x0220]    [true]      [true];
        [lda_absy4] [A]     [0xb9]      [false] [Y]     [0x0220]    [true]      [true];

        [ldy_absx1] [Y]     [0xbc]      [true]  [X]     [0x0020]    [false]     [false];
        [ldy_absx2] [Y]     [0xbc]      [true]  [X]     [0x00ff]    [true]      [false];
        [ldy_absx3] [Y]     [0xbc]      [false] [X]     [0x0020]    [true]      [true];
        [ldy_absx4] [Y]     [0xbc]      [false] [X]     [0x0220]    [true]      [true];

        [ldx_absy1] [X]     [0xbe]      [true]  [Y]     [0x0020]    [false]     [false];
        [ldx_absy2] [X]     [0xbe]      [true]  [Y]     [0x00ff]    [true]      [false];
        [ldx_absy3] [X]     [0xbe]      [false] [Y]     [0x0020]    [true]      [true];
        [ldx_absy4] [X]     [0xbe]      [false] [Y]     [0x0220]    [true]      [true];
    )]
    #[test]
    fn DUP_name() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.E = false; // non-emu mode to enable 16-bit instrs
        regs.P.M = false; // M=0 so A is 16-bit
        regs.P.X = DUP_xf;
        regs.DUP_reg = 0x9999; // value which will be overwritten
        regs.DUP_idx = DUP_idx_val;
        regs.DB = 0xdb;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, DUP_opcode);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0xaa, "address low");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0xbb, "address high");
        if DUP_idle {
            expect_internal_cycle(&mut cpu, "indexing");
        }
        expect_read_cycle(&mut cpu, snes_addr!(0xdb:(0xbbaa + DUP_idx_val)), 0x44, "value low");
        *expected_regs.DUP_reg.lo_mut() = 0x44;

        if DUP_16 {
            expect_read_cycle(&mut cpu, snes_addr!(0xdb:0xbbaa + DUP_idx_val + 1), 0x33, "value high");
            *expected_regs.DUP_reg.hi_mut() = 0x33;
        }
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0x3459;
        assert_eq!(*cpu.regs(), expected_regs);
    }

    // duplicate for most direct addressing modes, which have an idle
    // cycle when DL != 0
    #[duplicate_item(
        DUP1_name           DUP1_D      DUP1_idle;
        [direct_no_idle]    [0x5000]    [false];
        [direct_idle]       [0x5050]    [true]; // idle when DL != 0
    )]
    mod DUP1_name {
        use super::*;

        // duplicate over all regs for direct addrmode
        #[duplicate_item(
            DUP2_name   DUP2_reg    DUP2_opcode;
            [lda_d]     [A]         [0xa5];
            [ldx_d]     [X]         [0xa6];
            [ldy_d]     [Y]         [0xa4];
        )]
        #[test]
        fn DUP2_name() {
            let mut regs = Registers::default();
            regs.PB = 0x12;
            regs.PC = 0x3456;
            regs.P.E = false; // non-emu mode to enable 16-bit instrs
            regs.P.M = false; // M=0 so A is 16-bit
            regs.P.X = false; // X=0 so X and Y are 16-bit
            regs.DUP2_reg = 0x9999; // value which will be overwritten
            regs.D = DUP1_D;

            let mut expected_regs = regs.clone();
            let mut cpu = CPU::new(regs);

            expect_opcode_fetch(&mut cpu, DUP2_opcode);
            expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x40, "direct offset");
            if DUP1_idle {
                expect_internal_cycle(&mut cpu, "idle when DL != 0");
            }
            expect_load16_read(&mut cpu, snes_addr!(0:(DUP1_D + 0x40)), 0x4321);
            expect_opcode_fetch_cycle(&mut cpu);

            expected_regs.DUP2_reg = 0x4321;
            expected_regs.PC = 0x3458;
            assert_eq!(*cpu.regs(), expected_regs);
        }


        // duplicate for all direct indexed addrmodes
        #[duplicate_item(
            DUP2_name   DUP2_opcode     DUP2_reg    DUP2_index;
            [lda_dx]    [0xb5]          [A]         [X];
            [ldx_dy]    [0xb6]          [X]         [Y];
            [ldy_dx]    [0xb4]          [Y]         [X];
        )]
        #[test]
        fn DUP2_name() {
            let mut regs = Registers::default();
            regs.PB = 0x12;
            regs.PC = 0x3456;
            regs.P.E = false; // non-emu mode to enable 16-bit instrs
            regs.P.M = false; // M=0 so A is 16-bit
            regs.P.X = false; // X=0 so X and Y are 16-bit
            regs.DUP2_reg = 0x9999; // value which will be overwritten
            regs.DUP2_index = 0x10;
            regs.D = DUP1_D;

            let mut expected_regs = regs.clone();
            let mut cpu = CPU::new(regs);

            expect_opcode_fetch(&mut cpu, DUP2_opcode);
            expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x40, "direct offset");
            expect_internal_cycle(&mut cpu, "indexing");
            if DUP1_idle {
                expect_internal_cycle(&mut cpu, "idle when DL != 0");
            }
            expect_load16_read(&mut cpu, snes_addr!(0:(DUP1_D + 0x40 + 0x10)), 0x4321);
            expect_opcode_fetch_cycle(&mut cpu);

            expected_regs.DUP2_reg = 0x4321;
            expected_regs.PC = 0x3458;
            assert_eq!(*cpu.regs(), expected_regs);
        }

        // direct indirect only exists for LDA
        #[test]
        fn lda_dind() {
            let mut regs = Registers::default();
            regs.PB = 0x12;
            regs.PC = 0x3456;
            regs.P.E = false; // non-emu mode to enable 16-bit instrs
            regs.P.M = false; // M=0 so A is 16-bit
            regs.A = 0x9999; // value which will be overwritten
            regs.D = DUP1_D;
            regs.DB = 0xee;

            let mut expected_regs = regs.clone();
            let mut cpu = CPU::new(regs);
            
            expect_opcode_fetch(&mut cpu, 0xb2);
            expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x12, "direct offset");
            if DUP1_idle {
                expect_internal_cycle(&mut cpu, "idle when DL != 0");
            }
            expect_read_cycle(&mut cpu, snes_addr!(0:(DUP1_D + 0x12)), 0x88, "AAL");
            expect_read_cycle(&mut cpu, snes_addr!(0:(DUP1_D + 0x13)), 0x77, "AAH");
            expect_load16_read(&mut cpu, snes_addr!(0xee:0x7788), 0x1234);
            expect_opcode_fetch_cycle(&mut cpu);

            expected_regs.A = 0x1234;
            expected_regs.PC = 0x3458;
            assert_eq!(*cpu.regs(), expected_regs);
        }

        // direct x indirect only exists for LDA
        #[test]
        fn lda_dxind() {
            let mut regs = Registers::default();
            regs.PB = 0x12;
            regs.PC = 0x3456;
            regs.P.E = false; // non-emu mode to enable 16-bit instrs
            regs.P.M = false; // M=0 so A is 16-bit
            regs.A = 0x9999; // value which will be overwritten
            regs.D = DUP1_D;
            regs.X = 0x1020;
            regs.DB = 0xee;

            let mut expected_regs = regs.clone();
            let mut cpu = CPU::new(regs);
            
            expect_opcode_fetch(&mut cpu, 0xa1);
            expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x12, "direct offset");
            if DUP1_idle {
                expect_internal_cycle(&mut cpu, "idle when DL != 0");
            }
            expect_internal_cycle(&mut cpu, "indexing");
            expect_read_cycle(&mut cpu, snes_addr!(0:(DUP1_D + 0x12 + 0x1020)), 0x88, "AAL");
            expect_read_cycle(&mut cpu, snes_addr!(0:(DUP1_D + 0x13 + 0x1020)), 0x77, "AAH");
            expect_load16_read(&mut cpu, snes_addr!(0xee:0x7788), 0x1234);
            expect_opcode_fetch_cycle(&mut cpu);

            expected_regs.A = 0x1234;
            expected_regs.PC = 0x3458;
            assert_eq!(*cpu.regs(), expected_regs);
        }

        // direct indirect y only exists for LDA
        // duplicate over reasons why the cpu might idle for note 4
        #[duplicate_item(
            //            X flag  
            DUP2_name     DUP2_xf DUP2_y      DUP2_idle;
            [lda_dindy_1] [true]  [0x0020]    [false];
            [lda_dindy_2] [true]  [0x00ff]    [true];
            [lda_dindy_3] [false] [0x0020]    [true];
            [lda_dindy_4] [false] [0x0220]    [true];
        )]
        #[test]
        fn DUP2_name() {
            let mut regs = Registers::default();
            regs.PB = 0x12;
            regs.PC = 0x3456;
            regs.P.E = false; // non-emu mode to enable 16-bit instrs
            regs.P.M = false; // M=0 so A is 16-bit
            regs.P.X = DUP2_xf;
            regs.Y = DUP2_y;
            regs.A = 0x9999; // value which will be overwritten
            regs.D = DUP1_D;
            regs.DB = 0xee;

            let mut expected_regs = regs.clone();
            let mut cpu = CPU::new(regs);
            
            expect_opcode_fetch(&mut cpu, 0xb1);
            expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x12, "direct offset");
            if DUP1_idle {
                expect_internal_cycle(&mut cpu, "idle when DL != 0");
            }
            expect_read_cycle(&mut cpu, snes_addr!(0:(DUP1_D + 0x12)), 0x88, "AAL");
            expect_read_cycle(&mut cpu, snes_addr!(0:(DUP1_D + 0x13)), 0x77, "AAH");
            if DUP2_idle {
                expect_internal_cycle(&mut cpu, "indexing across page boundaries or P.X==0");
            }
            expect_load16_read(&mut cpu, snes_addr!(0xee:0x7788 + DUP2_y), 0x1234);
            expect_opcode_fetch_cycle(&mut cpu);

            expected_regs.A = 0x1234;
            expected_regs.PC = 0x3458;
            assert_eq!(*cpu.regs(), expected_regs);
        }

        // direct indirect long only exists for LDA
        #[test]
        fn lda_dindl() {
            let mut regs = Registers::default();
            regs.PB = 0x12;
            regs.PC = 0x3456;
            regs.P.E = false; // non-emu mode to enable 16-bit instrs
            regs.P.M = false; // M=0 so A is 16-bit
            regs.A = 0x9999; // value which will be overwritten
            regs.D = DUP1_D;

            let mut expected_regs = regs.clone();
            let mut cpu = CPU::new(regs);
            
            expect_opcode_fetch(&mut cpu, 0xa7);
            expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x12, "direct offset");
            if DUP1_idle {
                expect_internal_cycle(&mut cpu, "idle when DL != 0");
            }
            expect_read_cycle(&mut cpu, snes_addr!(0:(DUP1_D + 0x12)), 0x88, "AAL");
            expect_read_cycle(&mut cpu, snes_addr!(0:(DUP1_D + 0x13)), 0x77, "AAH");
            expect_read_cycle(&mut cpu, snes_addr!(0:(DUP1_D + 0x14)), 0x66, "AAB");
            expect_load16_read(&mut cpu, snes_addr!(0x66:0x7788), 0x1234);
            expect_opcode_fetch_cycle(&mut cpu);

            expected_regs.A = 0x1234;
            expected_regs.PC = 0x3458;
            assert_eq!(*cpu.regs(), expected_regs);
        }

        // direct indirect long y only exists for LDA
        #[test]
        fn lda_dindly() {
            let mut regs = Registers::default();
            regs.PB = 0x12;
            regs.PC = 0x3456;
            regs.P.E = false; // non-emu mode to enable 16-bit instrs
            regs.P.M = false; // M=0 so A is 16-bit
            regs.A = 0x9999; // value which will be overwritten
            regs.D = DUP1_D;
            regs.Y = 0x0303;

            let mut expected_regs = regs.clone();
            let mut cpu = CPU::new(regs);
            
            expect_opcode_fetch(&mut cpu, 0xb7);
            expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x12, "direct offset");
            if DUP1_idle {
                expect_internal_cycle(&mut cpu, "idle when DL != 0");
            }
            expect_read_cycle(&mut cpu, snes_addr!(0:(DUP1_D + 0x12)), 0x66, "AAL");
            expect_read_cycle(&mut cpu, snes_addr!(0:(DUP1_D + 0x13)), 0x55, "AAH");
            expect_read_cycle(&mut cpu, snes_addr!(0:(DUP1_D + 0x14)), 0x44, "AAB");
            expect_load16_read(&mut cpu, snes_addr!(0x44:0x5869), 0x1234);
            expect_opcode_fetch_cycle(&mut cpu);

            expected_regs.A = 0x1234;
            expected_regs.PC = 0x3458;
            assert_eq!(*cpu.regs(), expected_regs);
        }
    }

    // stack relative only exists for LDA
    #[test]
    fn lda_sr() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.E = false; // non-emu mode to enable 16-bit instrs
        regs.P.M = false; // M=0 so A is 16-bit
        regs.A = 0x9999; // value which will be overwritten
        regs.S = 0x0402;
            
        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xa3);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x80, "stack offset");
        expect_internal_cycle(&mut cpu, "indexing");
        expect_load16_read(&mut cpu, snes_addr!(0:0x0482), 0x4567);
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.A = 0x4567;
        expected_regs.PC = 0x3458;

        assert_eq!(*cpu.regs(), expected_regs);
    }

    // stack relative indirect Y only exists for LDA
    #[test]
    fn lda_sry() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        regs.P.E = false; // non-emu mode to enable 16-bit instrs
        regs.P.M = false; // M=0 so A is 16-bit
        regs.A = 0x9999; // value which will be overwritten
        regs.S = 0x0402;
        regs.DB = 0xdb;
        regs.Y = 0x3030;
            
        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0xb3);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x80, "stack offset");
        expect_internal_cycle(&mut cpu, "indexing");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x0482), 0xaa, "AAL");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x0483), 0xbb, "AAH");
        expect_internal_cycle(&mut cpu, "setting addrbus to abs address");
        expect_load16_read(&mut cpu, snes_addr!(0xdb:0xebda), 0x4567);
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.A = 0x4567;
        expected_regs.PC = 0x3458;

        assert_eq!(*cpu.regs(), expected_regs);
    }
}
