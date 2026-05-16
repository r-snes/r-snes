/// ALU and arithmetic instruction tests
///
/// Currently covers:
///   - INC A ($BC)
///  - DEC A ($9C)

use apu::cpu::{Spc700, FLAG_C, FLAG_N, FLAG_V, FLAG_Z, FLAG_P, FLAG_H};
use apu::Memory;

// ============================================================
// Helper
// ============================================================

fn make() -> (Spc700, Memory) {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    mem.write8(0xFFFE, 0x00);
    mem.write8(0xFFFF, 0x02); // reset vector → $0200
    cpu.reset(&mut mem);
    (cpu, mem)
}

// ============================================================
// INC A ($BC) — increment accumulator
// ============================================================

#[test]
fn test_inc_a_increments_by_1() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    mem.write8(0x0200, 0xBC);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x11);
}

#[test]
fn test_inc_a_wraps_from_ff_to_00() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0200, 0xBC);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x00);
}

#[test]
fn test_inc_a_sets_zero_flag_on_wrap() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0200, 0xBC);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_inc_a_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x7F;
    mem.write8(0x0200, 0xBC);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x80);
    assert!(cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_inc_a_clears_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x00;
    cpu.regs.psw = FLAG_Z;
    mem.write8(0x0200, 0xBC);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_inc_a_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0xBC);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C), "carry must be unaffected");
}

#[test]
fn test_inc_a_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xBC);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_inc_a_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xBC);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// DEC A ($9C) — decrement accumulator
// ============================================================

#[test]
fn test_dec_a_decrements_by_1() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    mem.write8(0x0200, 0x9C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_dec_a_wraps_from_00_to_ff() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x00;
    mem.write8(0x0200, 0x9C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xFF);
}

#[test]
fn test_dec_a_sets_negative_flag_on_wrap() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x00;
    mem.write8(0x0200, 0x9C);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_dec_a_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    mem.write8(0x0200, 0x9C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x00);
    assert!(cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_dec_a_clears_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x80;
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0200, 0x9C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x7F);
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_dec_a_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x00;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0x9C);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C), "carry must be unaffected");
}

#[test]
fn test_dec_a_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x9C);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_dec_a_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x9C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// INC X ($3D)
// ============================================================

#[test]
fn test_inc_x_increments_by_1() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x10;
    mem.write8(0x0200, 0x3D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x11);
}

#[test]
fn test_inc_x_wraps_from_ff_to_00() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0xFF;
    mem.write8(0x0200, 0x3D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x00);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_inc_x_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x7F;
    mem.write8(0x0200, 0x3D);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_inc_x_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0xFF;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0x3D);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_inc_x_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x3D);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

// ============================================================
// DEC X ($1D)
// ============================================================

#[test]
fn test_dec_x_decrements_by_1() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x10;
    mem.write8(0x0200, 0x1D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x0F);
}

#[test]
fn test_dec_x_wraps_from_00_to_ff() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x00;
    mem.write8(0x0200, 0x1D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0xFF);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_dec_x_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0200, 0x1D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x00);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_dec_x_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x00;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0x1D);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_dec_x_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x1D);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

// ============================================================
// INC Y ($FC)
// ============================================================

#[test]
fn test_inc_y_increments_by_1() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x10;
    mem.write8(0x0200, 0xFC);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x11);
}

#[test]
fn test_inc_y_wraps_from_ff_to_00() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0xFF;
    mem.write8(0x0200, 0xFC);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x00);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_inc_y_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x7F;
    mem.write8(0x0200, 0xFC);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_inc_y_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0xFF;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0xFC);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_inc_y_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xFC);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

// ============================================================
// DEC Y ($DC)
// ============================================================

#[test]
fn test_dec_y_decrements_by_1() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x10;
    mem.write8(0x0200, 0xDC);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x0F);
}

#[test]
fn test_dec_y_wraps_from_00_to_ff() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x00;
    mem.write8(0x0200, 0xDC);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0xFF);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_dec_y_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x01;
    mem.write8(0x0200, 0xDC);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x00);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_dec_y_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x00;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0xDC);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_dec_y_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xDC);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

// ============================================================
// INC dp ($AB)
// ============================================================

#[test]
fn test_inc_dp_increments_memory() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x10);
    mem.write8(0x0200, 0xAB);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x11);
}

#[test]
fn test_inc_dp_wraps_from_ff_to_00() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0xFF);
    mem.write8(0x0200, 0xAB);
    mem.write8(0x0201, 0x30);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0030), 0x00);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_inc_dp_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0040, 0x7F);
    mem.write8(0x0200, 0xAB);
    mem.write8(0x0201, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0040), 0x80);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_inc_dp_uses_page_one_when_p_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0120, 0x05);
    mem.write8(0x0200, 0xAB);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0120), 0x06);
}

#[test]
fn test_inc_dp_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0010, 0xFF);
    mem.write8(0x0200, 0xAB);
    mem.write8(0x0201, 0x10);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_inc_dp_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xAB);
    mem.write8(0x0201, 0x10);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// DEC dp ($8B)
// ============================================================

#[test]
fn test_dec_dp_decrements_memory() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x10);
    mem.write8(0x0200, 0x8B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x0F);
}

#[test]
fn test_dec_dp_wraps_from_00_to_ff() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0030, 0x00);
    mem.write8(0x0200, 0x8B);
    mem.write8(0x0201, 0x30);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0030), 0xFF);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_dec_dp_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0040, 0x01);
    mem.write8(0x0200, 0x8B);
    mem.write8(0x0201, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0040), 0x00);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_dec_dp_uses_page_one_when_p_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0120, 0x05);
    mem.write8(0x0200, 0x8B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0120), 0x04);
}

#[test]
fn test_dec_dp_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0010, 0x00);
    mem.write8(0x0200, 0x8B);
    mem.write8(0x0201, 0x10);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_dec_dp_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x8B);
    mem.write8(0x0201, 0x10);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// INC !abs ($AC)
// ============================================================

#[test]
fn test_inc_abs_increments_memory() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0x10);
    mem.write8(0x0200, 0xAC);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0x11);
}

#[test]
fn test_inc_abs_wraps_from_ff_to_00() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0xFF);
    mem.write8(0x0200, 0xAC);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0x00);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_inc_abs_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0x7F);
    mem.write8(0x0200, 0xAC);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0x80);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_inc_abs_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0500, 0xFF);
    mem.write8(0x0200, 0xAC);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_inc_abs_advances_pc_by_3() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xAC);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0203);
}

#[test]
fn test_inc_abs_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xAC);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// DEC !abs ($8C)
// ============================================================

#[test]
fn test_dec_abs_decrements_memory() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0x10);
    mem.write8(0x0200, 0x8C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0x0F);
}

#[test]
fn test_dec_abs_wraps_from_00_to_ff() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0x00);
    mem.write8(0x0200, 0x8C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0xFF);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_dec_abs_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0x01);
    mem.write8(0x0200, 0x8C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0x00);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_dec_abs_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0500, 0x00);
    mem.write8(0x0200, 0x8C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_dec_abs_advances_pc_by_3() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x8C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0203);
}

#[test]
fn test_dec_abs_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x8C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}
