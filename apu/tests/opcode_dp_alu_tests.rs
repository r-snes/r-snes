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

// ============================================================
// AND A,dp ($24)
// ============================================================

#[test]
fn test_and_a_dp_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0020, 0x0F);
    mem.write8(0x0200, 0x24);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_and_a_dp_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xF0;
    mem.write8(0x0020, 0x0F);
    mem.write8(0x0200, 0x24);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_and_a_dp_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0020, 0x80);
    mem.write8(0x0200, 0x24);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_and_a_dp_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    cpu.regs.a = 0xFF;
    mem.write8(0x0120, 0x01);
    mem.write8(0x0200, 0x24);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x01);
}

#[test]
fn test_and_a_dp_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x24);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// AND A,!abs ($25)
// ============================================================

#[test]
fn test_and_a_abs_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0500, 0x0F);
    mem.write8(0x0200, 0x25);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_and_a_abs_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x25);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// AND A,(X) ($26)
// ============================================================

#[test]
fn test_and_a_ix_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x20;
    mem.write8(0x0020, 0x0F);
    mem.write8(0x0200, 0x26);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_and_a_ix_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x26);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// AND A,[dp+X] ($27)
// ============================================================

#[test]
fn test_and_a_dp_x_ind_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x00);
    mem.write8(0x0023, 0x05);
    mem.write8(0x0500, 0x0F);
    mem.write8(0x0200, 0x27);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_and_a_dp_x_ind_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0200, 0x27);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// AND dd,ds ($29)
// ============================================================

#[test]
fn test_and_dp_dp_basic() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x0F);
    mem.write8(0x0040, 0xFF);
    mem.write8(0x0200, 0x29);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0040), 0x0F);
}

#[test]
fn test_and_dp_dp_does_not_modify_source() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x0F);
    mem.write8(0x0040, 0xFF);
    mem.write8(0x0200, 0x29);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0030), 0x0F);
}

#[test]
fn test_and_dp_dp_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x00);
    mem.write8(0x0040, 0x00);
    mem.write8(0x0200, 0x29);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// AND A,dp+X ($34)
// ============================================================

#[test]
fn test_and_a_dp_x_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x0F);
    mem.write8(0x0200, 0x34);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_and_a_dp_x_wraps_within_page() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x02;
    mem.write8(0x0001, 0x0F);
    mem.write8(0x0200, 0x34);
    mem.write8(0x0201, 0xFF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_and_a_dp_x_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x34);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// AND A,!abs+X ($35) / AND A,!abs+Y ($36)
// ============================================================

#[test]
fn test_and_a_abs_x_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x02;
    mem.write8(0x0502, 0x0F);
    mem.write8(0x0200, 0x35);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_and_a_abs_x_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x35);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_and_a_abs_y_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.y = 0x03;
    mem.write8(0x0503, 0x0F);
    mem.write8(0x0200, 0x36);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_and_a_abs_y_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x36);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// AND A,[dp]+Y ($37)
// ============================================================

#[test]
fn test_and_a_dp_ind_y_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.y = 0x03;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0503, 0x0F);
    mem.write8(0x0200, 0x37);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_and_a_dp_ind_y_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0200, 0x37);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// AND dp,#imm ($38)
// ============================================================

#[test]
fn test_and_dp_imm_basic() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0x38);
    mem.write8(0x0201, 0x0F);
    mem.write8(0x0202, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x0F);
}

#[test]
fn test_and_dp_imm_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xF0);
    mem.write8(0x0200, 0x38);
    mem.write8(0x0201, 0x0F);
    mem.write8(0x0202, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_and_dp_imm_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x38);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// AND (X),(Y) ($39)
// ============================================================

#[test]
fn test_and_ix_iy_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    cpu.regs.y = 0x30;
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0030, 0x0F);
    mem.write8(0x0200, 0x39);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x0F);
}

#[test]
fn test_and_ix_iy_does_not_modify_y_address() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    cpu.regs.y = 0x30;
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0030, 0x0F);
    mem.write8(0x0200, 0x39);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0030), 0x0F);
}

#[test]
fn test_and_ix_iy_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x39);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// EOR A,dp ($44)
// ============================================================

#[test]
fn test_eor_a_dp_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0020, 0x0F);
    mem.write8(0x0200, 0x44);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xF0);
}

