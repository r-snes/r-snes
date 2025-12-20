use instr_metalang_procmacro::cpu_instr_no_inc_pc;
use duplicate::duplicate;

duplicate! {
    [
        DUP_name    DUP_flag;
        [bcs]       [ cpu.registers.P.C]; // Branch if Carry Set
        [bcc]       [!cpu.registers.P.C]; // Branch if Carry Clear
        [beq]       [ cpu.registers.P.Z]; // Branch if EQual
        [bne]       [!cpu.registers.P.Z]; // Branch if Not Equal
        [bmi]       [ cpu.registers.P.N]; // Branch if MInus
        [bpl]       [!cpu.registers.P.N]; // Branch if PLus
        [bvs]       [ cpu.registers.P.V]; // Branch if oVerflow Set
        [bvc]       [!cpu.registers.P.V]; // Branch if oVerflow Clear
        [bra]       [true]; // BRanch Always
    ]
    cpu_instr_no_inc_pc!(DUP_name {
        meta FETCH8_IMM;

        // manually inc PC to where it would be for the next opcode
        cpu.registers.PC = cpu.registers.PC.wrapping_add(2);

        meta IDLE_IF DUP_flag; // idle if the branch is taken (cpu doc note 5)
        if DUP_flag {
            // when branching, save old PC before overwriting to check page boundary crossing
            cpu.internal_data_bus = cpu.registers.PC;
            // offset PC by the read value as a signed number
            cpu.registers.PC = cpu.registers.PC.wrapping_add(cpu.data_bus as i8 as u16);
        }

        // idle if the branch is taken across a page boundary (cpu doc note 6)
        meta IDLE_IF DUP_flag
            && cpu.registers.P.E
            && *cpu.internal_data_bus.hi() != *cpu.registers.PC.hi();
    });
}

// BRanch Long (unconditionally)
cpu_instr_no_inc_pc!(brl {
    meta FETCH16_IMM_INTO cpu.internal_data_bus;

    // manually inc PC to where it would be for the next opcode
    cpu.registers.PC = cpu.registers.PC.wrapping_add(3);

    cpu.registers.PC = cpu.registers.PC.wrapping_add(cpu.internal_data_bus);
    meta END_CYCLE Internal;
});

#[cfg(test)]
mod test {
    use super::super::test_prelude::*;
    use duplicate::duplicate_item;

    // duplicate for all branch instructions
    #[duplicate_item(
        DUP1_name   DUP1_opcode DUP1_flag   DUP1_set;
        [bcs]       [0xb0]      [regs.P.C]  [true];
        [bcc]       [0x90]      [regs.P.C]  [false];
        [beq]       [0xf0]      [regs.P.Z]  [true];
        [bne]       [0xd0]      [regs.P.Z]  [false];
        [bmi]       [0x30]      [regs.P.N]  [true];
        [bpl]       [0x10]      [regs.P.N]  [false];
        [bvs]       [0x70]      [regs.P.V]  [true];
        [bvc]       [0x50]      [regs.P.V]  [false];
        [bra]       [0x80]      [let _]     [0]; // for BRA, don't even set anything
    )]
    mod DUP1_name {
        use super::*;

        #[test]
        fn branch_not_taken() {
            if DUP1_opcode == 0x80 {
                return; // always pass test for BRA, it never takes a branch
            }

            let mut regs = Registers::default();
            regs.PB = 0x12;
            regs.PC = 0x3456;

            DUP1_flag = !DUP1_set; // branch not taken

            let mut expected_regs = regs.clone();
            let mut cpu = CPU::new(regs);

            expect_opcode_fetch(&mut cpu, DUP1_opcode);
            expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0xe1, "jump offset");
            // branch is not taken, straight to opcode fetch
            expect_opcode_fetch_cycle(&mut cpu);

            expected_regs.PC = 0x3458; // just go to next instruction, no jump
            assert_eq!(*cpu.regs(), expected_regs);
        }

        #[test]
        fn branch_taken_no_page_crossed() {
            let mut regs = Registers::default();
            regs.PB = 0x12;
            regs.PC = 0x3456;

            DUP1_flag = DUP1_set; // case where we do jump

            let mut expected_regs = regs.clone();
            let mut cpu = CPU::new(regs);

            expect_opcode_fetch(&mut cpu, DUP1_opcode);
            expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x30, "jump offset");
            expect_internal_cycle(&mut cpu, "branch taken");
            // no more idle, no page boundary crossed
            expect_opcode_fetch_cycle(&mut cpu);

            expected_regs.PC = 0x3488;
            assert_eq!(*cpu.regs(), expected_regs);
        }

        // duplicate over emu/non-emu: idle only in emu
        #[duplicate_item(
            DUP2_name                       DUP2_emu;
            [branch_taken_page_crossed_emu] [true];
            [branch_taken_page_crossed_nat] [false];
        )]
        #[test]
        fn DUP2_name() {
            let mut regs = Registers::default();
            regs.PB = 0x12;
            regs.PC = 0x3456;

            DUP1_flag = DUP1_set; // case where we do jump
            regs.P.E = DUP2_emu;

            let mut expected_regs = regs.clone();
            let mut cpu = CPU::new(regs);

            println!("pre opc: {:#?}", cpu.regs());
            expect_opcode_fetch(&mut cpu, DUP1_opcode);
            println!("pre offs: {:#?}", cpu.regs());
            // we jump to 0x60 lower, crossing a page boundary
            expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), -0x60_i8 as u8, "jump offset");
            println!("pre bt: {:#?}", cpu.regs());
            expect_internal_cycle(&mut cpu, "branch taken");
            println!("pre cond/opc: {:#?}", cpu.regs());
            if DUP2_emu {
                expect_internal_cycle(&mut cpu, "branch taken across page boundary");
                println!("pre opc2: {:#?}", cpu.regs());
            }
            expect_opcode_fetch_cycle(&mut cpu);
            println!("post_opc2: {:#?}", cpu.regs());

            expected_regs.PC = 0x33f8;
            assert_eq!(*cpu.regs(), expected_regs);
        }
    }

    #[test]
    fn brl() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;

        let mut expected_regs = regs.clone();
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x82);
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3457), 0x30, "offset low");
        expect_read_cycle(&mut cpu, snes_addr!(0x12:0x3458), 0x70, "offset high");
        expect_internal_cycle(&mut cpu, "jumping");
        expect_opcode_fetch_cycle(&mut cpu);

        expected_regs.PC = 0xa489;
        assert_eq!(*cpu.regs(), expected_regs);
    }
}
