/// Bit-level instruction tests (feature/spc700-bit-ops)
///
/// One file for the entire PR — grows by opcode group as each is added.
/// Currently covers:
///   - SET1/CLR1 d.bit ($02,$12,$22,$32,$42,$52,$62,$72,$82,$92,$A2,$B2,$C2,$D2,$E2,$F2)

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
// SET1 d.0 ($02) / CLR1 d.0 ($12)
// ============================================================

#[test]
fn test_set1_bit0() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x02);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b0000_0001);
}

#[test]
fn test_clr1_bit0() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0x12);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b1111_1110);
}

// ============================================================
// SET1 d.1 ($22) / CLR1 d.1 ($32)
// ============================================================

#[test]
fn test_set1_bit1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x22);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b0000_0010);
}

#[test]
fn test_clr1_bit1() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0x32);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b1111_1101);
}

// ============================================================
// SET1 d.2 ($42) / CLR1 d.2 ($52)
// ============================================================

#[test]
fn test_set1_bit2() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x42);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b0000_0100);
}

#[test]
fn test_clr1_bit2() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0x52);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b1111_1011);
}

// ============================================================
// SET1 d.3 ($62) / CLR1 d.3 ($72)
// ============================================================

#[test]
fn test_set1_bit3() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x62);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b0000_1000);
}

#[test]
fn test_clr1_bit3() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0x72);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b1111_0111);
}

// ============================================================
// SET1 d.4 ($82) / CLR1 d.4 ($92)
// ============================================================

#[test]
fn test_set1_bit4() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x82);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b0001_0000);
}

#[test]
fn test_clr1_bit4() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0x92);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b1110_1111);
}

// ============================================================
// SET1 d.5 ($A2) / CLR1 d.5 ($B2)
// ============================================================

#[test]
fn test_set1_bit5() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0xA2);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b0010_0000);
}

#[test]
fn test_clr1_bit5() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0xB2);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b1101_1111);
}

// ============================================================
// SET1 d.6 ($C2) / CLR1 d.6 ($D2)
// ============================================================

#[test]
fn test_set1_bit6() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0xC2);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b0100_0000);
}

#[test]
fn test_clr1_bit6() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0xD2);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b1011_1111);
}

// ============================================================
// SET1 d.7 ($E2) / CLR1 d.7 ($F2)
// ============================================================

#[test]
fn test_set1_bit7() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0xE2);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b1000_0000);
}

#[test]
fn test_clr1_bit7() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0xF2);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b0111_1111);
}

// ============================================================
// SET1/CLR1 general behavior (representative — bit 0 used,
// behavior is identical across all 8 bit positions)
// ============================================================

#[test]
fn test_set1_preserves_other_bits() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b1010_1010);
    mem.write8(0x0200, 0x02); // SET1 d.0
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b1010_1011, "only bit 0 should change");
}

#[test]
fn test_clr1_preserves_other_bits() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b1010_1011);
    mem.write8(0x0200, 0x12); // CLR1 d.0
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b1010_1010, "only bit 0 should change");
}

#[test]
fn test_set1_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0120, 0x00);
    mem.write8(0x0200, 0x02);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0120), 0b0000_0001);
}

#[test]
fn test_set1_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x02);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z);
}

#[test]
fn test_clr1_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    mem.write8(0x0020, 0xFF);
    mem.write8(0x0200, 0x12);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z);
}

#[test]
fn test_set1_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x02);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_clr1_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x12);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_set1_advances_pc_by_2() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x02);
    mem.write8(0x0201, 0x20);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}
