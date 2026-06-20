/// Bit-level instruction tests (feature/spc700-bit-ops)
///
/// One file for the entire PR — grows by opcode group as each is added.
/// Currently covers:
/// - SET1/CLR1 d.bit ($02,$12,$22,$32,$42,$52,$62,$72,$82,$92,$A2,$B2,$C2,$D2,$E2,$F2)
/// - BBS/BBC d.bit,rel ($03,$13,$23,$33,$43,$53,$63,$73,$83,$93,$A3,$B3,$C3,$D3,$E3,$F3)
/// - TSET1 !a ($0E)
/// - TCLR1 !a ($4E)
/// - MOV1 C,m.b ($AA)
/// - MOV1 m.b,C ($CA)
/// - OR1 C,m.b ($0A)
/// - OR1 m.b,C ($2A)
/// - AND1 C,m.b ($4A)
/// - AND1 m.b,C ($6A)
/// - EOR1 C,m.b ($8A)
/// - NOT1 m.b ($EA)

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

// ============================================================
// BBS/BBC — per-opcode bit dispatch correctness (branch-taken case)
// ============================================================

#[test]
fn test_bbs_bit0_branches_when_set() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0000_0001);
    mem.write8(0x0200, 0x03); // BBS d.0
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05); // forward +5
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbc_bit0_branches_when_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b1111_1110);
    mem.write8(0x0200, 0x13); // BBC d.0
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbs_bit1_branches_when_set() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0000_0010);
    mem.write8(0x0200, 0x23);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbc_bit1_branches_when_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b1111_1101);
    mem.write8(0x0200, 0x33);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbs_bit2_branches_when_set() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0000_0100);
    mem.write8(0x0200, 0x43);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbc_bit2_branches_when_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b1111_1011);
    mem.write8(0x0200, 0x53);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbs_bit3_branches_when_set() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0000_1000);
    mem.write8(0x0200, 0x63);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbc_bit3_branches_when_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b1111_0111);
    mem.write8(0x0200, 0x73);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbs_bit4_branches_when_set() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0001_0000);
    mem.write8(0x0200, 0x83);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbc_bit4_branches_when_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b1110_1111);
    mem.write8(0x0200, 0x93);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbs_bit5_branches_when_set() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0010_0000);
    mem.write8(0x0200, 0xA3);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbc_bit5_branches_when_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b1101_1111);
    mem.write8(0x0200, 0xB3);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbs_bit6_branches_when_set() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0100_0000);
    mem.write8(0x0200, 0xC3);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbc_bit6_branches_when_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b1011_1111);
    mem.write8(0x0200, 0xD3);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbs_bit7_branches_when_set() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b1000_0000);
    mem.write8(0x0200, 0xE3);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbc_bit7_branches_when_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0111_1111);
    mem.write8(0x0200, 0xF3);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

// ============================================================
// BBS/BBC general behavior (representative — bit 0 used,
// behavior is identical across all 8 bit positions)
// ============================================================

#[test]
fn test_bbs_does_not_branch_when_bit_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0000_0000); // bit 0 clear
    mem.write8(0x0200, 0x03); // BBS d.0
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0203, "PC should just advance past the instruction");
}

#[test]
fn test_bbc_does_not_branch_when_bit_set() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0000_0001); // bit 0 set
    mem.write8(0x0200, 0x13); // BBC d.0
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0203);
}

#[test]
fn test_bbs_backward_branch() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0000_0001);
    mem.write8(0x0200, 0x03);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0xFD); // -3 as i8: PC after fetch ($0203) - 3 = $0200
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0200);
}

#[test]
fn test_bbs_uses_dp_base() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0120, 0b0000_0001);
    mem.write8(0x0200, 0x03);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_bbs_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    mem.write8(0x0020, 0b0000_0001);
    mem.write8(0x0200, 0x03);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    // FLAG_P (bit 5) is preserved as 0 here; only N and Z should remain set
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z);
}

#[test]
fn test_bbs_does_not_modify_dp_memory() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0000_0001);
    mem.write8(0x0200, 0x03);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0b0000_0001, "BBS must not write to the tested byte");
}

#[test]
fn test_bbs_not_taken_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0000_0000);
    mem.write8(0x0200, 0x03);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_bbs_taken_costs_7_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0000_0001);
    mem.write8(0x0200, 0x03);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 7);
}

#[test]
fn test_bbc_not_taken_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0000_0001);
    mem.write8(0x0200, 0x13);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_bbc_taken_costs_7_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0b0000_0000);
    mem.write8(0x0200, 0x13);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 7);
}

// ============================================================
// TSET1 !a ($0E)
// ============================================================

#[test]
fn test_tset1_ors_a_into_memory() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0b0000_1111;
    mem.write8(0x0500, 0b1111_0000);
    mem.write8(0x0200, 0x0E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0xFF);
}