#[test]
fn test_eor_a_dp_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0x44);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_eor_a_dp_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x00;
    mem.write8(0x0020, 0x80);
    mem.write8(0x0200, 0x44);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_eor_a_dp_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    cpu.regs.a = 0xFF;
    mem.write8(0x0120, 0x0F);
    mem.write8(0x0200, 0x44);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xF0);
}

#[test]
fn test_eor_a_dp_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x44);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// EOR A,!abs ($45)
// ============================================================

#[test]
fn test_eor_a_abs_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0500, 0x0F);
    mem.write8(0x0200, 0x45);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xF0);
}

#[test]
fn test_eor_a_abs_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x45);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// EOR A,(X) ($46)
// ============================================================

#[test]
fn test_eor_a_ix_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x20;
    mem.write8(0x0020, 0x0F);
    mem.write8(0x0200, 0x46);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xF0);
}

#[test]
fn test_eor_a_ix_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x46);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// EOR A,[dp+X] ($47)
// ============================================================

#[test]
fn test_eor_a_dp_x_ind_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x00);
    mem.write8(0x0023, 0x05);
    mem.write8(0x0500, 0x0F);
    mem.write8(0x0200, 0x47);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xF0);
}

#[test]
fn test_eor_a_dp_x_ind_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0200, 0x47);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// EOR dd,ds ($49)
// ============================================================

#[test]
fn test_eor_dp_dp_basic() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x0F);
    mem.write8(0x0040, 0xFF);
    mem.write8(0x0200, 0x49);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0040), 0xF0);
}

#[test]
fn test_eor_dp_dp_does_not_modify_source() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x0F);
    mem.write8(0x0040, 0xFF);
    mem.write8(0x0200, 0x49);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0030), 0x0F);
}

#[test]
fn test_eor_dp_dp_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x00);
    mem.write8(0x0040, 0x00);
    mem.write8(0x0200, 0x49);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// EOR A,dp+X ($54)
// ============================================================

#[test]
fn test_eor_a_dp_x_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x0F);
    mem.write8(0x0200, 0x54);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xF0);
}

#[test]
fn test_eor_a_dp_x_wraps_within_page() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x02;
    mem.write8(0x0001, 0x0F);
    mem.write8(0x0200, 0x54);
    mem.write8(0x0201, 0xFF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xF0);
}

#[test]
fn test_eor_a_dp_x_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x54);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// EOR A,!abs+X ($55) / EOR A,!abs+Y ($56)
// ============================================================

#[test]
fn test_eor_a_abs_x_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x02;
    mem.write8(0x0502, 0x0F);
    mem.write8(0x0200, 0x55);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xF0);
}

#[test]
fn test_eor_a_abs_x_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x55);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_eor_a_abs_y_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.y = 0x03;
    mem.write8(0x0503, 0x0F);
    mem.write8(0x0200, 0x56);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xF0);
}

#[test]
fn test_eor_a_abs_y_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x56);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// EOR A,[dp]+Y ($57)
// ============================================================

#[test]
fn test_eor_a_dp_ind_y_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.y = 0x03;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0503, 0x0F);
    mem.write8(0x0200, 0x57);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xF0);
}

#[test]
fn test_eor_a_dp_ind_y_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0200, 0x57);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// EOR dp,#imm ($58)
// ============================================================

#[test]
fn test_eor_dp_imm_basic() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0x58);
    mem.write8(0x0201, 0x0F);
    mem.write8(0x0202, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0xF0);
}

#[test]
fn test_eor_dp_imm_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0x58);
    mem.write8(0x0201, 0xFF);
    mem.write8(0x0202, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_eor_dp_imm_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x58);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// EOR (X),(Y) ($59)
// ============================================================

#[test]
fn test_eor_ix_iy_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    cpu.regs.y = 0x30;
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0030, 0x0F);
    mem.write8(0x0200, 0x59);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0xF0);
}

#[test]
fn test_eor_ix_iy_does_not_modify_y_address() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    cpu.regs.y = 0x30;
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0030, 0x0F);
    mem.write8(0x0200, 0x59);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0030), 0x0F);
}

#[test]
fn test_eor_ix_iy_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x59);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// CMP A,dp ($64)
// ============================================================

