/// Direct page ALU instruction tests (feature/spc700-dp-alu)
///
/// One file for the entire PR — grows by operation family as each is
/// added: OR, AND, EOR, CMP, ADC, SBC, each across all addressing modes.
///
/// Currently covers:
///   - OR  family ($04,$05,$06,$07,$09,$14,$15,$16,$17,$18,$19)

use apu::cpu::{Spc700, FLAG_C, FLAG_N, FLAG_P, FLAG_V, FLAG_Z};
use apu::Memory;

// ============================================================
// Helper
// ============================================================

fn make() -> (Spc700, Memory) {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    mem.write8(0xFFFE, 0x00);
    mem.write8(0xFFFF, 0x02);
    cpu.reset(&mut mem);
    (cpu, mem)
}

// ============================================================
// OR A,dp ($04)
// ============================================================

#[test]
fn test_or_a_dp_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0F;
    mem.write8(0x0020, 0xF0);
    mem.write8(0x0200, 0x04);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xFF);
}

#[test]
fn test_or_a_dp_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x00;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x04);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_or_a_dp_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x00;
    mem.write8(0x0020, 0x80);
    mem.write8(0x0200, 0x04);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_or_a_dp_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0120, 0x01);
    mem.write8(0x0200, 0x04);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x01);
}

#[test]
fn test_or_a_dp_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x04);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// OR A,!abs ($05)
// ============================================================

#[test]
fn test_or_a_abs_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0F;
    mem.write8(0x0500, 0xF0);
    mem.write8(0x0200, 0x05);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xFF);
}

#[test]
fn test_or_a_abs_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x00;
    mem.write8(0x0500, 0x00);
    mem.write8(0x0200, 0x05);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_or_a_abs_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x05);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// OR A,(X) ($06)
// ============================================================

#[test]
fn test_or_a_ix_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0F;
    cpu.regs.x = 0x20;
    mem.write8(0x0020, 0xF0);
    mem.write8(0x0200, 0x06);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xFF);
}

#[test]
fn test_or_a_ix_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    cpu.regs.x = 0x20;
    mem.write8(0x0120, 0x01);
    mem.write8(0x0200, 0x06);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x01);
}

#[test]
fn test_or_a_ix_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x06);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// OR A,[dp+X] ($07)
// ============================================================

#[test]
fn test_or_a_dp_x_ind_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0F;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x00);
    mem.write8(0x0023, 0x05); // pointer → $0500
    mem.write8(0x0500, 0xF0);
    mem.write8(0x0200, 0x07);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xFF);
}

#[test]
fn test_or_a_dp_x_ind_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0200, 0x07);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// OR dd,ds ($09) — direct page to direct page
// ============================================================

#[test]
fn test_or_dp_dp_basic() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0xF0); // src
    mem.write8(0x0040, 0x0F); // dst
    mem.write8(0x0200, 0x09);
    mem.write8(0x0201, 0x30); // src offset
    mem.write8(0x0202, 0x40); // dst offset
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0040), 0xFF);
}

#[test]
fn test_or_dp_dp_does_not_modify_source() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0xF0);
    mem.write8(0x0040, 0x0F);
    mem.write8(0x0200, 0x09);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0030), 0xF0, "source must be unchanged");
}

#[test]
fn test_or_dp_dp_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x00);
    mem.write8(0x0040, 0x00);
    mem.write8(0x0200, 0x09);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_or_dp_dp_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x00);
    mem.write8(0x0040, 0x00);
    mem.write8(0x0200, 0x09);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// OR A,dp+X ($14)
// ============================================================

#[test]
fn test_or_a_dp_x_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0F;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0xF0);
    mem.write8(0x0200, 0x14);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xFF);
}

#[test]
fn test_or_a_dp_x_wraps_within_page() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0F;
    cpu.regs.x = 0x02;
    mem.write8(0x0001, 0xF0);
    mem.write8(0x0200, 0x14);
    mem.write8(0x0201, 0xFF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xFF);
}

#[test]
fn test_or_a_dp_x_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x14);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// OR A,!abs+X ($15) / OR A,!abs+Y ($16)
// ============================================================

#[test]
fn test_or_a_abs_x_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0F;
    cpu.regs.x = 0x02;
    mem.write8(0x0502, 0xF0);
    mem.write8(0x0200, 0x15);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xFF);
}

#[test]
fn test_or_a_abs_x_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x15);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_or_a_abs_y_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0F;
    cpu.regs.y = 0x03;
    mem.write8(0x0503, 0xF0);
    mem.write8(0x0200, 0x16);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xFF);
}

#[test]
fn test_or_a_abs_y_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x16);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// OR A,[dp]+Y ($17)
// ============================================================

#[test]
fn test_or_a_dp_ind_y_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0F;
    cpu.regs.y = 0x03;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05); // pointer → $0500
    mem.write8(0x0503, 0xF0); // $0500 + Y
    mem.write8(0x0200, 0x17);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xFF);
}

#[test]
fn test_or_a_dp_ind_y_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0200, 0x17);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// OR dp,#imm ($18)
// ============================================================

#[test]
fn test_or_dp_imm_basic() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x0F);
    mem.write8(0x0200, 0x18);
    mem.write8(0x0201, 0xF0); // immediate
    mem.write8(0x0202, 0x20); // dp offset
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0xFF);
}

#[test]
fn test_or_dp_imm_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x18);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_or_dp_imm_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x18);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// OR (X),(Y) ($19)
// ============================================================

#[test]
fn test_or_ix_iy_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    cpu.regs.y = 0x30;
    mem.write8(0x0020, 0x0F); // dst (X)
    mem.write8(0x0030, 0xF0); // src (Y)
    mem.write8(0x0200, 0x19);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0xFF);
}

#[test]
fn test_or_ix_iy_does_not_modify_y_address() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    cpu.regs.y = 0x30;
    mem.write8(0x0020, 0x0F);
    mem.write8(0x0030, 0xF0);
    mem.write8(0x0200, 0x19);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0030), 0xF0, "Y address must be unchanged");
}

#[test]
fn test_or_ix_iy_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x19);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}
