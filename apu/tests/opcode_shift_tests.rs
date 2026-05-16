/// Shift instruction tests
///
/// Currently covers:
///   - ASL A ($1C)

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
// ASL A ($1C) — arithmetic shift left, accumulator
// ============================================================

#[test]
fn test_asl_a_shifts_left_by_1() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    mem.write8(0x0200, 0x1C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x02);
}

#[test]
fn test_asl_a_bit7_goes_to_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x80;
    mem.write8(0x0200, 0x1C);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C), "bit 7 must shift into carry");
    assert_eq!(cpu.regs.a, 0x00);
}

#[test]
fn test_asl_a_clears_carry_when_bit7_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    cpu.regs.psw = FLAG_C; // pre-set carry
    mem.write8(0x0200, 0x1C);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C), "carry must be cleared when bit 7 was 0");
}

#[test]
fn test_asl_a_zero_into_bit0() {
    // bit 0 must always be 0 after shift
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0200, 0x1C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a & 0x01, 0, "bit 0 must be 0 after ASL");
}

#[test]
fn test_asl_a_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x80;
    mem.write8(0x0200, 0x1C);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_asl_a_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x40;
    mem.write8(0x0200, 0x1C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x80);
    assert!(cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_asl_a_clears_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    cpu.regs.psw = FLAG_Z;
    mem.write8(0x0200, 0x1C);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_asl_a_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x1C);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_asl_a_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x1C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// ASL dp ($0B) — arithmetic shift left, direct page
// ============================================================

#[test]
fn test_asl_dp_shifts_memory_left() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x0B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x02);
}

#[test]
fn test_asl_dp_bit7_goes_to_carry() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x80);
    mem.write8(0x0200, 0x0B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
    assert_eq!(mem.read8(0x0020), 0x00);
}

#[test]
fn test_asl_dp_clears_carry_when_bit7_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x0B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_asl_dp_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x80);
    mem.write8(0x0200, 0x0B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_asl_dp_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x40);
    mem.write8(0x0200, 0x0B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x80);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_asl_dp_uses_page_one_when_p_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0120, 0x01);
    mem.write8(0x0200, 0x0B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0120), 0x02);
}

#[test]
fn test_asl_dp_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x0B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_asl_dp_advances_pc_by_2() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x0B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}

// ============================================================
// ASL dp+X ($1B) — arithmetic shift left, direct page indexed by X
// ============================================================

#[test]
fn test_asl_dp_x_shifts_indexed_address() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x01); // $0020 + X=$02 = $0022
    mem.write8(0x0200, 0x1B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0022), 0x02);
}

#[test]
fn test_asl_dp_x_bit7_goes_to_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0021, 0x80);
    mem.write8(0x0200, 0x1B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
    assert_eq!(mem.read8(0x0021), 0x00);
}

#[test]
fn test_asl_dp_x_wraps_within_page() {
    // offset $FF + X=$01 wraps to $00 within the page
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0000, 0x01); // wrap target
    mem.write8(0x0200, 0x1B);
    mem.write8(0x0201, 0xFF);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0000), 0x02);
}

#[test]
fn test_asl_dp_x_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x1B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// ASL !abs ($0C) — arithmetic shift left, absolute address
// ============================================================

#[test]
fn test_asl_abs_shifts_memory_left() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0x01);
    mem.write8(0x0200, 0x0C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0x02);
}

#[test]
fn test_asl_abs_bit7_goes_to_carry() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0x80);
    mem.write8(0x0200, 0x0C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
    assert_eq!(mem.read8(0x0500), 0x00);
}

#[test]
fn test_asl_abs_clears_carry_when_bit7_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0500, 0x01);
    mem.write8(0x0200, 0x0C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_asl_abs_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0x80);
    mem.write8(0x0200, 0x0C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_asl_abs_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0x40);
    mem.write8(0x0200, 0x0C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0x80);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_asl_abs_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x0C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

#[test]
fn test_asl_abs_advances_pc_by_3() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x0C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0203);
}

// ============================================================
// LSR A ($5C) — logical shift right, accumulator
// ============================================================

#[test]
fn test_lsr_a_shifts_right_by_1() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x02;
    mem.write8(0x0200, 0x5C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x01);
}

#[test]
fn test_lsr_a_bit0_goes_to_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    mem.write8(0x0200, 0x5C);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C), "bit 0 must shift into carry");
    assert_eq!(cpu.regs.a, 0x00);
}

#[test]
fn test_lsr_a_clears_carry_when_bit0_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x02;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0x5C);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_lsr_a_zero_into_bit7() {
    // bit 7 must always be 0 after LSR
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0200, 0x5C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a & 0x80, 0, "bit 7 must be 0 after LSR");
    assert!(!cpu.get_flag(FLAG_N), "N must always be clear after LSR");
}

#[test]
fn test_lsr_a_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    mem.write8(0x0200, 0x5C);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_lsr_a_clears_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x80;
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0200, 0x5C);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_N));
    assert_eq!(cpu.regs.a, 0x40);
}

#[test]
fn test_lsr_a_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x5C);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_lsr_a_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x5C);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// LSR dp ($4B) — logical shift right, direct page
// ============================================================

#[test]
fn test_lsr_dp_shifts_memory_right() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x02);
    mem.write8(0x0200, 0x4B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x01);
}

#[test]
fn test_lsr_dp_bit0_goes_to_carry() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x4B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
    assert_eq!(mem.read8(0x0020), 0x00);
}

#[test]
fn test_lsr_dp_clears_carry_when_bit0_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0020, 0x02);
    mem.write8(0x0200, 0x4B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_lsr_dp_never_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0x4B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_lsr_dp_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x4B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_lsr_dp_uses_page_one_when_p_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0120, 0x04);
    mem.write8(0x0200, 0x4B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0120), 0x02);
}

#[test]
fn test_lsr_dp_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// LSR dp+X ($5B) — logical shift right, direct page indexed by X
// ============================================================

#[test]
fn test_lsr_dp_x_shifts_indexed_address() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x04);
    mem.write8(0x0200, 0x5B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0022), 0x02);
}

#[test]
fn test_lsr_dp_x_bit0_goes_to_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0021, 0x01);
    mem.write8(0x0200, 0x5B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
    assert_eq!(mem.read8(0x0021), 0x00);
}

#[test]
fn test_lsr_dp_x_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x5B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// LSR !abs ($4C) — logical shift right, absolute address
// ============================================================

#[test]
fn test_lsr_abs_shifts_memory_right() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0x04);
    mem.write8(0x0200, 0x4C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0x02);
}

#[test]
fn test_lsr_abs_bit0_goes_to_carry() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0x01);
    mem.write8(0x0200, 0x4C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
    assert_eq!(mem.read8(0x0500), 0x00);
}

#[test]
fn test_lsr_abs_never_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0500, 0xFF);
    mem.write8(0x0200, 0x4C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_lsr_abs_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

#[test]
fn test_lsr_abs_advances_pc_by_3() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4C);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0203);
}
