/// Flag and miscellaneous accumulator instruction tests
/// (feature/spc700-flags-misc)
///
/// One file for the entire PR — grows as each opcode is added.
/// Currently covers:
///   - CLRC ($60) / SETC ($80)

use apu::cpu::{Spc700, FLAG_C, FLAG_H, FLAG_N, FLAG_P, FLAG_V, FLAG_Z, FLAG_I};
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
// CLRC ($60) — clear carry flag
// ============================================================

#[test]
fn test_clrc_clears_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0x60);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_clrc_already_clear_stays_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x60);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_clrc_does_not_affect_other_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_N | FLAG_Z;
    mem.write8(0x0200, 0x60);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z);
}

#[test]
fn test_clrc_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x60);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_clrc_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x60);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// SETC ($80) — set carry flag
// ============================================================

#[test]
fn test_setc_sets_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x80);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_setc_already_set_stays_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0x80);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_setc_does_not_affect_other_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    mem.write8(0x0200, 0x80);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z | FLAG_C);
}

#[test]
fn test_setc_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x80);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_setc_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x80);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_clrc_setc_round_trip() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x80); // SETC
    mem.write8(0x0201, 0x60); // CLRC
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

// ============================================================
// NOTC ($ED) — complement carry flag
// ============================================================

#[test]
fn test_notc_clears_carry_when_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0xED);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_notc_sets_carry_when_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xED);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_notc_twice_restores_carry() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0xED);
    mem.write8(0x0201, 0xED);
    cpu.step(&mut mem);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_notc_does_not_affect_other_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_N | FLAG_Z;
    mem.write8(0x0200, 0xED);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z);
}

#[test]
fn test_notc_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xED);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

#[test]
fn test_notc_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xED);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// CLRP ($20) — clear direct page flag
// ============================================================

#[test]
fn test_clrp_clears_p_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0200, 0x20);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_P));
}

#[test]
fn test_clrp_does_not_affect_other_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P | FLAG_N | FLAG_C;
    mem.write8(0x0200, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_C);
}

#[test]
fn test_clrp_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_clrp_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_clrp_makes_dp_use_page_zero() {
    // After CLRP, a dp load must read from $0000+offset
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0030, 0xAB); // page 0
    mem.write8(0x0200, 0x20); // CLRP
    mem.write8(0x0201, 0xE4); // LDA dp
    mem.write8(0x0202, 0x30);
    cpu.step(&mut mem); // CLRP
    cpu.step(&mut mem); // LDA $30
    assert_eq!(cpu.regs.a, 0xAB);
}

// ============================================================
// SETP ($40) — set direct page flag
// ============================================================

#[test]
fn test_setp_sets_p_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x40);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_P));
}

#[test]
fn test_setp_does_not_affect_other_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_C;
    mem.write8(0x0200, 0x40);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_C | FLAG_P);
}

#[test]
fn test_setp_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x40);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_setp_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x40);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_setp_makes_dp_use_page_one() {
    // After SETP, a dp load must read from $0100+offset
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0130, 0xCD); // page 1
    mem.write8(0x0200, 0x40); // SETP
    mem.write8(0x0201, 0xE4); // LDA dp
    mem.write8(0x0202, 0x30);
    cpu.step(&mut mem); // SETP
    cpu.step(&mut mem); // LDA $30
    assert_eq!(cpu.regs.a, 0xCD);
}

#[test]
fn test_clrp_setp_round_trip() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x40); // SETP
    mem.write8(0x0201, 0x20); // CLRP
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_P));
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_P));
}

// ============================================================
// CLRV ($E0) — clear overflow and half-carry flags
// ============================================================

#[test]
fn test_clrv_clears_overflow_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_V;
    mem.write8(0x0200, 0xE0);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_V));
}

#[test]
fn test_clrv_clears_half_carry_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_H;
    mem.write8(0x0200, 0xE0);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_H));
}

