/// 16-bit opcodes tests
/// Currently covers:
/// - MOVW YA,dp ($BA)
/// - MOVW dp,YA ($DA)
/// - ADDW YA,dp ($7A)
/// - SUBW YA,dp ($9A)
/// - CMPW YA,dp ($5A)
/// - DECW dp ($1A)
/// - INCW dp ($3A)

use apu::cpu::{Spc700, FLAG_N, FLAG_P, FLAG_Z, FLAG_C};
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
// MOVW YA,dp ($BA) — load 16-bit word from dp into YA
// ============================================================

#[test]
fn test_movw_ya_dp_loads_low_and_high_bytes() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xAB); // low byte → A
    mem.write8(0x0021, 0xCD); // high byte → Y
    mem.write8(0x0200, 0xBA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xAB);
    assert_eq!(cpu.regs.y, 0xCD);
}

#[test]
fn test_movw_ya_dp_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0120, 0x11);
    mem.write8(0x0121, 0x22);
    mem.write8(0x0200, 0xBA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x11);
    assert_eq!(cpu.regs.y, 0x22);
}

#[test]
fn test_movw_ya_dp_sets_zero_flag_when_both_bytes_zero() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0xBA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_movw_ya_dp_low_byte_nonzero_clears_zero_flag() {
    // Even if high byte (Y) is zero, a non-zero low byte means the full
    // 16-bit word is non-zero, so Z must be clear.
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x01);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0xBA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_movw_ya_dp_sets_negative_flag_from_bit15() {
    // High byte (Y) bit 7 = bit 15 of the 16-bit word
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x80);
    mem.write8(0x0200, 0xBA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_movw_ya_dp_clears_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0021, 0x7F); // bit 15 = 0
    mem.write8(0x0200, 0xBA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_movw_ya_dp_high_byte_at_dp_plus_1() {
    // Confirm the high byte is read from addr+1, not addr again.
    // Offset $7E/$7F sit in plain RAM, away from both the opcode's own
    // location ($0200) and the reserved I/O window ($00F0-$00FF).
    let (mut cpu, mut mem) = make();
    mem.write8(0x007E, 0x11); // low byte at offset $7E
    mem.write8(0x007F, 0x22); // high byte at offset $7E+1 = $7F
    mem.write8(0x0200, 0xBA);
    mem.write8(0x0201, 0x7E);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x11);
    assert_eq!(cpu.regs.y, 0x22);
}

#[test]
fn test_movw_ya_dp_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = apu::cpu::FLAG_C;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0xBA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(apu::cpu::FLAG_C));
}

#[test]
fn test_movw_ya_dp_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xBA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_movw_ya_dp_advances_pc_by_2() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xBA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}

// ============================================================
// MOVW dp,YA ($DA) — store YA as a 16-bit word to dp
// ============================================================

#[test]
fn test_movw_dp_ya_stores_low_and_high_bytes() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xAB; // low byte
    cpu.regs.y = 0xCD; // high byte
    mem.write8(0x0200, 0xDA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0xAB);
    assert_eq!(mem.read8(0x0021), 0xCD);
}

#[test]
fn test_movw_dp_ya_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    cpu.regs.a = 0x11;
    cpu.regs.y = 0x22;
    mem.write8(0x0200, 0xDA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0120), 0x11);
    assert_eq!(mem.read8(0x0121), 0x22);
}

#[test]
fn test_movw_dp_ya_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    cpu.regs.a = 0x00;
    cpu.regs.y = 0x00;
    mem.write8(0x0200, 0xDA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z);
}

#[test]
fn test_movw_dp_ya_does_not_modify_a_or_y() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x11;
    cpu.regs.y = 0x22;
    mem.write8(0x0200, 0xDA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x11);
    assert_eq!(cpu.regs.y, 0x22);
}

#[test]
fn test_movw_dp_ya_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xDA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_movw_dp_ya_advances_pc_by_2() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xDA);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}

// ============================================================
// ADDW YA,dp ($7A) — 16-bit add YA + word(dp) → YA
// ============================================================

#[test]
fn test_addw_basic_addition() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x00; cpu.regs.a = 0x10; // YA = $0010
    mem.write8(0x0020, 0x05); // lo
    mem.write8(0x0021, 0x00); // hi → word = $0005
    mem.write8(0x0200, 0x7A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x15);
    assert_eq!(cpu.regs.y, 0x00);
}

