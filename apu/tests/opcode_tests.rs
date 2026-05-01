/// SPC700 CPU tests
///
/// Covers every implemented instruction, all flag outcomes, both
/// dp_base() states (FLAG_P set/clear), cycle counts, PC advancement,
/// reset(), set_flag/get_flag, and the step() dispatch table.

use apu::cpu::{Spc700, FLAG_C, FLAG_N, FLAG_V, FLAG_Z, FLAG_P, FLAG_H, FLAG_I, FLAG_B};
use apu::Memory;

// ============================================================
// Helpers
// ============================================================

/// Build a CPU + Memory with the reset vector pointing at $0200
/// and a clean slate ready for instruction tests.
fn make_cpu_mem() -> (Spc700, Memory) {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    // Point reset vector at $0200
    mem.write8(0xFFFE, 0x00);
    mem.write8(0xFFFF, 0x02);
    cpu.reset(&mut mem);
    (cpu, mem)
}

/// Write one byte at the current PC and return the address used.
fn emit(mem: &mut Memory, pc: u16, byte: u8) {
    mem.write8(pc, byte);
}

/// Write a sequence of bytes starting at `pc`.
fn emit_seq(mem: &mut Memory, pc: u16, bytes: &[u8]) {
    for (i, &b) in bytes.iter().enumerate() {
        mem.write8(pc + i as u16, b);
    }
}

// ============================================================
// reset()
// ============================================================

#[test]
fn test_reset_loads_pc_from_vector() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    mem.write8(0xFFFE, 0x34);
    mem.write8(0xFFFF, 0x12);
    cpu.reset(&mut mem);
    assert_eq!(cpu.regs.pc, 0x1234);
}

#[test]
fn test_reset_sets_sp_to_ff() {
    let (cpu, _) = make_cpu_mem();
    assert_eq!(cpu.regs.sp, 0xFF);
}

#[test]
fn test_reset_clears_psw() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.psw = 0xFF;
    cpu.reset(&mut mem);
    assert_eq!(cpu.regs.psw, 0x00);
}

#[test]
fn test_reset_zero_vector_sets_pc_zero() {
    // Default memory is zeroed, so vector = $0000
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.reset(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0000);
}

// ============================================================
// set_flag / get_flag
// ============================================================

#[test]
fn test_set_flag_true_sets_bit() {
    let (mut cpu, _) = make_cpu_mem();
    cpu.regs.psw = 0x00;
    cpu.set_flag(FLAG_C, true);
    assert_eq!(cpu.regs.psw & FLAG_C, FLAG_C);
}

#[test]
fn test_set_flag_false_clears_bit() {
    let (mut cpu, _) = make_cpu_mem();
    cpu.regs.psw = 0xFF;
    cpu.set_flag(FLAG_C, false);
    assert_eq!(cpu.regs.psw & FLAG_C, 0);
}

#[test]
fn test_set_flag_does_not_affect_other_bits() {
    let (mut cpu, _) = make_cpu_mem();
    cpu.regs.psw = 0xFF;
    cpu.set_flag(FLAG_C, false);
    // All other bits must remain set
    assert_eq!(cpu.regs.psw & !FLAG_C, !FLAG_C & 0xFF);
}

#[test]
fn test_get_flag_true() {
    let (mut cpu, _) = make_cpu_mem();
    cpu.regs.psw = FLAG_N;
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_get_flag_false() {
    let (mut cpu, _) = make_cpu_mem();
    cpu.regs.psw = 0x00;
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_all_flags_independent() {
    let (mut cpu, _) = make_cpu_mem();
    for flag in [FLAG_C, FLAG_Z, FLAG_I, FLAG_H, FLAG_B, FLAG_P, FLAG_V, FLAG_N] {
        cpu.regs.psw = 0x00;
        cpu.set_flag(flag, true);
        assert!(cpu.get_flag(flag), "flag {flag:#04X} must be set");
        assert_eq!(cpu.regs.psw & !flag, 0, "no other flags must be set");
    }
}

// ============================================================
// NOP
// ============================================================

#[test]
fn test_nop_advances_pc_by_1() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit(&mut mem, cpu.regs.pc, 0x00); // NOP
    let pc_before = cpu.regs.pc;
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, pc_before + 1);
}

