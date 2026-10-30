#![cfg(test)]
//! Advanced tests for accurate IRQ behaviour,
//! based on the following resources:
//! - https://wiki.superfamicom.org/timing#interrupts-28

use crate::{
    instrs::test_prelude::{
        CPU, Registers, expect_internal_cycle_with_irqb, expect_opcode_fetch_cycle,
        expect_opcode_fetch_with_irqb, expect_read_cycle_with_irqb, expect_write_cycle_with_irqb,
        snes_addr,
    },
    registers::RegisterP,
};

/// ```
/// ; set up IRQ
/// SEI
/// WAI
/// STZ $00
/// LDA #$01
/// CLI
/// LDA #$42
/// STP
///
/// IRQHandler:
/// STA $00
/// RTI
/// ```
///
/// This expects 0x42 to be in $00
#[test]
fn cli_executes_next_instr() {
    let mut irqb = true; // no IRQ request at the start

    let regs = Registers {
        PB: 0x10,
        PC: 0x1000,
        S: 0x0180,
        P: RegisterP {
            M: true,
            I: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut expected_regs = regs;
    let mut cpu = CPU::new(regs);

    // SEI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x78);
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "SEI");
    expected_regs.P.I = true;
    expected_regs.PC = 0x1001;
    assert_eq!(cpu.regs(), &expected_regs, "I should have been set by SEI");

    // WAI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0xCB);
    for i in 0..100 {
        expect_internal_cycle_with_irqb(&mut cpu, irqb, &format!("WAI waiting... ({i})"));
    }
    irqb = false;
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "WAI acknowledge IRQB cycle");
    expected_regs.PC = 0x1002;
    assert_eq!(cpu.regs(), &expected_regs);

    // STZ $0000
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x9C);
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0x10:0x1003), 0, "STZ addr lo");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0x10:0x1004), 0, "STZ addr hi");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0), 0, "STZ write");
    expected_regs.PC = 0x1005;
    assert_eq!(cpu.regs(), &expected_regs);

    // LDA #$01
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0xA9);
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0x10:0x1006), 1, "LDA imm op");
    expected_regs.PC = 0x1007;
    assert_eq!(cpu.regs(), &expected_regs);
    expected_regs.A = 0x0001; // A will only be set once the CPU gets one more cycle

    // CLI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x58);
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "CLI");
    expected_regs.P.I = false;
    expected_regs.PC = 0x1008;
    assert_eq!(
        cpu.regs(),
        &expected_regs,
        "I should have been cleared by CLI"
    );

    // LDA #$42
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0xA9);
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0x10:0x1009), 0x42, "LDA imm op");
    expected_regs.PC = 0x100A;
    assert_eq!(cpu.regs(), &expected_regs);
    expected_regs.A = 0x0042; // A will only be set once the CPU gets one more cycle

    // IRQ setup
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0180), 0x10, "push PB");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x017F), 0x10, "push PCH");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x017E), 0x0A, "push PCL");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x017D), 0b00100000, "push P");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0xFFEE), 0x00, "IRQ vec lo");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0xFFEF), 0x20, "IRQ vec hi");
    expected_regs.P.I = true; // interrupt handlers sets I to avoid getting IRQs immediately again
    expected_regs.PC = 0x1000; // only lo byte of PC has been fetched yet
    expected_regs.S = 0x017C;
    assert_eq!(cpu.regs(), &expected_regs);
    expected_regs.PC = 0x2000; // now PC has been fully fetched
    expected_regs.PB = 0; // interrupt handlers are in bank 0

    // STA $0000
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x8D);
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x2001), 0, "STA addr lo");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x2002), 0, "STA addr hi");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0), 0x42, "STA write");
    expected_regs.PC = 0x2003;
    assert_eq!(cpu.regs(), &expected_regs);

    // RTI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x40);
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "RTI idle 1");
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "RTI idle 2");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x017D), 0b00100000, "pull P");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x017E), 0x0A, "pull PCL");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x017F), 0x10, "pull PCH");
    irqb = true; // stop requesting IRQ, otherwise since we fetched I=0 we would IRQ again
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0180), 0x10, "pull PB");
    expected_regs.P.I = false;
    expected_regs.PC = 0x100A;
    expected_regs.S = 0x0180;
    assert_eq!(cpu.regs(), &expected_regs);
    expected_regs.PB = 0x10;

    expect_opcode_fetch_cycle(&mut cpu);
    assert_eq!(cpu.regs(), &expected_regs);
}