#[test]
fn test_addw_sets_carry_on_overflow() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0xFF; cpu.regs.a = 0xFF; // YA = $FFFF
    mem.write8(0x0020, 0x01);
    mem.write8(0x0021, 0x00); // word = $0001
    mem.write8(0x0200, 0x7A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x00);
    assert_eq!(cpu.regs.y, 0x00);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_addw_clears_carry_when_no_overflow() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = apu::cpu::FLAG_C;
    cpu.regs.y = 0x00; cpu.regs.a = 0x01;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x7A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_addw_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0xFF; cpu.regs.a = 0xFF;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x7A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_addw_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x7F; cpu.regs.a = 0xFF; // YA = $7FFF
    mem.write8(0x0020, 0x01);
    mem.write8(0x0021, 0x00); // + 1 → $8000
    mem.write8(0x0200, 0x7A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_addw_sets_overflow_flag() {
    // $7FFF + $0001 = $8000 — pos+pos=neg, signed overflow
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x7F; cpu.regs.a = 0xFF;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x7A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(apu::cpu::FLAG_V));
}

#[test]
fn test_addw_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x7A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// SUBW YA,dp ($9A) — 16-bit subtract YA - word(dp) → YA
// ============================================================

#[test]
fn test_subw_basic_subtraction() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x00; cpu.regs.a = 0x10; // YA = $0010
    mem.write8(0x0020, 0x05);
    mem.write8(0x0021, 0x00); // word = $0005
    mem.write8(0x0200, 0x9A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0B);
    assert_eq!(cpu.regs.y, 0x00);
}

#[test]
fn test_subw_sets_carry_when_no_borrow() {
    // SPC700 SBC/SUBW convention: C set means no borrow occurred
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x00; cpu.regs.a = 0x10;
    mem.write8(0x0020, 0x05);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x9A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C), "C set when result >= 0 (no borrow)");
}

#[test]
fn test_subw_clears_carry_on_borrow() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x00; cpu.regs.a = 0x00; // YA = $0000
    mem.write8(0x0020, 0x01);
    mem.write8(0x0021, 0x00); // word = $0001
    mem.write8(0x0200, 0x9A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C), "C clear when borrow occurred");
    assert_eq!(cpu.regs.a, 0xFF);
    assert_eq!(cpu.regs.y, 0xFF);
}

#[test]
fn test_subw_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x00; cpu.regs.a = 0x05;
    mem.write8(0x0020, 0x05);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x9A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_subw_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x00; cpu.regs.a = 0x00;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x9A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_subw_sets_overflow_flag() {
    // $8000 - $0001 = $7FFF — neg-pos=pos, signed overflow
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x80; cpu.regs.a = 0x00;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x9A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(apu::cpu::FLAG_V));
}

#[test]
fn test_subw_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x9A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// CMPW YA,dp ($5A) — 16-bit compare, flags only
// ============================================================

#[test]
fn test_cmpw_does_not_modify_ya() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x12; cpu.regs.a = 0x34;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x5A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x12, "Y must be unchanged");
    assert_eq!(cpu.regs.a, 0x34, "A must be unchanged");
}

#[test]
fn test_cmpw_sets_carry_when_ya_greater_or_equal() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x00; cpu.regs.a = 0x10;
    mem.write8(0x0020, 0x05);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x5A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmpw_clears_carry_when_ya_less() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x00; cpu.regs.a = 0x05;
    mem.write8(0x0020, 0x10);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x5A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_cmpw_sets_zero_flag_when_equal() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x01; cpu.regs.a = 0x00; // YA = $0100
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x01); // word = $0100
    mem.write8(0x0200, 0x5A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_cmpw_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x00; cpu.regs.a = 0x00;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x5A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N), "0 - 1 wraps negative");
}

#[test]
fn test_cmpw_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x5A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// DECW dp ($1A) — 16-bit decrement
// ============================================================

#[test]
fn test_decw_decrements_word() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00); // lo
    mem.write8(0x0021, 0x01); // hi → word = $0100
    mem.write8(0x0200, 0x1A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0xFF);
    assert_eq!(mem.read8(0x0021), 0x00);
}

#[test]
fn test_decw_wraps_from_0000_to_ffff() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x1A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0xFF);
    assert_eq!(mem.read8(0x0021), 0xFF);
}