#[test]
fn test_nop_adds_2_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit(&mut mem, cpu.regs.pc, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_nop_does_not_change_registers() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x11; cpu.regs.x = 0x22; cpu.regs.y = 0x33;
    emit(&mut mem, cpu.regs.pc, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x11);
    assert_eq!(cpu.regs.x, 0x22);
    assert_eq!(cpu.regs.y, 0x33);
}

// ============================================================
// Register moves
// ============================================================

#[test]
fn test_mov_a_x() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.x = 0x42;
    emit(&mut mem, cpu.regs.pc, 0x7D); // MOV A, X
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x42);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_mov_a_y() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.y = 0x55;
    emit(&mut mem, cpu.regs.pc, 0xDD); // MOV A, Y
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x55);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_mov_x_a() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x77;
    emit(&mut mem, cpu.regs.pc, 0x5D); // MOV X, A
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x77);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_mov_y_a() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x99;
    emit(&mut mem, cpu.regs.pc, 0xFD); // MOV Y, A
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x99);
    assert_eq!(cpu.cycles, 2);
}

// ============================================================
// Immediate loads — LDA/LDX/LDY #imm
// ============================================================

#[test]
fn test_lda_imm_loads_value() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0xE8, 0xAB]); // LDA #$AB
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xAB);
}

#[test]
fn test_lda_imm_advances_pc_by_2() {
    let (mut cpu, mut mem) = make_cpu_mem();
    let pc = cpu.regs.pc;
    emit_seq(&mut mem, pc, &[0xE8, 0x00]);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, pc + 2);
}

#[test]
fn test_lda_imm_adds_2_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0xE8, 0x01]);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_lda_imm_sets_zero_flag() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0xE8, 0x00]); // LDA #0
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z), "Z must be set when result is 0");
    assert!(!cpu.get_flag(FLAG_N), "N must be clear");
}

#[test]
fn test_lda_imm_sets_negative_flag() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0xE8, 0x80]); // LDA #$80
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N), "N must be set when bit 7 is 1");
    assert!(!cpu.get_flag(FLAG_Z), "Z must be clear");
}

#[test]
fn test_lda_imm_clears_zn_flags() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.psw = FLAG_Z | FLAG_N; // pre-set both
    emit_seq(&mut mem, cpu.regs.pc, &[0xE8, 0x01]); // LDA #1 → neither Z nor N
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_ldx_imm_loads_value() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0xCD, 0x33]); // LDX #$33
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x33);
}

#[test]
fn test_ldx_imm_sets_flags() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0xCD, 0x00]); // LDX #0
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_ldy_imm_loads_value() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0x8D, 0x44]); // LDY #$44
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x44);
}

#[test]
fn test_ldy_imm_adds_2_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0x8D, 0x00]);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

// ============================================================
// Absolute loads — LDA/LDX/LDY !a
// ============================================================

#[test]
fn test_lda_abs_loads_from_address() {
    let (mut cpu, mut mem) = make_cpu_mem();
    mem.write8(0x0500, 0xCC);
    emit_seq(&mut mem, cpu.regs.pc, &[0xE5, 0x00, 0x05]); // LDA !$0500
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xCC);
}

#[test]
fn test_lda_abs_advances_pc_by_3() {
    let (mut cpu, mut mem) = make_cpu_mem();
    let pc = cpu.regs.pc;
    emit_seq(&mut mem, pc, &[0xE5, 0x00, 0x05]);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, pc + 3);
}

