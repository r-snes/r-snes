use duplicate::duplicate;
use instr_metalang_procmacro::cpu_instr_no_inc_pc;

duplicate! {
    [
        DUP_name    DUP_flag                DUP_attr;
        [bcs]       [ cpu.registers.P.C]    []; // Branch if Carry Set
        [bcc]       [!cpu.registers.P.C]    []; // Branch if Carry Clear
        [beq]       [ cpu.registers.P.Z]    []; // Branch if EQual
        [bne]       [!cpu.registers.P.Z]    []; // Branch if Not Equal
        [bmi]       [ cpu.registers.P.N]    []; // Branch if MInus
        [bpl]       [!cpu.registers.P.N]    []; // Branch if PLus
        [bvs]       [ cpu.registers.P.V]    []; // Branch if oVerflow Set
        [bvc]       [!cpu.registers.P.V]    []; // Branch if oVerflow Clear
        [bra]       [true]                  [#[expect(clippy::nonminimal_bool, reason = "false positive for BRA")]]; // BRanch Always
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
        DUP_attr
        meta IDLE_IF DUP_flag
            && cpu.registers.E
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
    #![expect(
        clippy::nonminimal_bool,
        reason = "`DUP1_set is !true or !false in the duplicated tests"
    )]
    use super::super::test_prelude::*;
    use duplicate::{duplicate, duplicate_item};

    // duplicate for all branch instructions
    duplicate! {
        [
            DUP1_name   DUP1_opcode DUP1_jump(do_jump);
            [bcs]       [0xb0]      [C: do_jump,];
            [bcc]       [0x90]      [C: !do_jump,];
            [beq]       [0xf0]      [Z: do_jump,];
            [bne]       [0xd0]      [Z: !do_jump,];
            [bmi]       [0x30]      [N: do_jump,];
            [bpl]       [0x10]      [N: !do_jump,];
            [bvs]       [0x70]      [V: do_jump,];
            [bvc]       [0x50]      [V: !do_jump,];
            [bra]       [0x80]      []; // for BRA, don't even set anything
        ]
        mod DUP1_name {
            use crate::registers::RegisterP;
            use super::*;

            #[test]
            fn branch_not_taken() {
                if DUP1_opcode == 0x80 {
                    return; // always pass test for BRA, it never takes a branch
                }

                let regs = Registers {
                    PB: 0x12,
                    PC: 0x3456,
                    P: RegisterP {
                        DUP1_jump([false])
                        ..0.into()
                    },
                    ..Default::default()
                };

                let mut expected_regs = regs;
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
                let regs = Registers {
                    PB: 0x12,
                    PC: 0x3456,
                    P: RegisterP {
                        DUP1_jump([true])
                        ..0.into()
                    },
                    ..Default::default()
                };

                let mut expected_regs = regs;
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
                let regs = Registers {
                    PB: 0x12,
                    PC: 0x3456,
                    P: RegisterP {
                        DUP1_jump([true])
                        ..0.into()
                    },
                    E: DUP2_emu,
                    ..Default::default()
                };

                let mut expected_regs = regs;
                let mut cpu = CPU::new(regs);

                expect_opcode_fetch(&mut cpu, DUP1_opcode);
                // we jump to 0x60 lower, crossing a page boundary
                expect_read_cycle(
                    &mut cpu,
                    snes_addr!(0x12:0x3457),
                    -0x60_i8 as u8,
                    "jump offset",
                );
                expect_internal_cycle(&mut cpu, "branch taken");
                if DUP2_emu {
                    expect_internal_cycle(&mut cpu, "branch taken across page boundary");
                }
                expect_opcode_fetch_cycle(&mut cpu);

                expected_regs.PC = 0x33f8;
                assert_eq!(*cpu.regs(), expected_regs);
            }
        }
    }

    #[test]
    fn brl() {
        let regs = Registers {
            PB: 0x12,
            PC: 0x3456,
            ..Default::default()
        };

        let mut expected_regs = regs;
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