#[test]
fn test_decw_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x01);
    mem.write8(0x0021, 0x00); // word = $0001
    mem.write8(0x0200, 0x1A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_decw_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x00); // word = $0000 → decrements to $FFFF
    mem.write8(0x0200, 0x1A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_decw_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0120, 0x05);
    mem.write8(0x0121, 0x00);
    mem.write8(0x0200, 0x1A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0120), 0x04);
}

#[test]
fn test_decw_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x01);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x1A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// INCW dp ($3A) — 16-bit increment
// ============================================================

#[test]
fn test_incw_increments_word() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0021, 0x00); // word = $00FF
    mem.write8(0x0200, 0x3A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x00);
    assert_eq!(mem.read8(0x0021), 0x01);
}

#[test]
fn test_incw_wraps_from_ffff_to_0000() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0021, 0xFF);
    mem.write8(0x0200, 0x3A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x00);
    assert_eq!(mem.read8(0x0021), 0x00);
}

#[test]
fn test_incw_sets_zero_flag_on_wrap() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0021, 0xFF);
    mem.write8(0x0200, 0x3A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_incw_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0021, 0x7F); // word = $7FFF → increments to $8000
    mem.write8(0x0200, 0x3A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_incw_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0120, 0x05);
    mem.write8(0x0121, 0x00);
    mem.write8(0x0200, 0x3A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0120), 0x06);
}

#[test]
fn test_incw_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x3A);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

#[test]
fn test_incw_decw_round_trip() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0021, 0x01); // word = $0100
    mem.write8(0x0200, 0x3A); // INCW → $0101
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x1A); // DECW → $0100
    mem.write8(0x0203, 0x20);
    cpu.step(&mut mem);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x00);
    assert_eq!(mem.read8(0x0021), 0x01);
}

// ============================================================
// INC dp+X ($BB) — increment direct page byte indexed by X
// ============================================================

#[test]
fn test_inc_dp_x_increments_indexed() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x10);
    mem.write8(0x0200, 0xBB);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0022), 0x11);
}

#[test]
fn test_inc_dp_x_wraps_within_page() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x02;
    mem.write8(0x0001, 0x10);
    mem.write8(0x0200, 0xBB);
    mem.write8(0x0201, 0xFF);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0001), 0x11);
}

#[test]
fn test_inc_dp_x_wraps_from_ff_to_00() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0021, 0xFF);
    mem.write8(0x0200, 0xBB);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0021), 0x00);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_inc_dp_x_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0021, 0x7F);
    mem.write8(0x0200, 0xBB);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0021), 0x80);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_inc_dp_x_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    cpu.regs.x = 0x02;
    mem.write8(0x0122, 0x05);
    mem.write8(0x0200, 0xBB);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0122), 0x06);
}

#[test]
fn test_inc_dp_x_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = apu::cpu::FLAG_C;
    cpu.regs.x = 0x01;
    mem.write8(0x0021, 0xFF);
    mem.write8(0x0200, 0xBB);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(apu::cpu::FLAG_C));
}

#[test]
fn test_inc_dp_x_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xBB);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// DEC dp+X ($9B) — decrement direct page byte indexed by X
// ============================================================

#[test]
fn test_dec_dp_x_decrements_indexed() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x10);
    mem.write8(0x0200, 0x9B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0022), 0x0F);
}

#[test]
fn test_dec_dp_x_wraps_within_page() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x02;
    mem.write8(0x0001, 0x10);
    mem.write8(0x0200, 0x9B);
    mem.write8(0x0201, 0xFF);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0001), 0x0F);
}

#[test]
fn test_dec_dp_x_wraps_from_00_to_ff() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x9B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0021), 0xFF);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_dec_dp_x_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0021, 0x01);
    mem.write8(0x0200, 0x9B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0021), 0x00);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_dec_dp_x_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    cpu.regs.x = 0x02;
    mem.write8(0x0122, 0x05);
    mem.write8(0x0200, 0x9B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0122), 0x04);
}

#[test]
fn test_dec_dp_x_does_not_affect_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = apu::cpu::FLAG_C;
    cpu.regs.x = 0x01;
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0x9B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(apu::cpu::FLAG_C));
}

#[test]
fn test_dec_dp_x_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x9B);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}
