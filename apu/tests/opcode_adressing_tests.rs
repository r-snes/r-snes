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