#[test]
fn test_cmp_a_dp_equal_sets_zero_and_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    mem.write8(0x0020, 0x10);
    mem.write8(0x0200, 0x64);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_a_dp_greater_sets_carry_not_zero() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x20;
    mem.write8(0x0020, 0x10);
    mem.write8(0x0200, 0x64);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_cmp_a_dp_less_clears_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x05;
    mem.write8(0x0020, 0x10);
    mem.write8(0x0200, 0x64);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_a_dp_does_not_modify_a() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x20;
    mem.write8(0x0020, 0x10);
    mem.write8(0x0200, 0x64);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x20, "CMP must not modify A");
}

#[test]
fn test_cmp_a_dp_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x64);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// CMP A,!abs ($65)
// ============================================================

#[test]
fn test_cmp_a_abs_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x20;
    mem.write8(0x0500, 0x10);
    mem.write8(0x0200, 0x65);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_a_abs_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x65);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// CMP A,(X) ($66)
// ============================================================

#[test]
fn test_cmp_a_ix_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x20;
    cpu.regs.x = 0x20;
    mem.write8(0x0020, 0x10);
    mem.write8(0x0200, 0x66);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_a_ix_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x66);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// CMP A,[dp+X] ($67)
// ============================================================

#[test]
fn test_cmp_a_dp_x_ind_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x20;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x00);
    mem.write8(0x0023, 0x05);
    mem.write8(0x0500, 0x10);
    mem.write8(0x0200, 0x67);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_a_dp_x_ind_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0200, 0x67);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// CMP dd,ds ($69)
// ============================================================

#[test]
fn test_cmp_dp_dp_basic() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x10); // src
    mem.write8(0x0040, 0x20); // dst
    mem.write8(0x0200, 0x69);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_dp_dp_does_not_write_back() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x10);
    mem.write8(0x0040, 0x20);
    mem.write8(0x0200, 0x69);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0040), 0x20, "dst memory must be unchanged");
}

#[test]
fn test_cmp_dp_dp_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x00);
    mem.write8(0x0040, 0x00);
    mem.write8(0x0200, 0x69);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// CMP A,dp+X ($74)
// ============================================================

#[test]
fn test_cmp_a_dp_x_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x20;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x10);
    mem.write8(0x0200, 0x74);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_a_dp_x_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x74);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// CMP A,!abs+X ($75) / CMP A,!abs+Y ($76)
// ============================================================

#[test]
fn test_cmp_a_abs_x_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x20;
    cpu.regs.x = 0x02;
    mem.write8(0x0502, 0x10);
    mem.write8(0x0200, 0x75);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_a_abs_x_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x75);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_cmp_a_abs_y_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x20;
    cpu.regs.y = 0x03;
    mem.write8(0x0503, 0x10);
    mem.write8(0x0200, 0x76);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_a_abs_y_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x76);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// CMP A,[dp]+Y ($77)
// ============================================================

#[test]
fn test_cmp_a_dp_ind_y_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x20;
    cpu.regs.y = 0x03;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0503, 0x10);
    mem.write8(0x0200, 0x77);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_a_dp_ind_y_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0200, 0x77);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// CMP dp,#imm ($78)
// ============================================================

#[test]
fn test_cmp_dp_imm_basic() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x20);
    mem.write8(0x0200, 0x78);
    mem.write8(0x0201, 0x10); // immediate
    mem.write8(0x0202, 0x20); // dp offset
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_dp_imm_does_not_write_back() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x20);
    mem.write8(0x0200, 0x78);
    mem.write8(0x0201, 0x10);
    mem.write8(0x0202, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x20, "dp memory must be unchanged");
}

#[test]
fn test_cmp_dp_imm_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x78);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// CMP (X),(Y) ($79)
// ============================================================

#[test]
fn test_cmp_ix_iy_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    cpu.regs.y = 0x30;
    mem.write8(0x0020, 0x20);
    mem.write8(0x0030, 0x10);
    mem.write8(0x0200, 0x79);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_ix_iy_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x79);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// CMP X,#imm ($C8) / CMP Y,#imm ($AD)
// ============================================================