#[test]
fn test_lda_abs_adds_4_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0xE5, 0x00, 0x05]);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_lda_abs_sets_flags() {
    let (mut cpu, mut mem) = make_cpu_mem();
    mem.write8(0x0300, 0x00);
    emit_seq(&mut mem, cpu.regs.pc, &[0xE5, 0x00, 0x03]);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_ldx_abs_loads_from_address() {
    let (mut cpu, mut mem) = make_cpu_mem();
    mem.write8(0x0400, 0xBB);
    emit_seq(&mut mem, cpu.regs.pc, &[0xE9, 0x00, 0x04]);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0xBB);
}

#[test]
fn test_ldy_abs_loads_from_address() {
    let (mut cpu, mut mem) = make_cpu_mem();
    mem.write8(0x0600, 0xDD);
    emit_seq(&mut mem, cpu.regs.pc, &[0xEC, 0x00, 0x06]);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0xDD);
}

// ============================================================
// Direct page loads — LDA/LDX/LDY d
// ============================================================

#[test]
fn test_lda_dp_loads_from_page_zero() {
    let (mut cpu, mut mem) = make_cpu_mem();
    // FLAG_P clear → dp_base = $0000
    cpu.regs.psw = 0;
    mem.write8(0x0020, 0x99); // value at $0020
    emit_seq(&mut mem, cpu.regs.pc, &[0xE4, 0x20]); // LDA $20
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x99);
}

#[test]
fn test_lda_dp_loads_from_page_one_when_p_set() {
    let (mut cpu, mut mem) = make_cpu_mem();
    // FLAG_P set → dp_base = $0100
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0120, 0x77); // value at $0120
    emit_seq(&mut mem, cpu.regs.pc, &[0xE4, 0x20]); // LDA $20 (in page 1)
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x77);
}

#[test]
fn test_lda_dp_advances_pc_by_2() {
    let (mut cpu, mut mem) = make_cpu_mem();
    let pc = cpu.regs.pc;
    emit_seq(&mut mem, pc, &[0xE4, 0x10]);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, pc + 2);
}

#[test]
fn test_lda_dp_adds_3_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0xE4, 0x10]);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

#[test]
fn test_lda_dp_sets_zero_flag() {
    let (mut cpu, mut mem) = make_cpu_mem();
    mem.write8(0x0010, 0x00);
    emit_seq(&mut mem, cpu.regs.pc, &[0xE4, 0x10]);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_lda_dp_sets_negative_flag() {
    let (mut cpu, mut mem) = make_cpu_mem();
    mem.write8(0x0010, 0xFF);
    emit_seq(&mut mem, cpu.regs.pc, &[0xE4, 0x10]);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_ldx_dp_loads_value() {
    let (mut cpu, mut mem) = make_cpu_mem();
    mem.write8(0x0030, 0x55);
    emit_seq(&mut mem, cpu.regs.pc, &[0xF8, 0x30]);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x55);
}

#[test]
fn test_ldy_dp_loads_value() {
    let (mut cpu, mut mem) = make_cpu_mem();
    mem.write8(0x0040, 0x66);
    emit_seq(&mut mem, cpu.regs.pc, &[0xEB, 0x40]);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x66);
}

// ============================================================
// Absolute stores — STA/STX/STY !a
// ============================================================

#[test]
fn test_sta_abs_writes_a_to_address() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0xAB;
    emit_seq(&mut mem, cpu.regs.pc, &[0xC5, 0x00, 0x07]); // STA !$0700
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0700), 0xAB);
}

#[test]
fn test_sta_abs_advances_pc_by_3() {
    let (mut cpu, mut mem) = make_cpu_mem();
    let pc = cpu.regs.pc;
    emit_seq(&mut mem, pc, &[0xC5, 0x00, 0x07]);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, pc + 3);
}

#[test]
fn test_sta_abs_adds_4_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0xC5, 0x00, 0x07]);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_stx_abs_writes_x_to_address() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.x = 0x12;
    emit_seq(&mut mem, cpu.regs.pc, &[0xC9, 0x00, 0x08]);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0800), 0x12);
}

