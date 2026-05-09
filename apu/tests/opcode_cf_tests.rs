/// Control flow instruction tests
/// Currently covers:
///   - BRA ($2F) — branch always
///   - BEQ ($F0) — branch if Z set

use apu::cpu::{Spc700, FLAG_C, FLAG_N, FLAG_Z};
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
// BRA ($2F) — branch always
// ============================================================

#[test]
fn test_bra_positive_offset() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x2F);
    mem.write8(0x0201, 0x05); // +5 → $0207
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0207);
}

#[test]
fn test_bra_negative_offset() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x2F);
    mem.write8(0x0201, 0xFD); // -3 → $01FF
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x01FF);
}

#[test]
fn test_bra_zero_offset() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x2F);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}

#[test]
fn test_bra_max_positive_offset() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x2F);
    mem.write8(0x0201, 0x7F); // +127 → $0281
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0281);
}

#[test]
fn test_bra_max_negative_offset() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x2F);
    mem.write8(0x0201, 0x80); // -128 → $0182
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0182);
}

#[test]
fn test_bra_pc_wraps_at_ffff() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.pc = 0xFFFF;
    mem.write8(0xFFFF, 0x2F);
    mem.write8(0x0000, 0x01); // offset +1 → $0002
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0002);
}

#[test]
fn test_bra_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x2F);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_bra_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0xFF;
    mem.write8(0x0200, 0x2F);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, 0xFF);
}

#[test]
fn test_bra_always_taken_regardless_of_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_Z | FLAG_C;
    mem.write8(0x0200, 0x2F);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}

// ============================================================
// BEQ ($F0) — branch if Zero set
// ============================================================

#[test]
fn test_beq_taken_when_z_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_Z;
    mem.write8(0x0200, 0xF0);
    mem.write8(0x0201, 0x04); // +4 → $0206
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}

#[test]
fn test_beq_taken_negative_offset() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_Z;
    mem.write8(0x0200, 0xF0);
    mem.write8(0x0201, 0xFD); // -3 → $01FF
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x01FF);
}

#[test]
fn test_beq_taken_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_Z;
    mem.write8(0x0200, 0xF0);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_beq_not_taken_when_z_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xF0);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}

#[test]
fn test_beq_not_taken_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xF0);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_beq_not_taken_when_only_n_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0200, 0xF0);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}

#[test]
fn test_beq_taken_with_other_flags_also_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_Z | FLAG_N | FLAG_C;
    mem.write8(0x0200, 0xF0);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}

#[test]
fn test_beq_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_Z | FLAG_C;
    mem.write8(0x0200, 0xF0);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_Z | FLAG_C);
}

#[test]
fn test_beq_used_as_loop_exit() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xE8); // LDA #0
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0xF0); // BEQ +1
    mem.write8(0x0203, 0x01);
    mem.write8(0x0204, 0xFF); // skipped
    mem.write8(0x0205, 0x00); // NOP — branch target

    cpu.step(&mut mem); // LDA #0 → sets Z
    cpu.step(&mut mem); // BEQ → taken
    assert_eq!(cpu.regs.pc, 0x0205);
}