/// ```
/// ; set up IRQ
/// SEI
/// WAI
/// CLI
/// SEI
/// ```
///
/// Here the IRQ should trigger after the SEI, and push I=1 to the stack
#[test]
fn cli_sei_pushes_i_set() {
    let mut irqb = true; // no IRQ request at the start

    let regs = Registers {
        PB: 0x10,
        PC: 0x1000,
        S: 0x0180,
        P: RegisterP {
            M: true,
            I: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut expected_regs = regs;
    let mut cpu = CPU::new(regs);

    // SEI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x78);
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "SEI");
    expected_regs.P.I = true;
    expected_regs.PC = 0x1001;
    assert_eq!(cpu.regs(), &expected_regs, "I should have been set by SEI");

    // WAI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0xCB);
    for i in 0..100 {
        expect_internal_cycle_with_irqb(&mut cpu, irqb, &format!("WAI waiting... ({i})"));
    }
    irqb = false;
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "WAI acknowledge IRQB cycle");
    expected_regs.PC = 0x1002;
    assert_eq!(cpu.regs(), &expected_regs);

    // CLI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x58);
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "CLI");
    expected_regs.P.I = false;
    expected_regs.PC = 0x1003;
    assert_eq!(
        cpu.regs(),
        &expected_regs,
        "I should have been cleared by CLI"
    );

    // SEI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x78);
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "SEI");
    expected_regs.P.I = true;
    expected_regs.PC = 0x1004;
    assert_eq!(cpu.regs(), &expected_regs, "I should have been set by SEI");

    // IRQ setup
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0180), 0x10, "push PB");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x017F), 0x10, "push PCH");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x017E), 0x04, "push PCL");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x017D), 0b00100100, "push P");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0xFFEE), 0x00, "IRQ vec lo");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0xFFEF), 0x20, "IRQ vec hi");
    expected_regs.PC = 0x1000; // only hi byte of PC has been fetched yet
    expected_regs.S = 0x017C;
    assert_eq!(cpu.regs(), &expected_regs);
    expected_regs.PC = 0x2000; // now PC has been fully fetched
    expected_regs.PB = 0; // interrupt handlers are in bank 0

    expect_opcode_fetch_cycle(&mut cpu);
    assert_eq!(cpu.regs(), &expected_regs);
}

/// ```
/// ; set up IRQ
/// SEI
/// WAI
/// CLI
/// RTI
/// ```
/// Here we shouldn't get an IRQ at all (assuming RTI pulls I=1) because
/// RTI gets I way before its last cycle, and CLI only sets it during its
/// last cycle, and since I is only checked with IRQB at the very start
/// of the instructions' last cycle, there is no IRQ to do here: there is
/// no beginning of a last cycle where I=0, it's set then cleared fast enough
#[test]
fn cli_rti_might_not_irq() {
    let mut irqb = true; // no IRQ request at the start

    let regs = Registers {
        PB: 0x00,
        PC: 0x2000,
        S: 0x0180,
        P: RegisterP {
            I: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut expected_regs = regs;
    let mut cpu = CPU::new(regs);

    // SEI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x78);
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "SEI");
    expected_regs.P.I = true;
    expected_regs.PC = 0x2001;
    assert_eq!(cpu.regs(), &expected_regs, "I should have been set by SEI");

    // WAI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0xCB);
    for i in 0..100 {
        expect_internal_cycle_with_irqb(&mut cpu, irqb, &format!("WAI waiting... ({i})"));
    }
    irqb = false;
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "WAI acknowledge IRQB cycle");
    expected_regs.PC = 0x2002;
    assert_eq!(cpu.regs(), &expected_regs);

    // CLI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x58);
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "CLI");
    expected_regs.P.I = false;
    expected_regs.PC = 0x2003;
    assert_eq!(
        cpu.regs(),
        &expected_regs,
        "I should have been cleared by CLI"
    );

    // RTI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x40);
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "RTI idle 1");
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "RTI idle 2");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0181), 0b00000100, "pull P");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0182), 0x00, "pull PCL");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0183), 0x10, "pull PCH");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0184), 0x10, "pull PB");
    expected_regs.P.I = true; // we pulled I=1
    expected_regs.PC = 0x1000;
    expected_regs.S = 0x0184;
    assert_eq!(cpu.regs(), &expected_regs);
    expected_regs.PB = 0x10;

    expect_opcode_fetch_cycle(&mut cpu);
    assert_eq!(cpu.regs(), &expected_regs);
}