#[test]
fn test_cmp_x_imm_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    mem.write8(0x0200, 0xC8);
    mem.write8(0x0201, 0x10);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_x_imm_does_not_modify_x() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    mem.write8(0x0200, 0xC8);
    mem.write8(0x0201, 0x10);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x20);
}

#[test]
fn test_cmp_x_imm_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xC8);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_cmp_y_imm_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x20;
    mem.write8(0x0200, 0xAD);
    mem.write8(0x0201, 0x10);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_y_imm_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xAD);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

// ============================================================
// CMP X,dp ($3E) / CMP X,!abs ($1E)
// ============================================================

#[test]
fn test_cmp_x_dp_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    mem.write8(0x0020, 0x10);
    mem.write8(0x0200, 0x3E);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_x_dp_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x3E);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

#[test]
fn test_cmp_x_abs_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    mem.write8(0x0500, 0x10);
    mem.write8(0x0200, 0x1E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_x_abs_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x1E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// CMP Y,dp ($7E) / CMP Y,!abs ($5E)
// ============================================================

#[test]
fn test_cmp_y_dp_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x20;
    mem.write8(0x0020, 0x10);
    mem.write8(0x0200, 0x7E);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_y_dp_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x7E);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

#[test]
fn test_cmp_y_abs_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x20;
    mem.write8(0x0500, 0x10);
    mem.write8(0x0200, 0x5E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmp_y_abs_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x5E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// ADC A,dp ($84)
// ============================================================

#[test]
fn test_adc_a_dp_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    mem.write8(0x0020, 0x05);
    mem.write8(0x0200, 0x84);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x15);
}

#[test]
fn test_adc_a_dp_adds_carry_in() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x10;
    mem.write8(0x0020, 0x05);
    mem.write8(0x0200, 0x84);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x16);
}

#[test]
fn test_adc_a_dp_sets_carry_on_overflow() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x84);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x00);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_adc_a_dp_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x84);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_adc_a_dp_sets_overflow_flag() {
    // $7F + $01 = $80 — pos+pos=neg, signed overflow
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x7F;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x84);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_V));
}

#[test]
fn test_adc_a_dp_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    cpu.regs.a = 0x10;
    mem.write8(0x0120, 0x05);
    mem.write8(0x0200, 0x84);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x15);
}

#[test]
fn test_adc_a_dp_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x84);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// ADC A,!abs ($85)
// ============================================================

#[test]
fn test_adc_a_abs_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    mem.write8(0x0500, 0x05);
    mem.write8(0x0200, 0x85);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x15);
}

#[test]
fn test_adc_a_abs_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x85);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// ADC A,(X) ($86)
// ============================================================

#[test]
fn test_adc_a_ix_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    cpu.regs.x = 0x20;
    mem.write8(0x0020, 0x05);
    mem.write8(0x0200, 0x86);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x15);
}

#[test]
fn test_adc_a_ix_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x86);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// ADC A,[dp+X] ($87)
// ============================================================

#[test]
fn test_adc_a_dp_x_ind_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x00);
    mem.write8(0x0023, 0x05);
    mem.write8(0x0500, 0x05);
    mem.write8(0x0200, 0x87);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x15);
}

#[test]
fn test_adc_a_dp_x_ind_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0200, 0x87);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// ADC dd,ds ($89)
// ============================================================

#[test]
fn test_adc_dp_dp_basic() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x05); // src
    mem.write8(0x0040, 0x10); // dst
    mem.write8(0x0200, 0x89);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0040), 0x15);
}

#[test]
fn test_adc_dp_dp_does_not_modify_source() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x05);
    mem.write8(0x0040, 0x10);
    mem.write8(0x0200, 0x89);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0030), 0x05);
}

#[test]
fn test_adc_dp_dp_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x00);
    mem.write8(0x0040, 0x00);
    mem.write8(0x0200, 0x89);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// ADC A,dp+X ($94)
// ============================================================

#[test]
fn test_adc_a_dp_x_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x05);
    mem.write8(0x0200, 0x94);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x15);
}

#[test]
fn test_adc_a_dp_x_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x94);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// ADC A,!abs+X ($95) / ADC A,!abs+Y ($96)
// ============================================================

#[test]
fn test_adc_a_abs_x_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    cpu.regs.x = 0x02;
    mem.write8(0x0502, 0x05);
    mem.write8(0x0200, 0x95);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x15);
}