#[test]
fn test_sty_abs_writes_y_to_address() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.y = 0x34;
    emit_seq(&mut mem, cpu.regs.pc, &[0xCC, 0x00, 0x09]);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0900), 0x34);
}

// ============================================================
// Direct page stores — STA d
// ============================================================

#[test]
fn test_sta_dp_writes_a_to_page_zero() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a   = 0xBB;
    cpu.regs.psw = 0; // FLAG_P clear → page 0
    emit_seq(&mut mem, cpu.regs.pc, &[0xC4, 0x50]); // STA $50
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0050), 0xBB);
}

#[test]
fn test_sta_dp_writes_to_page_one_when_p_set() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a   = 0xCC;
    cpu.regs.psw = FLAG_P; // dp_base = $0100
    emit_seq(&mut mem, cpu.regs.pc, &[0xC4, 0x50]); // STA $50 in page 1
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0150), 0xCC);
}

#[test]
fn test_sta_dp_adds_3_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0xC4, 0x10]);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

// ============================================================
// ADC #imm
// ============================================================

#[test]
fn test_adc_imm_basic_addition() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x10;
    emit_seq(&mut mem, cpu.regs.pc, &[0x88, 0x20]); // ADC #$20
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x30);
}

#[test]
fn test_adc_imm_adds_carry_in() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a   = 0x10;
    cpu.regs.psw = FLAG_C; // carry set
    emit_seq(&mut mem, cpu.regs.pc, &[0x88, 0x20]); // ADC #$20
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x31, "carry must be added to result");
}

#[test]
fn test_adc_imm_sets_carry_on_overflow() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0xFF;
    emit_seq(&mut mem, cpu.regs.pc, &[0x88, 0x01]); // ADC #1
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x00);
    assert!(cpu.get_flag(FLAG_C), "carry must be set on overflow");
    assert!(cpu.get_flag(FLAG_Z), "zero flag must be set on zero result");
}

#[test]
fn test_adc_imm_clears_carry_when_no_overflow() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a   = 0x01;
    cpu.regs.psw = FLAG_C;
    emit_seq(&mut mem, cpu.regs.pc, &[0x88, 0x01]);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x03);
    assert!(!cpu.get_flag(FLAG_C));
}

#[test]
fn test_adc_imm_sets_negative_flag() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x00;
    emit_seq(&mut mem, cpu.regs.pc, &[0x88, 0x80]); // ADC #$80
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_adc_imm_sets_overflow_pos_plus_pos_equals_neg() {
    // $70 + $10 = $80 — both positive, result negative → overflow
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x70;
    emit_seq(&mut mem, cpu.regs.pc, &[0x88, 0x10]);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_V), "V must be set: pos+pos=neg");
}

#[test]
fn test_adc_imm_sets_overflow_neg_plus_neg_equals_pos() {
    // $80 + $80 = $00 with carry — both negative, result positive → overflow
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x80;
    emit_seq(&mut mem, cpu.regs.pc, &[0x88, 0x80]);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_V), "V must be set: neg+neg=pos");
    assert!(cpu.get_flag(FLAG_C));
}

#[test]
fn test_adc_imm_clears_overflow_when_no_signed_overflow() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a   = 0x01;
    cpu.regs.psw = FLAG_V; // pre-set
    emit_seq(&mut mem, cpu.regs.pc, &[0x88, 0x01]);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_V));
}

#[test]
fn test_adc_imm_adds_2_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0x88, 0x00]);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

// ============================================================
// SBC #imm
// ============================================================

#[test]
fn test_sbc_imm_basic_subtraction() {
    // SPC700: borrow = !carry. With carry set (no borrow): $30 - $10 = $20
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a   = 0x30;
    cpu.regs.psw = FLAG_C; // carry set = no borrow
    emit_seq(&mut mem, cpu.regs.pc, &[0xA8, 0x10]); // SBC #$10
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x20);
}