/// ```
/// ; set up IRQ
/// SEI
/// WAI
/// STZ $00
/// LDA #$01
/// RTI
///
/// ; -> RTI returns here
/// LDA #$42
/// STP
///
/// IRQHandler:
/// STA $00
/// RTI
/// ```
#[test]
fn rti_triggers_irq() {
    let mut irqb = true; // no IRQ request at the start

    let regs = Registers {
        PB: 0x00,
        PC: 0x2000,
        S: 0x0180,
        P: RegisterP {
            M: true,
            I: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut expected_regs = regs;
    let mut cpu = CPU::new(regs);

    // SEI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x78);
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "SEI");
    expected_regs.P.I = true;
    expected_regs.PC = 0x2001;
    assert_eq!(cpu.regs(), &expected_regs, "I should have been set by SEI");

    // WAI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0xCB);
    for i in 0..100 {
        expect_internal_cycle_with_irqb(&mut cpu, irqb, &format!("WAI waiting... ({i})"));
    }
    irqb = false;
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "WAI acknowledge IRQB cycle");
    expected_regs.PC = 0x2002;
    assert_eq!(cpu.regs(), &expected_regs);

    // STZ $0000
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x9C);
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x2003), 0, "STZ addr lo");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x2004), 0, "STZ addr hi");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0), 0, "STZ write");
    expected_regs.PC = 0x2005;
    assert_eq!(cpu.regs(), &expected_regs);

    // LDA #$01
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0xA9);
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x2006), 1, "LDA imm op");
    expected_regs.PC = 0x2007;
    assert_eq!(cpu.regs(), &expected_regs);
    expected_regs.A = 0x0001; // A will only be set once the CPU gets one more cycle

    // RTI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x40);
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "RTI idle 1");
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "RTI idle 2");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0181), 0b00100000, "pull P");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0182), 0x00, "pull PCL");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0183), 0x10, "pull PCH");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0184), 0x10, "pull PB");
    expected_regs.P.I = false; // we pulled I=0
    expected_regs.PC = 0x1000;
    expected_regs.S = 0x0184;
    assert_eq!(cpu.regs(), &expected_regs);
    expected_regs.PB = 0x10;

    // IRQ setup
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0184), 0x10, "push PB");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0183), 0x10, "push PCH");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0182), 0x00, "push PCL");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0181), 0b00100000, "push P");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0xFFEE), 0x10, "IRQ vec lo");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0xFFEF), 0x30, "IRQ vec hi");
    expected_regs.PC = 0x1010; // only lo byte of PC has been fetched yet
    expected_regs.S = 0x0180;
    expected_regs.P.I = true; // IRQ setup sets I to avoid setting up IRQs indefinitely
    assert_eq!(cpu.regs(), &expected_regs);
    expected_regs.PC = 0x3010; // now PC has been fully fetched
    expected_regs.PB = 0; // interrupt handlers are in bank 0

    // STA $0000
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x8D);
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x3011), 0, "STA addr lo");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x3012), 0, "STA addr hi");
    expect_write_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0), 1, "STA write");
    expected_regs.PC = 0x3013;
    assert_eq!(cpu.regs(), &expected_regs);

    // RTI
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0x40);
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "RTI idle 1");
    expect_internal_cycle_with_irqb(&mut cpu, irqb, "RTI idle 2");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0181), 0b00100100, "pull P");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0182), 0x00, "pull PCL");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0183), 0x10, "pull PCH");
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0:0x0184), 0x10, "pull PB");
    expected_regs.P.I = true; // we pulled I=0
    expected_regs.PC = 0x1000;
    expected_regs.S = 0x0184;
    assert_eq!(cpu.regs(), &expected_regs);
    expected_regs.PB = 0x10;

    // LDA #$42
    expect_opcode_fetch_with_irqb(&mut cpu, irqb, 0xA9);
    expect_read_cycle_with_irqb(&mut cpu, irqb, snes_addr!(0x10:0x1001), 0x42, "LDA imm op");
    expected_regs.PC = 0x1002;
    assert_eq!(cpu.regs(), &expected_regs);
    expected_regs.A = 0x0042; // A will only be set once the CPU gets one more cycle

    expect_opcode_fetch_cycle(&mut cpu);
    assert_eq!(cpu.regs(), &expected_regs);
}

