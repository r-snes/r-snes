/// MOV addressing mode tests
/// Currently covers:
///   - MOV A,(X)  ($E6)
///   - MOV (X),A  ($C6)

use apu::cpu::{Spc700, FLAG_N, FLAG_P, FLAG_Z};
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
// MOV A,(X) ($E6) — load A from dp address in X
// ============================================================

#[test]
fn test_mov_a_ix_loads_from_x_address() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    mem.write8(0x0020, 0xAB);
    mem.write8(0x0200, 0xE6);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xAB);
}

#[test]
fn test_mov_a_ix_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    cpu.regs.x = 0x20;
    mem.write8(0x0120, 0xCD);
    mem.write8(0x0200, 0xE6);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xCD);
}

#[test]
fn test_mov_a_ix_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x10;
    mem.write8(0x0010, 0x00);
    mem.write8(0x0200, 0xE6);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_mov_a_ix_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x10;
    mem.write8(0x0010, 0x80);
    mem.write8(0x0200, 0xE6);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_mov_a_ix_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xE6);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

#[test]
fn test_mov_a_ix_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xE6);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// MOV (X),A ($C6) — store A to dp address in X
// ============================================================

#[test]
fn test_mov_ix_a_stores_a() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    cpu.regs.a = 0xAB;
    mem.write8(0x0200, 0xC6);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0xAB);
}

#[test]
fn test_mov_ix_a_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    cpu.regs.x = 0x20;
    cpu.regs.a = 0xCD;
    mem.write8(0x0200, 0xC6);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0120), 0xCD);
}

#[test]
fn test_mov_ix_a_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    cpu.regs.a = 0x00;
    mem.write8(0x0200, 0xC6);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z);
}

#[test]
fn test_mov_ix_a_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xC6);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_mov_ix_a_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xC6);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// MOV A,(X)+ ($BF) — load A from (X), post-increment X
// ============================================================

#[test]
fn test_mov_a_ixp_loads_from_x_address() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    mem.write8(0x0020, 0xAB);
    mem.write8(0x0200, 0xBF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xAB);
}

#[test]
fn test_mov_a_ixp_increments_x() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    mem.write8(0x0200, 0xBF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x21);
}

#[test]
fn test_mov_a_ixp_x_wraps_from_ff_to_00() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0xFF;
    mem.write8(0x0200, 0xBF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x00);
}

#[test]
fn test_mov_a_ixp_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    cpu.regs.x = 0x20;
    mem.write8(0x0120, 0xCD);
    mem.write8(0x0200, 0xBF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xCD);
}

#[test]
fn test_mov_a_ixp_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x10;
    mem.write8(0x0010, 0x00);
    mem.write8(0x0200, 0xBF);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_mov_a_ixp_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x10;
    mem.write8(0x0010, 0x80);
    mem.write8(0x0200, 0xBF);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_mov_a_ixp_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xBF);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_mov_a_ixp_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xBF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

// ============================================================
// MOV (X)+,A ($AF) — store A to (X), post-increment X
// ============================================================

#[test]
fn test_mov_ixp_a_stores_a() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    cpu.regs.a = 0xAB;
    mem.write8(0x0200, 0xAF);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0xAB);
}

#[test]
fn test_mov_ixp_a_increments_x() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x20;
    mem.write8(0x0200, 0xAF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x21);
}

#[test]
fn test_mov_ixp_a_x_wraps_from_ff_to_00() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0xFF;
    mem.write8(0x0200, 0xAF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x00);
}

#[test]
fn test_mov_ixp_a_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    cpu.regs.x = 0x20;
    cpu.regs.a = 0xCD;
    mem.write8(0x0200, 0xAF);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0120), 0xCD);
}

#[test]
fn test_mov_ixp_a_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    mem.write8(0x0200, 0xAF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z);
}