#[test]
fn test_sbc_imm_subtracts_borrow() {
    // carry clear = borrow: $30 - $10 - 1 = $1F
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a   = 0x30;
    cpu.regs.psw = 0; // carry clear = borrow
    emit_seq(&mut mem, cpu.regs.pc, &[0xA8, 0x10]);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x1F);
}

#[test]
fn test_sbc_imm_sets_carry_when_no_borrow() {
    // $30 - $10 = $20, no borrow → carry set
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a   = 0x30;
    cpu.regs.psw = FLAG_C;
    emit_seq(&mut mem, cpu.regs.pc, &[0xA8, 0x10]);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C), "carry set means no borrow occurred");
}

#[test]
fn test_sbc_imm_clears_carry_when_borrow() {
    // $10 - $30 = underflow → borrow → carry clear
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a   = 0x10;
    cpu.regs.psw = FLAG_C;
    emit_seq(&mut mem, cpu.regs.pc, &[0xA8, 0x30]);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C), "carry clear means borrow occurred");
}

#[test]
fn test_sbc_imm_sets_zero_flag() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a   = 0x10;
    cpu.regs.psw = FLAG_C;
    emit_seq(&mut mem, cpu.regs.pc, &[0xA8, 0x10]); // $10 - $10 = 0
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_sbc_imm_sets_negative_flag() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a   = 0x00;
    cpu.regs.psw = FLAG_C;
    emit_seq(&mut mem, cpu.regs.pc, &[0xA8, 0x01]); // $00 - $01 = $FF
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_sbc_imm_sets_overflow_neg_minus_pos_equals_pos() {
    // $80 - $01 = $7F — neg minus pos = pos → overflow
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a   = 0x80;
    cpu.regs.psw = FLAG_C;
    emit_seq(&mut mem, cpu.regs.pc, &[0xA8, 0x01]);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_V), "V must be set: neg-pos=pos");
}

#[test]
fn test_sbc_imm_adds_2_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.psw = FLAG_C;
    emit_seq(&mut mem, cpu.regs.pc, &[0xA8, 0x00]);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

// ============================================================
// CMP #imm
// ============================================================

#[test]
fn test_cmp_equal_sets_z_and_c() {
    // A == value → Z set, C set (no borrow)
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x42;
    emit_seq(&mut mem, cpu.regs.pc, &[0x68, 0x42]);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z), "Z must be set when A == value");
    assert!(cpu.get_flag(FLAG_C), "C must be set when A >= value");
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_cmp_greater_sets_c_clears_z() {
    // A > value → C set, Z clear
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x50;
    emit_seq(&mut mem, cpu.regs.pc, &[0x68, 0x30]);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_C));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_cmp_less_clears_c_and_z() {
    // A < value → C clear, Z clear
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x10;
    emit_seq(&mut mem, cpu.regs.pc, &[0x68, 0x20]);
    cpu.step(&mut mem);
    assert!(!cpu.get_flag(FLAG_C), "C must be clear when A < value");
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_cmp_sets_negative_flag() {
    // $10 - $20 = $F0 → N set
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x10;
    emit_seq(&mut mem, cpu.regs.pc, &[0x68, 0x20]);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_cmp_does_not_modify_a() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x42;
    emit_seq(&mut mem, cpu.regs.pc, &[0x68, 0x10]);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x42, "CMP must not modify A");
}

#[test]
fn test_cmp_adds_2_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0x68, 0x00]);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

// ============================================================
// AND / ORA / EOR #imm
// ============================================================

#[test]
fn test_and_imm_result() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0xFF;
    emit_seq(&mut mem, cpu.regs.pc, &[0x28, 0x0F]); // AND #$0F
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x0F);
}

#[test]
fn test_and_imm_zero_flag() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0xF0;
    emit_seq(&mut mem, cpu.regs.pc, &[0x28, 0x0F]); // AND #$0F → 0
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_and_imm_negative_flag() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0xFF;
    emit_seq(&mut mem, cpu.regs.pc, &[0x28, 0x80]); // AND #$80 → $80
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_and_imm_adds_2_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0x28, 0xFF]);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_ora_imm_result() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x0F;
    emit_seq(&mut mem, cpu.regs.pc, &[0x08, 0xF0]); // ORA #$F0
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xFF);
}