#[test]
fn test_tset1_sets_zero_flag_when_a_equals_data() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x42;
    mem.write8(0x0500, 0x42);
    mem.write8(0x0200, 0x0E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_tset1_sets_negative_flag() {
    // A - data wraps negative when data > A
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x00;
    mem.write8(0x0500, 0x01);
    mem.write8(0x0200, 0x0E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_tset1_does_not_modify_a() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x0F;
    mem.write8(0x0500, 0xF0);
    mem.write8(0x0200, 0x0E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_tset1_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x0E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// TCLR1 !a ($4E)
// ============================================================

#[test]
fn test_tclr1_clears_a_bits_in_memory() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0b0000_1111;
    mem.write8(0x0500, 0xFF);
    mem.write8(0x0200, 0x4E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0b1111_0000);
}

#[test]
fn test_tclr1_sets_zero_flag_when_a_equals_data() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x42;
    mem.write8(0x0500, 0x42);
    mem.write8(0x0200, 0x4E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_tclr1_preserves_bits_not_in_a() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0b0000_0001;
    mem.write8(0x0500, 0b1010_1011);
    mem.write8(0x0200, 0x4E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0500), 0b1010_1010);
}

#[test]
fn test_tclr1_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4E);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// MOV1 C,m.b ($AA)
// ============================================================

#[test]
fn test_mov1_c_mb_loads_set_bit() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0100, 0b0000_1000); // bit 3 set
    mem.write8(0x0200, 0xAA);
    mem.write8(0x0201, 0x00); // packed lo: addr=$0100, bit=3 -> packed=0x6100
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_mov1_c_mb_loads_clear_bit() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C; // pre-set, must be cleared
    mem.write8(0x0100, 0b0000_0000); // bit 3 clear
    mem.write8(0x0200, 0xAA);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_mov1_c_mb_does_not_modify_memory() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0100, 0b0000_1000);
    mem.write8(0x0200, 0xAA);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0100), 0b0000_1000);
}

#[test]
fn test_mov1_c_mb_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xAA);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// MOV1 m.b,C ($CA)
// ============================================================

#[test]
fn test_mov1_mb_c_sets_bit_when_carry_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0100, 0x00);
    mem.write8(0x0200, 0xCA);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61); // addr=$0100, bit=3
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0100), 0b0000_1000);
}

#[test]
fn test_mov1_mb_c_clears_bit_when_carry_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0100, 0xFF);
    mem.write8(0x0200, 0xCA);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0100), 0b1111_0111);
}

#[test]
fn test_mov1_mb_c_preserves_other_bits() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0100, 0b1010_0010);
    mem.write8(0x0200, 0xCA);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61); // sets bit 3
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0100), 0b1010_1010);
}

#[test]
fn test_mov1_mb_c_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xCA);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// OR1 C,m.b ($0A) / OR1 C,/m.b ($2A)
// ============================================================

#[test]
fn test_or1_c_mb_sets_carry_when_bit_set() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0100, 0b0000_1000);
    mem.write8(0x0200, 0x0A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_or1_c_mb_keeps_carry_clear_when_both_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0100, 0x00);
    mem.write8(0x0200, 0x0A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_or1_c_mb_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x0A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_or1_c_not_mb_sets_carry_when_bit_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0100, 0x00); // bit 3 clear -> inverted = set
    mem.write8(0x0200, 0x2A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_or1_c_not_mb_keeps_carry_clear_when_bit_set() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0100, 0b0000_1000); // bit 3 set -> inverted = clear
    mem.write8(0x0200, 0x2A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_or1_c_not_mb_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x2A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// AND1 C,m.b ($4A) / AND1 C,/m.b ($6A)
// ============================================================

#[test]
fn test_and1_c_mb_keeps_carry_when_both_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0100, 0b0000_1000);
    mem.write8(0x0200, 0x4A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_and1_c_mb_clears_carry_when_bit_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0100, 0x00);
    mem.write8(0x0200, 0x4A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_and1_c_mb_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_and1_c_not_mb_keeps_carry_when_carry_set_and_bit_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0100, 0x00); // bit clear -> inverted = set
    mem.write8(0x0200, 0x6A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_and1_c_not_mb_clears_carry_when_bit_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0100, 0b0000_1000); // bit set -> inverted = clear
    mem.write8(0x0200, 0x6A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_and1_c_not_mb_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x6A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

// ============================================================
// EOR1 C,m.b ($8A)
// ============================================================

#[test]
fn test_eor1_c_mb_toggles_carry_when_bit_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0100, 0b0000_1000);
    mem.write8(0x0200, 0x8A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C), "C XOR 1 with C=1 should clear");
}

#[test]
fn test_eor1_c_mb_keeps_carry_when_bit_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0100, 0x00);
    mem.write8(0x0200, 0x8A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C), "C XOR 0 with C=1 should stay set");
}

#[test]
fn test_eor1_c_mb_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x8A);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

// ============================================================
// NOT1 m.b ($EA)
// ============================================================

#[test]
fn test_not1_mb_toggles_set_bit_to_clear() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0100, 0b0000_1000);
    mem.write8(0x0200, 0xEA);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0100), 0x00);
}

#[test]
fn test_not1_mb_toggles_clear_bit_to_set() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0100, 0x00);
    mem.write8(0x0200, 0xEA);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0100), 0b0000_1000);
}

#[test]
fn test_not1_mb_preserves_other_bits() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0100, 0b1010_0010);
    mem.write8(0x0200, 0xEA);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61); // toggles bit 3
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0100), 0b1010_1010);
}

#[test]
fn test_not1_mb_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    mem.write8(0x0100, 0x00);
    mem.write8(0x0200, 0xEA);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z);
}

#[test]
fn test_not1_mb_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xEA);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x61);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}