#[test]
fn test_clrv_clears_both_v_and_h_simultaneously() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_V | FLAG_H;
    mem.write8(0x0200, 0xE0);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_V));
    assert!(!cpu.get_flag(FLAG_H));
}

#[test]
fn test_clrv_does_not_affect_other_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_V | FLAG_H | FLAG_N | FLAG_C | FLAG_Z;
    mem.write8(0x0200, 0xE0);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_C | FLAG_Z);
}

#[test]
fn test_clrv_already_clear_stays_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xE0);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_V));
    assert!(!cpu.get_flag(FLAG_H));
}

#[test]
fn test_clrv_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xE0);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_clrv_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xE0);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// EI ($A0)— enable interrupts
// ============================================================

#[test]
fn test_ei_sets_interrupt_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xA0);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_I));
}

#[test]
fn test_ei_does_not_affect_other_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_C;
    mem.write8(0x0200, 0xA0);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_C | FLAG_I);
}

#[test]
fn test_ei_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xA0);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

#[test]
fn test_ei_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xA0);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// DI ($C0) — disable interrupts
// ============================================================

#[test]
fn test_di_clears_interrupt_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_I;
    mem.write8(0x0200, 0xC0);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_I));
}

#[test]
fn test_di_does_not_affect_other_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_I | FLAG_N | FLAG_C;
    mem.write8(0x0200, 0xC0);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_C);
}

#[test]
fn test_di_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xC0);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

#[test]
fn test_di_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xC0);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_ei_di_round_trip() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xA0); // EI
    mem.write8(0x0201, 0xC0); // DI
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_I));
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_I));
}

// ============================================================
// DAA ($DF) — decimal adjust after BCD addition
// ============================================================

#[test]
fn test_daa_no_adjustment_needed() {
    // $05 + $03 = $08 — valid BCD, no adjustment needed
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x08;
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xDF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x08);
}

#[test]
fn test_daa_adjusts_low_nibble() {
    // $08 + $05 = $0D — low nibble > 9, add 6 → $13
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0D;
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xDF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x13);
}

#[test]
fn test_daa_adjusts_high_nibble() {
    // result > $99 — add $60, set carry
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xA0;
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xDF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x00);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_daa_adjusts_with_carry_set() {
    // carry set from prior ADC — always add $60
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0xDF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x70);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_daa_adjusts_with_half_carry_set() {
    // H set from prior ADC — always add $06
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    cpu.regs.psw = FLAG_H;
    mem.write8(0x0200, 0xDF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x16);
}

#[test]
fn test_daa_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x9A;
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xDF);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_daa_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x80;
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xDF);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_daa_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xDF);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

#[test]
fn test_daa_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xDF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// DAS ($BE) — decimal adjust after BCD subtraction
// ============================================================

#[test]
fn test_das_no_adjustment_needed() {
    // Valid BCD result with C and H set — no adjustment needed
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x08;
    cpu.regs.psw = FLAG_C | FLAG_H;
    mem.write8(0x0200, 0xBE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x08);
}

#[test]
fn test_das_adjusts_low_nibble() {
    // H clear — subtract 6 from low nibble
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0F;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0xBE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x09);
}

#[test]
fn test_das_adjusts_high_nibble() {
    // C clear — subtract $60, clear carry
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x70;
    cpu.regs.psw = FLAG_H;
    mem.write8(0x0200, 0xBE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x10);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_das_adjusts_with_carry_clear() {
    // C clear always subtracts $60
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x80;
    cpu.regs.psw = FLAG_H;
    mem.write8(0x0200, 0xBE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x20);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_das_adjusts_with_half_carry_clear() {
    // H clear always subtracts $06
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x1F;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0xBE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x19);
}

#[test]
fn test_das_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x66;
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xBE);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_das_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    cpu.regs.psw = FLAG_C | FLAG_H;
    mem.write8(0x0200, 0xBE);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_das_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xBE);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

#[test]
fn test_das_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xBE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}