#[test]
fn test_mov_ixp_a_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xAF);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_mov_ixp_a_advances_pc_by_1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xAF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_mov_ixp_a_and_mov_a_ixp_round_trip() {
    // Write 3 bytes with (X)+,A then read them back with A,(X)+
    let (mut cpu, mut mem) = make();
    // Write $11, $22, $33 to $0020, $0021, $0022
    mem.write8(0x0200, 0xAF); // MOV (X)+,A
    mem.write8(0x0201, 0xAF);
    mem.write8(0x0202, 0xAF);
    mem.write8(0x0203, 0xBF); // MOV A,(X)+
    mem.write8(0x0204, 0xBF);
    mem.write8(0x0205, 0xBF);

    cpu.regs.x = 0x20;
    cpu.regs.a = 0x11; cpu.step(&mut mem); // write $11 to $0020
    cpu.regs.a = 0x22; cpu.step(&mut mem); // write $22 to $0021
    cpu.regs.a = 0x33; cpu.step(&mut mem); // write $33 to $0022

    // X is now $23 — reset to $20 to read back
    cpu.regs.x = 0x20;
    cpu.step(&mut mem); assert_eq!(cpu.regs.a, 0x11);
    cpu.step(&mut mem); assert_eq!(cpu.regs.a, 0x22);
    cpu.step(&mut mem); assert_eq!(cpu.regs.a, 0x33);
}

// ============================================================
// MOV A,dp+X ($F4) — load A from direct page indexed by X
// ============================================================

#[test]
fn test_mov_a_dp_x_loads_indexed() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0xAB); // $0020 + X=$02
    mem.write8(0x0200, 0xF4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xAB);
}

#[test]
fn test_mov_a_dp_x_wraps_within_page() {
    // offset $FF + X=$02 → $01 within the page
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x02;
    mem.write8(0x0001, 0xCD);
    mem.write8(0x0200, 0xF4);
    mem.write8(0x0201, 0xFF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xCD);
}

#[test]
fn test_mov_a_dp_x_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    cpu.regs.x = 0x02;
    mem.write8(0x0122, 0xEF);
    mem.write8(0x0200, 0xF4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xEF);
}

#[test]
fn test_mov_a_dp_x_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0021, 0x00);
    mem.write8(0x0200, 0xF4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_mov_a_dp_x_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0021, 0x80);
    mem.write8(0x0200, 0xF4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_mov_a_dp_x_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xF4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_mov_a_dp_x_advances_pc_by_2() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xF4);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}

// ============================================================
// MOV A,!abs+X ($F5)
// ============================================================

#[test]
fn test_mov_a_abs_x_loads_indexed() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x02;
    mem.write8(0x0502, 0xAB); // $0500 + X=$02
    mem.write8(0x0200, 0xF5);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xAB);
}

#[test]
fn test_mov_a_abs_x_wraps_at_ffff() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0000, 0xCD); // $FFFF + 1 wraps to $0000
    mem.write8(0x0200, 0xF5);
    mem.write8(0x0201, 0xFF);
    mem.write8(0x0202, 0xFF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xCD);
}

#[test]
fn test_mov_a_abs_x_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0501, 0x00);
    mem.write8(0x0200, 0xF5);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_mov_a_abs_x_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x01;
    mem.write8(0x0501, 0x80);
    mem.write8(0x0200, 0xF5);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_mov_a_abs_x_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xF5);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_mov_a_abs_x_advances_pc_by_3() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xF5);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0203);
}

// ============================================================
// MOV A,!abs+Y ($F6)
// ============================================================

#[test]
fn test_mov_a_abs_y_loads_indexed() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x03;
    mem.write8(0x0503, 0xEF); // $0500 + Y=$03
    mem.write8(0x0200, 0xF6);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xEF);
}

#[test]
fn test_mov_a_abs_y_sets_zero_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x01;
    mem.write8(0x0501, 0x00);
    mem.write8(0x0200, 0xF6);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_mov_a_abs_y_sets_negative_flag() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x01;
    mem.write8(0x0501, 0x80);
    mem.write8(0x0200, 0xF6);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_mov_a_abs_y_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xF6);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_mov_a_abs_y_advances_pc_by_3() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xF6);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0203);
}