#[test]
fn test_adc_a_abs_x_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x95);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_adc_a_abs_y_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    cpu.regs.y = 0x03;
    mem.write8(0x0503, 0x05);
    mem.write8(0x0200, 0x96);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x15);
}

#[test]
fn test_adc_a_abs_y_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x96);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// ADC A,[dp]+Y ($97)
// ============================================================

#[test]
fn test_adc_a_dp_ind_y_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    cpu.regs.y = 0x03;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0503, 0x05);
    mem.write8(0x0200, 0x97);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x15);
}

#[test]
fn test_adc_a_dp_ind_y_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0200, 0x97);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// ADC dp,#imm ($98)
// ============================================================

#[test]
fn test_adc_dp_imm_basic() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x10);
    mem.write8(0x0200, 0x98);
    mem.write8(0x0201, 0x05); // immediate
    mem.write8(0x0202, 0x20); // dp offset
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x15);
}

#[test]
fn test_adc_dp_imm_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x98);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// ADC (X),(Y) ($99)
// ============================================================

#[test]
fn test_adc_ix_iy_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    cpu.regs.y = 0x30;
    mem.write8(0x0020, 0x10); // dst (X)
    mem.write8(0x0030, 0x05); // src (Y)
    mem.write8(0x0200, 0x99);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x15);
}

#[test]
fn test_adc_ix_iy_does_not_modify_y_address() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    cpu.regs.y = 0x30;
    mem.write8(0x0020, 0x10);
    mem.write8(0x0030, 0x05);
    mem.write8(0x0200, 0x99);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0030), 0x05);
}

#[test]
fn test_adc_ix_iy_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x99);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// ADC half-carry (FLAG_H) regression coverage
// ============================================================

#[test]
fn test_adc_a_dp_sets_half_carry_on_nibble_overflow() {
    // $0F + $01 = $10 — carry out of bit 3
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0F;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x84);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(apu::cpu::FLAG_H));
}

#[test]
fn test_adc_a_dp_clears_half_carry_without_nibble_overflow() {
    // $01 + $01 = $02 — no carry out of bit 3
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x84);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(apu::cpu::FLAG_H));
}

#[test]
fn test_adc_a_dp_half_carry_includes_carry_in() {
    // $0E + $00 + carry-in(1) = $0F+1 = $10 — carry out of bit 3 only
    // because of the incoming carry
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x0E;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x84);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(apu::cpu::FLAG_H));
}

// ============================================================
// SBC A,dp ($A4)
// ============================================================

#[test]
fn test_sbc_a_dp_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C; // C set = no borrow-in
    cpu.regs.a = 0x15;
    mem.write8(0x0020, 0x05);
    mem.write8(0x0200, 0xA4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x10);
}

#[test]
fn test_sbc_a_dp_subtracts_borrow_in() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0; // C clear = borrow-in present
    cpu.regs.a = 0x15;
    mem.write8(0x0020, 0x05);
    mem.write8(0x0200, 0xA4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_sbc_a_dp_clears_carry_on_borrow() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x00;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0xA4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
    assert_eq!(cpu.regs.a, 0xFF);
}

#[test]
fn test_sbc_a_dp_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x05;
    mem.write8(0x0020, 0x05);
    mem.write8(0x0200, 0xA4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_sbc_a_dp_sets_overflow_flag() {
    // $80 - $01 = $7F — neg-pos=pos, signed overflow
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x80;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0xA4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_V));
}

#[test]
fn test_sbc_a_dp_sets_half_borrow_clear_on_nibble_underflow() {
    // $10 - $01: low nibble $0 - $1 underflows -> half-borrow occurred -> H clear
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x10;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0xA4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(apu::cpu::FLAG_H));
}

#[test]
fn test_sbc_a_dp_sets_half_borrow_set_without_nibble_underflow() {
    // $15 - $05: low nibble $5 - $5 = 0, no underflow -> H set
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x15;
    mem.write8(0x0020, 0x05);
    mem.write8(0x0200, 0xA4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(apu::cpu::FLAG_H));
}

#[test]
fn test_sbc_a_dp_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P | FLAG_C;
    cpu.regs.a = 0x15;
    mem.write8(0x0120, 0x05);
    mem.write8(0x0200, 0xA4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x10);
}