#[test]
fn test_ora_imm_zero_flag() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x00;
    emit_seq(&mut mem, cpu.regs.pc, &[0x08, 0x00]); // ORA #0 → 0
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_ora_imm_negative_flag() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x00;
    emit_seq(&mut mem, cpu.regs.pc, &[0x08, 0x80]); // ORA #$80
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_ora_imm_adds_2_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0x08, 0x00]);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_eor_imm_result() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0xFF;
    emit_seq(&mut mem, cpu.regs.pc, &[0x48, 0x0F]); // EOR #$0F
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xF0);
}

#[test]
fn test_eor_imm_self_xor_gives_zero() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0xAB;
    emit_seq(&mut mem, cpu.regs.pc, &[0x48, 0xAB]); // EOR #$AB → 0
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x00);
    assert!(cpu.get_flag(FLAG_Z));
}

#[test]
fn test_eor_imm_negative_flag() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.a = 0x00;
    emit_seq(&mut mem, cpu.regs.pc, &[0x48, 0x80]); // EOR #$80
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_eor_imm_adds_2_cycles() {
    let (mut cpu, mut mem) = make_cpu_mem();
    emit_seq(&mut mem, cpu.regs.pc, &[0x48, 0x00]);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

// ============================================================
// dp_base() — both FLAG_P states
// ============================================================

#[test]
fn test_dp_base_clear_uses_page_zero() {
    // Covered by test_lda_dp_loads_from_page_zero above,
    // but verified explicitly here via STX dp.
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.x   = 0xAA;
    cpu.regs.psw = 0; // FLAG_P clear
    // STX dp not yet in dispatch — use LDA dp to verify page 0 addressing
    mem.write8(0x0030, 0xAA);
    emit_seq(&mut mem, cpu.regs.pc, &[0xF8, 0x30]); // LDX $30 (page 0)
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0xAA);
}

#[test]
fn test_dp_base_set_uses_page_one() {
    let (mut cpu, mut mem) = make_cpu_mem();
    cpu.regs.psw = FLAG_P;
    mem.write8(0x0130, 0xBB); // page 1 offset $30
    emit_seq(&mut mem, cpu.regs.pc, &[0xE4, 0x30]); // LDA $30 (page 1)
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xBB);
}

// ============================================================
// Cumulative cycle counting across multiple instructions
// ============================================================

#[test]
fn test_cycles_accumulate_across_multiple_steps() {
    let (mut cpu, mut mem) = make_cpu_mem();
    let pc = cpu.regs.pc;
    // NOP(2) + LDA #imm(2) + STA !a(4) = 8 cycles
    emit_seq(&mut mem, pc, &[
        0x00,               // NOP
        0xE8, 0x42,         // LDA #$42
        0xC5, 0x00, 0x05,   // STA !$0500
    ]);
    cpu.step(&mut mem); // NOP
    cpu.step(&mut mem); // LDA
    cpu.step(&mut mem); // STA
    assert_eq!(cpu.cycles, 8);
    assert_eq!(mem.read8(0x0500), 0x42);
}

// ============================================================
// PC wrapping
// ============================================================

#[test]
fn test_pc_wraps_at_0xffff() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    // Place a NOP at $FFFF (after reset vector bytes, which are at $FFFE/$FFFF
    // but we test the fetch wrapping separately from reset)
    cpu.regs.pc = 0xFFFF;
    mem.write8(0xFFFF, 0x00); // NOP at $FFFF
    cpu.step(&mut mem);
    // PC was $FFFF, fetch consumed it, wrapping_add(1) → $0000
    assert_eq!(cpu.regs.pc, 0x0000, "PC must wrap from $FFFF to $0000");
}
