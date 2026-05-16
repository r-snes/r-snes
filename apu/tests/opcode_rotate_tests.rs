/// Rotate instruction tests
///
/// Currently covers:
///   - ROL A ($3C)

use apu::cpu::{Spc700, FLAG_C, FLAG_N, FLAG_Z, FLAG_P};
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
// ROL A ($3C) — rotate left through carry, accumulator
// ============================================================

#[test]
fn test_rol_a_shifts_left_by_1() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    mem.write8(0x0200, 0x3C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x02);
}

#[test]
fn test_rol_a_old_carry_goes_to_bit0() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x00;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0x3C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x01, "old carry must rotate into bit 0");
    assert!(!cpu.get_flag(FLAG_C), "new carry must be 0 (bit 7 was 0)");
}

#[test]
fn test_rol_a_bit7_goes_to_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x80;
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x3C);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C), "bit 7 must rotate into carry");
    assert_eq!(cpu.regs.a, 0x00, "bit 0 must be 0 (old carry was 0)");
}

#[test]
fn test_rol_a_rotates_carry_through() {
    // A=$80, C=1 → new A=$01, new C=1
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x80;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0x3C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x01);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_rol_a_sets_zero_flag() {
    // A=$80, C=0 → result=$00 → Z set
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x80;
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x3C);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_rol_a_sets_negative_flag() {
    // A=$40, C=0 → result=$80 → N set
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x40;
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x3C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x80);
    assert!(cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_rol_a_full_rotation_8_steps() {
    // Rotating $01 left 8 times with C=0 must return to $01
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    cpu.regs.psw = 0x00;
    for i in 0..8 {
        mem.write8(0x0200 + i, 0x3C);
    }
    for _ in 0..8 {
        cpu.step(&mut mem);
    }
    assert_eq!(cpu.regs.a, 0x01);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_rol_a_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x3C);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_rol_a_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x3C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// ROL dp ($2B) — rotate left through carry, direct page
// ============================================================

#[test]
fn test_rol_dp_shifts_left_with_carry_in() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x2B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x01, "old carry must rotate into bit 0");
}

#[test]
fn test_rol_dp_bit7_goes_to_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0020, 0x80);
    mem.write8(0x0200, 0x2B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
    assert_eq!(mem.read8(0x0020), 0x00);
}

#[test]
fn test_rol_dp_clears_carry_when_bit7_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x2B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x03);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_rol_dp_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0020, 0x80);
    mem.write8(0x0200, 0x2B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_rol_dp_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0020, 0x40);
    mem.write8(0x0200, 0x2B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x80);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_rol_dp_uses_page_one_when_p_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0120, 0x01);
    mem.write8(0x0200, 0x2B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0120), 0x02);
}

#[test]
fn test_rol_dp_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x2B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// ROL !abs ($2C) — rotate left through carry, absolute address
// ============================================================

#[test]
fn test_rol_abs_shifts_left_with_carry_in() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0500, 0x00);
    mem.write8(0x0200, 0x2C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0x01);
}

#[test]
fn test_rol_abs_bit7_goes_to_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0500, 0x80);
    mem.write8(0x0200, 0x2C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
    assert_eq!(mem.read8(0x0500), 0x00);
}

#[test]
fn test_rol_abs_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0500, 0x40);
    mem.write8(0x0200, 0x2C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0x80);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_rol_abs_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x2C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

#[test]
fn test_rol_abs_advances_pc_by_3() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x2C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0203);
}