#[test]
fn test_sbc_a_dp_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xA4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// SBC A,!abs ($A5)
// ============================================================

#[test]
fn test_sbc_a_abs_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x15;
    mem.write8(0x0500, 0x05);
    mem.write8(0x0200, 0xA5);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x10);
}

#[test]
fn test_sbc_a_abs_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xA5);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// SBC A,(X) ($A6)
// ============================================================

#[test]
fn test_sbc_a_ix_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x15;
    cpu.regs.x = 0x20;
    mem.write8(0x0020, 0x05);
    mem.write8(0x0200, 0xA6);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x10);
}

#[test]
fn test_sbc_a_ix_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xA6);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// SBC A,[dp+X] ($A7)
// ============================================================

#[test]
fn test_sbc_a_dp_x_ind_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x15;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x00);
    mem.write8(0x0023, 0x05);
    mem.write8(0x0500, 0x05);
    mem.write8(0x0200, 0xA7);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x10);
}

#[test]
fn test_sbc_a_dp_x_ind_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0200, 0xA7);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// SBC dd,ds ($A9)
// ============================================================

#[test]
fn test_sbc_dp_dp_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0030, 0x05); // src
    mem.write8(0x0040, 0x15); // dst
    mem.write8(0x0200, 0xA9);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0040), 0x10);
}

#[test]
fn test_sbc_dp_dp_does_not_modify_source() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0030, 0x05);
    mem.write8(0x0040, 0x15);
    mem.write8(0x0200, 0xA9);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0030), 0x05);
}

#[test]
fn test_sbc_dp_dp_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0030, 0x00);
    mem.write8(0x0040, 0x00);
    mem.write8(0x0200, 0xA9);
    mem.write8(0x0201, 0x30);
    mem.write8(0x0202, 0x40);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// SBC A,dp+X ($B4)
// ============================================================

#[test]
fn test_sbc_a_dp_x_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x15;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x05);
    mem.write8(0x0200, 0xB4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x10);
}

#[test]
fn test_sbc_a_dp_x_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xB4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// SBC A,!abs+X ($B5) / SBC A,!abs+Y ($B6)
// ============================================================

#[test]
fn test_sbc_a_abs_x_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x15;
    cpu.regs.x = 0x02;
    mem.write8(0x0502, 0x05);
    mem.write8(0x0200, 0xB5);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x10);
}

#[test]
fn test_sbc_a_abs_x_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xB5);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_sbc_a_abs_y_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x15;
    cpu.regs.y = 0x03;
    mem.write8(0x0503, 0x05);
    mem.write8(0x0200, 0xB6);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x10);
}

#[test]
fn test_sbc_a_abs_y_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xB6);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// SBC A,[dp]+Y ($B7)
// ============================================================

#[test]
fn test_sbc_a_dp_ind_y_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.a = 0x15;
    cpu.regs.y = 0x03;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0503, 0x05);
    mem.write8(0x0200, 0xB7);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x10);
}

#[test]
fn test_sbc_a_dp_ind_y_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x05);
    mem.write8(0x0200, 0xB7);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// SBC dp,#imm ($B8)
// ============================================================

#[test]
fn test_sbc_dp_imm_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0020, 0x15);
    mem.write8(0x0200, 0xB8);
    mem.write8(0x0201, 0x05); // immediate
    mem.write8(0x0202, 0x20); // dp offset
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x10);
}

#[test]
fn test_sbc_dp_imm_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0xB8);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// SBC (X),(Y) ($B9)
// ============================================================

#[test]
fn test_sbc_ix_iy_basic() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.x = 0x20;
    cpu.regs.y = 0x30;
    mem.write8(0x0020, 0x15); // dst (X)
    mem.write8(0x0030, 0x05); // src (Y)
    mem.write8(0x0200, 0xB9);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x10);
}

#[test]
fn test_sbc_ix_iy_does_not_modify_y_address() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    cpu.regs.x = 0x20;
    cpu.regs.y = 0x30;
    mem.write8(0x0020, 0x15);
    mem.write8(0x0030, 0x05);
    mem.write8(0x0200, 0xB9);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0030), 0x05);
}

#[test]
fn test_sbc_ix_iy_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xB9);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}
