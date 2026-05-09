/// Control flow instruction tests
/// Currently covers:
///   - BRA ($2F) — branch always
///   - BEQ ($F0) — branch if Z set
///  - BNE ($D0) — branch if Z clear
///  - BPL ($10) — branch if N clear
/// - BMI ($30) — branch if N set
/// - BVC ($50) — branch if V clear
/// - BVS ($70) — branch if V set
/// - BCC ($90) — branch if C clear
/// - BCS ($B0) — branch if C set
/// - CALL ($3F) — absolute subroutine call

use apu::cpu::{Spc700, FLAG_C, FLAG_N, FLAG_Z, FLAG_V};
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

// ============================================================
// BNE ($D0) — branch if Zero clear
// ============================================================

#[test]
fn test_bne_taken_when_z_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xD0);
    mem.write8(0x0201, 0x04); // +4 → $0206
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}

#[test]
fn test_bne_taken_negative_offset() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xD0);
    mem.write8(0x0201, 0xFD); // -3 → $01FF
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x01FF);
}

#[test]
fn test_bne_taken_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xD0);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_bne_not_taken_when_z_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_Z;
    mem.write8(0x0200, 0xD0);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}

#[test]
fn test_bne_not_taken_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_Z;
    mem.write8(0x0200, 0xD0);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_bne_not_taken_when_z_and_n_set() {
    // Z set is all that matters — N alongside it must not rescue the branch
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_Z | FLAG_N;
    mem.write8(0x0200, 0xD0);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}

#[test]
fn test_bne_taken_when_only_n_set() {
    // N set but Z clear — BNE must branch
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0200, 0xD0);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}

#[test]
fn test_bne_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_C;
    mem.write8(0x0200, 0xD0);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_C);
}

#[test]
fn test_bne_used_as_loop_counter() {
    // Classic decrement loop: LDA #2, loop: SBC #1, BNE back
    // After 2 iterations A reaches 0 and Z is set so BNE falls through
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C; // carry set = no borrow for SBC

    // $0200: SBC A,#1  ($A8 $01)
    // $0202: BNE -4    ($D0 $FC) — back to $0200
    mem.write8(0x0200, 0xA8);
    mem.write8(0x0201, 0x01);
    mem.write8(0x0202, 0xD0);
    mem.write8(0x0203, 0xFC); // -4 → back to $0200
    mem.write8(0x0204, 0x00); // NOP — loop exit

    cpu.regs.a = 2;

    // Iteration 1: A=2 → SBC → A=1, Z clear → BNE taken
    cpu.step(&mut mem);
    cpu.regs.psw |= FLAG_C; // restore carry
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0200, "BNE must loop back");

    // Iteration 2: A=1 → SBC → A=0, Z set → BNE not taken
    cpu.step(&mut mem);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0204, "BNE must fall through when Z set");
}

// ============================================================
// BPL ($10) — branch if Negative clear
// ============================================================

#[test]
fn test_bpl_taken_when_n_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x10);
    mem.write8(0x0201, 0x04); // +4 → $0206
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}

#[test]
fn test_bpl_taken_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x10);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_bpl_not_taken_when_n_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0200, 0x10);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}

#[test]
fn test_bpl_not_taken_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0200, 0x10);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_bpl_taken_when_z_set_but_n_clear() {
    // Z alongside a clear N must not block the branch
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_Z;
    mem.write8(0x0200, 0x10);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}

#[test]
fn test_bpl_not_taken_when_n_and_z_both_set() {
    // N set is all that matters — Z alongside it must not rescue the branch
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    mem.write8(0x0200, 0x10);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}

#[test]
fn test_bpl_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_Z | FLAG_C;
    mem.write8(0x0200, 0x10);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_Z | FLAG_C);
}

#[test]
fn test_bpl_used_after_positive_load() {
    // LDA #$01 sets N=0, Z=0 → BPL must branch
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xE8); // LDA #$01
    mem.write8(0x0201, 0x01);
    mem.write8(0x0202, 0x10); // BPL +2
    mem.write8(0x0203, 0x02);
    mem.write8(0x0204, 0xFF); // skipped
    mem.write8(0x0205, 0xFF); // skipped
    mem.write8(0x0206, 0x00); // NOP — branch target

    cpu.step(&mut mem); // LDA #$01
    cpu.step(&mut mem); // BPL — taken
    assert_eq!(cpu.regs.pc, 0x0206);
}

#[test]
fn test_bpl_not_taken_after_negative_load() {
    // LDA #$80 sets N=1 → BPL must not branch
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xE8); // LDA #$80
    mem.write8(0x0201, 0x80);
    mem.write8(0x0202, 0x10); // BPL +2
    mem.write8(0x0203, 0x02);

    cpu.step(&mut mem); // LDA #$80
    cpu.step(&mut mem); // BPL — not taken
    assert_eq!(cpu.regs.pc, 0x0204);
}

// ============================================================
// BMI ($30) — branch if Negative set
// ============================================================
 
#[test]
fn test_bmi_taken_when_n_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0200, 0x30);
    mem.write8(0x0201, 0x04); // +4 → $0206
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bmi_taken_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0200, 0x30);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}
 
#[test]
fn test_bmi_not_taken_when_n_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x30);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}
 
#[test]
fn test_bmi_not_taken_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x30);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}
 
#[test]
fn test_bmi_taken_with_other_flags_also_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z | FLAG_C;
    mem.write8(0x0200, 0x30);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bmi_not_taken_when_only_z_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_Z;
    mem.write8(0x0200, 0x30);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}
 
#[test]
fn test_bmi_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_C;
    mem.write8(0x0200, 0x30);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_C);
}
 
#[test]
fn test_bmi_taken_after_negative_load() {
    // LDA #$80 sets N=1 → BMI must branch
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xE8); // LDA #$80
    mem.write8(0x0201, 0x80);
    mem.write8(0x0202, 0x30); // BMI +2
    mem.write8(0x0203, 0x02);
    mem.write8(0x0204, 0xFF); // skipped
    mem.write8(0x0205, 0xFF); // skipped
    mem.write8(0x0206, 0x00); // NOP — branch target
 
    cpu.step(&mut mem); // LDA #$80 → N=1
    cpu.step(&mut mem); // BMI — taken
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bmi_not_taken_after_positive_load() {
    // LDA #$01 sets N=0 → BMI must not branch
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xE8); // LDA #$01
    mem.write8(0x0201, 0x01);
    mem.write8(0x0202, 0x30); // BMI +2
    mem.write8(0x0203, 0x02);
 
    cpu.step(&mut mem); // LDA #$01 → N=0
    cpu.step(&mut mem); // BMI — not taken
    assert_eq!(cpu.regs.pc, 0x0204);
}

// ============================================================
// BVC ($50) — branch if Overflow clear
// ============================================================
 
#[test]
fn test_bvc_taken_when_v_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x50);
    mem.write8(0x0201, 0x04); // +4 → $0206
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bvc_taken_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x50);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}
 
#[test]
fn test_bvc_not_taken_when_v_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_V;
    mem.write8(0x0200, 0x50);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}
 
#[test]
fn test_bvc_not_taken_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_V;
    mem.write8(0x0200, 0x50);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}
 
#[test]
fn test_bvc_taken_when_n_and_z_set_but_v_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    mem.write8(0x0200, 0x50);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bvc_not_taken_when_v_and_n_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_V | FLAG_N;
    mem.write8(0x0200, 0x50);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}
 
#[test]
fn test_bvc_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_C;
    mem.write8(0x0200, 0x50);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_C);
}
 
#[test]
fn test_bvc_taken_after_adc_no_overflow() {
    // $01 + $01 = $02 — no signed overflow, V=0 → BVC taken
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    mem.write8(0x0200, 0x88); // ADC A,#$01
    mem.write8(0x0201, 0x01);
    mem.write8(0x0202, 0x50); // BVC +2
    mem.write8(0x0203, 0x02);
    mem.write8(0x0204, 0xFF); // skipped
    mem.write8(0x0205, 0xFF); // skipped
    mem.write8(0x0206, 0x00); // NOP — branch target
 
    cpu.step(&mut mem); // ADC — V=0
    cpu.step(&mut mem); // BVC — taken
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bvc_not_taken_after_adc_overflow() {
    // $70 + $10 = $80 — pos+pos=neg, signed overflow, V=1 → BVC not taken
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x70;
    mem.write8(0x0200, 0x88); // ADC A,#$10
    mem.write8(0x0201, 0x10);
    mem.write8(0x0202, 0x50); // BVC +2
    mem.write8(0x0203, 0x02);
 
    cpu.step(&mut mem); // ADC — V=1
    cpu.step(&mut mem); // BVC — not taken
    assert_eq!(cpu.regs.pc, 0x0204);
}

// ============================================================
// BVS ($70) — branch if Overflow set
// ============================================================
 
#[test]
fn test_bvs_taken_when_v_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_V;
    mem.write8(0x0200, 0x70);
    mem.write8(0x0201, 0x04); // +4 → $0206
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bvs_taken_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_V;
    mem.write8(0x0200, 0x70);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}
 
#[test]
fn test_bvs_not_taken_when_v_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x70);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}
 
#[test]
fn test_bvs_not_taken_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x70);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}
 
#[test]
fn test_bvs_taken_with_other_flags_also_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_V | FLAG_N | FLAG_C;
    mem.write8(0x0200, 0x70);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bvs_not_taken_when_only_n_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0200, 0x70);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}
 
#[test]
fn test_bvs_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_V | FLAG_C;
    mem.write8(0x0200, 0x70);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_V | FLAG_C);
}
 
#[test]
fn test_bvs_taken_after_adc_overflow() {
    // $70 + $10 = $80 — pos+pos=neg, V=1 → BVS taken
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x70;
    mem.write8(0x0200, 0x88); // ADC A,#$10
    mem.write8(0x0201, 0x10);
    mem.write8(0x0202, 0x70); // BVS +2
    mem.write8(0x0203, 0x02);
    mem.write8(0x0204, 0xFF); // skipped
    mem.write8(0x0205, 0xFF); // skipped
    mem.write8(0x0206, 0x00); // NOP — branch target
 
    cpu.step(&mut mem); // ADC — V=1
    cpu.step(&mut mem); // BVS — taken
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bvs_not_taken_after_adc_no_overflow() {
    // $01 + $01 = $02 — no overflow, V=0 → BVS not taken
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    mem.write8(0x0200, 0x88); // ADC A,#$01
    mem.write8(0x0201, 0x01);
    mem.write8(0x0202, 0x70); // BVS +2
    mem.write8(0x0203, 0x02);
 
    cpu.step(&mut mem); // ADC — V=0
    cpu.step(&mut mem); // BVS — not taken
    assert_eq!(cpu.regs.pc, 0x0204);
}

// ============================================================
// BCC ($90) — branch if Carry clear
// ============================================================
 
#[test]
fn test_bcc_taken_when_c_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x90);
    mem.write8(0x0201, 0x04); // +4 → $0206
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bcc_taken_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0x90);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}
 
#[test]
fn test_bcc_not_taken_when_c_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0x90);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}
 
#[test]
fn test_bcc_not_taken_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0x90);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}
 
#[test]
fn test_bcc_taken_when_n_set_but_c_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0200, 0x90);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bcc_not_taken_when_c_and_n_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_N;
    mem.write8(0x0200, 0x90);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}
 
#[test]
fn test_bcc_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    mem.write8(0x0200, 0x90);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z);
}
 
#[test]
fn test_bcc_taken_after_adc_no_carry() {
    // $01 + $01 = $02 — no carry out, C=0 → BCC taken
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    mem.write8(0x0200, 0x88); // ADC A,#$01
    mem.write8(0x0201, 0x01);
    mem.write8(0x0202, 0x90); // BCC +2
    mem.write8(0x0203, 0x02);
    mem.write8(0x0204, 0xFF); // skipped
    mem.write8(0x0205, 0xFF); // skipped
    mem.write8(0x0206, 0x00); // NOP — branch target
 
    cpu.step(&mut mem); // ADC — C=0
    cpu.step(&mut mem); // BCC — taken
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bcc_not_taken_after_adc_carry() {
    // $FF + $01 = $00 with carry out, C=1 → BCC not taken
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0200, 0x88); // ADC A,#$01
    mem.write8(0x0201, 0x01);
    mem.write8(0x0202, 0x90); // BCC +2
    mem.write8(0x0203, 0x02);
 
    cpu.step(&mut mem); // ADC — C=1
    cpu.step(&mut mem); // BCC — not taken
    assert_eq!(cpu.regs.pc, 0x0204);
}

// ============================================================
// BCS ($B0) — branch if Carry set
// ============================================================
 
#[test]
fn test_bcs_taken_when_c_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0xB0);
    mem.write8(0x0201, 0x04); // +4 → $0206
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bcs_taken_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0xB0);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}
 
#[test]
fn test_bcs_not_taken_when_c_clear() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xB0);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}
 
#[test]
fn test_bcs_not_taken_costs_2_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0x00;
    mem.write8(0x0200, 0xB0);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 2);
}
 
#[test]
fn test_bcs_taken_with_other_flags_also_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_N | FLAG_Z;
    mem.write8(0x0200, 0xB0);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bcs_not_taken_when_only_n_set() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0x0200, 0xB0);
    mem.write8(0x0201, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0202);
}
 
#[test]
fn test_bcs_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_N;
    mem.write8(0x0200, 0xB0);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_C | FLAG_N);
}
 
#[test]
fn test_bcs_taken_after_adc_carry() {
    // $FF + $01 = $00 with carry out, C=1 → BCS taken
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xFF;
    mem.write8(0x0200, 0x88); // ADC A,#$01
    mem.write8(0x0201, 0x01);
    mem.write8(0x0202, 0xB0); // BCS +2
    mem.write8(0x0203, 0x02);
    mem.write8(0x0204, 0xFF); // skipped
    mem.write8(0x0205, 0xFF); // skipped
    mem.write8(0x0206, 0x00); // NOP — branch target
 
    cpu.step(&mut mem); // ADC — C=1
    cpu.step(&mut mem); // BCS — taken
    assert_eq!(cpu.regs.pc, 0x0206);
}
 
#[test]
fn test_bcs_not_taken_after_adc_no_carry() {
    // $01 + $01 = $02 — no carry, C=0 → BCS not taken
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    mem.write8(0x0200, 0x88); // ADC A,#$01
    mem.write8(0x0201, 0x01);
    mem.write8(0x0202, 0xB0); // BCS +2
    mem.write8(0x0203, 0x02);
 
    cpu.step(&mut mem); // ADC — C=0
    cpu.step(&mut mem); // BCS — not taken
    assert_eq!(cpu.regs.pc, 0x0204);
}

// ============================================================
// stack_push / stack_pop helpers
//
// Tested indirectly through CALL/RET since the helpers are private.
// Covers: correct stack page ($0100-$01FF), SP decrement/increment,
// SP wrap at $00→$FF and $FF→$00, and LIFO ordering.
// ============================================================
 
#[test]
fn test_stack_push_writes_to_stack_page() {
    // CALL writes to $0100|SP — verify the byte lands in the stack page
    let (mut cpu, mut mem) = make();
    cpu.regs.sp = 0xFE; // SP starts at $FE
    mem.write8(0x0200, 0x3F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem); // CALL pushes hi then lo of $0203
    // hi byte of $0203 = $02 written to $01FE
    // lo byte of $0203 = $03 written to $01FD
    assert_eq!(mem.read8(0x01FE), 0x02, "hi byte at $01FE");
    assert_eq!(mem.read8(0x01FD), 0x03, "lo byte at $01FD");
    assert_eq!(cpu.regs.sp, 0xFC);
}
 
#[test]
fn test_stack_pop_reads_from_stack_page() {
    // RET reads from $0100|SP after increment
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x3F); // CALL $0500
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    mem.write8(0x0500, 0x6F); // RET
    cpu.step(&mut mem); // CALL
    let sp_after_call = cpu.regs.sp;
    cpu.step(&mut mem); // RET
    // RET incremented SP twice and read from the stack page
    assert_eq!(cpu.regs.sp, sp_after_call.wrapping_add(2));
}
 
#[test]
fn test_stack_sp_wraps_from_00_to_ff_on_push() {
    // SP at $01 — after two pushes (CALL) SP wraps to $FF
    let (mut cpu, mut mem) = make();
    cpu.regs.sp = 0x01;
    mem.write8(0x0200, 0x3F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem); // CALL pushes 2 bytes: $01 → $00 → $FF
    assert_eq!(cpu.regs.sp, 0xFF);
}
 
#[test]
fn test_stack_sp_wraps_from_ff_to_00_on_pop() {
    // Prime the stack manually, set SP to $FD so RET increments to $FF then $00
    let (mut cpu, mut mem) = make();
    // Write a return address at $01FE (hi) and $01FF (lo)
    mem.write8(0x01FE, 0x03); // hi = $03
    mem.write8(0x01FF, 0x00); // lo = $00 → return to $0300
    cpu.regs.sp = 0xFD;
    mem.write8(0x0200, 0x6F); // RET
    cpu.step(&mut mem);
    // SP: $FD → $FE (read lo $00) → $FF (read hi $03) — wait, wrong order
    // RET pops lo first: SP $FD→$FE reads $01FE = $03 (lo)
    // then pops hi: SP $FE→$FF reads $01FF = $00 (hi)
    // PC = ($00 << 8) | $03 = $0003
    assert_eq!(cpu.regs.sp, 0xFF);
    assert_eq!(cpu.regs.pc, 0x0003);
}
 
#[test]
fn test_stack_lifo_ordering() {
    // Push A three times via CALL (we use the return address bytes),
    // then pop via RET — must come back in reverse order.
    // Simpler: use three nested CALLs and verify RET unwinds correctly.
    let (mut cpu, mut mem) = make();
 
    // Outer: CALL $0400 at $0200 → return = $0203
    mem.write8(0x0200, 0x3F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x04);
    // Middle: CALL $0600 at $0400 → return = $0403
    mem.write8(0x0400, 0x3F);
    mem.write8(0x0401, 0x00);
    mem.write8(0x0402, 0x06);
    // Inner: RET at $0600 → $0403
    mem.write8(0x0600, 0x6F);
    // Back: RET at $0403 → $0203
    mem.write8(0x0403, 0x6F);
 
    cpu.step(&mut mem); // CALL $0400
    cpu.step(&mut mem); // CALL $0600
    cpu.step(&mut mem); // RET → $0403
    assert_eq!(cpu.regs.pc, 0x0403, "first RET must return to inner caller");
    cpu.step(&mut mem); // RET → $0203
    assert_eq!(cpu.regs.pc, 0x0203, "second RET must return to outer caller");
}

// ============================================================
// CALL ($3F) — push return address, jump to absolute target
// ============================================================
 
#[test]
fn test_call_jumps_to_target() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x3F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05); // target = $0500
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0500);
}
 
#[test]
fn test_call_pushes_return_address() {
    // CALL at $0200 is 3 bytes → return address = $0203
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x3F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    let sp = cpu.regs.sp;
    cpu.step(&mut mem);
    let hi = mem.read8(0x0100 | sp as u16);
    let lo = mem.read8(0x0100 | sp.wrapping_sub(1) as u16);
    let ret = ((hi as u16) << 8) | lo as u16;
    assert_eq!(ret, 0x0203, "return address must be instruction after CALL");
}
 
#[test]
fn test_call_decrements_sp_by_2() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x3F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    let sp_before = cpu.regs.sp;
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, sp_before.wrapping_sub(2));
}
 
#[test]
fn test_call_costs_8_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x3F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 8);
}
 
#[test]
fn test_call_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_N;
    mem.write8(0x0200, 0x3F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_C | FLAG_N);
}
 
#[test]
fn test_call_sp_wraps_at_00() {
    // SP at $01 — after pushing 2 bytes SP wraps to $FF
    let (mut cpu, mut mem) = make();
    cpu.regs.sp = 0x01;
    mem.write8(0x0200, 0x3F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, 0xFF);
}

// ============================================================
// RET ($6F) — pop return address and jump
// ============================================================
 
#[test]
fn test_ret_jumps_to_return_address() {
    // CALL pushes $0203, RET must restore PC to $0203
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x3F); // CALL $0500
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    mem.write8(0x0500, 0x6F); // RET
    cpu.step(&mut mem); // CALL
    cpu.step(&mut mem); // RET
    assert_eq!(cpu.regs.pc, 0x0203);
}
 
#[test]
fn test_ret_restores_sp() {
    let (mut cpu, mut mem) = make();
    let sp_before = cpu.regs.sp;
    mem.write8(0x0200, 0x3F); // CALL $0500
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    mem.write8(0x0500, 0x6F); // RET
    cpu.step(&mut mem); // CALL — SP -= 2
    cpu.step(&mut mem); // RET  — SP += 2
    assert_eq!(cpu.regs.sp, sp_before, "SP must be restored after CALL+RET");
}
 
#[test]
fn test_ret_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x3F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    mem.write8(0x0500, 0x6F);
    cpu.step(&mut mem); // CALL
    let cycles_before = cpu.cycles;
    cpu.step(&mut mem); // RET
    assert_eq!(cpu.cycles - cycles_before, 5);
}
 
#[test]
fn test_ret_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_N;
    mem.write8(0x0200, 0x3F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    mem.write8(0x0500, 0x6F);
    cpu.step(&mut mem);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_C | FLAG_N);
}
 
#[test]
fn test_call_ret_round_trip_multiple_times() {
    // Call and return three times — SP must be the same each time
    let (mut cpu, mut mem) = make();
    let sp_start = cpu.regs.sp;
 
    for _ in 0..3 {
        mem.write8(0x0200, 0x3F); // CALL $0500
        mem.write8(0x0201, 0x00);
        mem.write8(0x0202, 0x05);
        mem.write8(0x0500, 0x6F); // RET
        cpu.regs.pc = 0x0200;
        cpu.step(&mut mem);
        cpu.step(&mut mem);
        assert_eq!(cpu.regs.sp, sp_start, "SP must be restored after each CALL+RET");
        assert_eq!(cpu.regs.pc, 0x0203);
    }
}
 
#[test]
fn test_nested_call_ret() {
    // Outer call → inner call → inner ret → outer ret
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x3F); // CALL $0400 (outer)
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x04);
    mem.write8(0x0400, 0x3F); // CALL $0600 (inner)
    mem.write8(0x0401, 0x00);
    mem.write8(0x0402, 0x06);
    mem.write8(0x0600, 0x6F); // RET → back to $0403
    mem.write8(0x0403, 0x6F); // RET → back to $0203
 
    cpu.step(&mut mem); // outer CALL → $0400
    cpu.step(&mut mem); // inner CALL → $0600
    cpu.step(&mut mem); // inner RET  → $0403
    assert_eq!(cpu.regs.pc, 0x0403);
    cpu.step(&mut mem); // outer RET  → $0203
    assert_eq!(cpu.regs.pc, 0x0203);
}

// ============================================================
// PCALL ($4F) — push return address, jump to $FF00 + u
// ============================================================
 
#[test]
fn test_pcall_jumps_to_ff00_plus_u() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4F);
    mem.write8(0x0201, 0x20); // u = $20 → target = $FF20
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0xFF20);
}
 
#[test]
fn test_pcall_u_zero_jumps_to_ff00() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4F);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0xFF00);
}
 
#[test]
fn test_pcall_u_ff_jumps_to_ffff() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4F);
    mem.write8(0x0201, 0xFF);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0xFFFF);
}
 
#[test]
fn test_pcall_pushes_return_address() {
    // PCALL is 2 bytes → return address = $0202
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4F);
    mem.write8(0x0201, 0x20);
    let sp = cpu.regs.sp;
    cpu.step(&mut mem);
    let hi = mem.read8(0x0100 | sp as u16);
    let lo = mem.read8(0x0100 | sp.wrapping_sub(1) as u16);
    let ret = ((hi as u16) << 8) | lo as u16;
    assert_eq!(ret, 0x0202);
}
 
#[test]
fn test_pcall_decrements_sp_by_2() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4F);
    mem.write8(0x0201, 0x00);
    let sp_before = cpu.regs.sp;
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, sp_before.wrapping_sub(2));
}
 
#[test]
fn test_pcall_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4F);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}
 
#[test]
fn test_pcall_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_Z;
    mem.write8(0x0200, 0x4F);
    mem.write8(0x0201, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_C | FLAG_Z);
}
 
#[test]
fn test_pcall_ret_round_trip() {
    // PCALL to $FF20, RET returns to $0202
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4F); // PCALL $20
    mem.write8(0x0201, 0x20);
    mem.write8(0xFF20, 0x6F); // RET
    let sp_before = cpu.regs.sp;
    cpu.step(&mut mem); // PCALL
    cpu.step(&mut mem); // RET
    assert_eq!(cpu.regs.pc, 0x0202);
    assert_eq!(cpu.regs.sp, sp_before);
}

// ============================================================
// TCALL ($01/$11/.../$F1) — call via vector table at $FFDE-(n*2)
// ============================================================
 
#[test]
fn test_tcall_0_reads_vector_at_ffde() {
    // n=0: vector at $FFDE/$FFDF
    let (mut cpu, mut mem) = make();
    mem.write8(0xFFDE, 0x00);
    mem.write8(0xFFDF, 0x08); // target = $0800
    mem.write8(0x0200, 0x01); // TCALL 0
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0800);
}
 
#[test]
fn test_tcall_1_reads_vector_at_ffdc() {
    // n=1: vector at $FFDE - 2 = $FFDC/$FFDD
    let (mut cpu, mut mem) = make();
    mem.write8(0xFFDC, 0x00);
    mem.write8(0xFFDD, 0x09); // target = $0900
    mem.write8(0x0200, 0x11); // TCALL 1
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0900);
}
 
#[test]
fn test_tcall_15_reads_vector_at_ffc0() {
    // n=15: vector at $FFDE - 30 = $FFC0/$FFC1
    let (mut cpu, mut mem) = make();
    mem.write8(0xFFC0, 0x00);
    mem.write8(0xFFC1, 0x0A); // target = $0A00
    mem.write8(0x0200, 0xF1); // TCALL 15
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0A00);
}
 
#[test]
fn test_tcall_pushes_return_address() {
    // TCALL is 1 byte → return address = $0201
    let (mut cpu, mut mem) = make();
    mem.write8(0xFFDE, 0x00);
    mem.write8(0xFFDF, 0x08);
    mem.write8(0x0200, 0x01);
    let sp = cpu.regs.sp;
    cpu.step(&mut mem);
    let hi = mem.read8(0x0100 | sp as u16);
    let lo = mem.read8(0x0100 | sp.wrapping_sub(1) as u16);
    let ret = ((hi as u16) << 8) | lo as u16;
    assert_eq!(ret, 0x0201);
}
 
#[test]
fn test_tcall_decrements_sp_by_2() {
    let (mut cpu, mut mem) = make();
    mem.write8(0xFFDE, 0x00);
    mem.write8(0xFFDF, 0x08);
    mem.write8(0x0200, 0x01);
    let sp_before = cpu.regs.sp;
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, sp_before.wrapping_sub(2));
}
 
#[test]
fn test_tcall_costs_8_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0xFFDE, 0x00);
    mem.write8(0xFFDF, 0x08);
    mem.write8(0x0200, 0x01);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 8);
}
 
#[test]
fn test_tcall_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_Z;
    mem.write8(0xFFDE, 0x00);
    mem.write8(0xFFDF, 0x08);
    mem.write8(0x0200, 0x01);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_C | FLAG_Z);
}
 
#[test]
fn test_tcall_vector_is_little_endian() {
    // Low byte at vector_addr, high byte at vector_addr+1
    let (mut cpu, mut mem) = make();
    mem.write8(0xFFDE, 0x34); // lo
    mem.write8(0xFFDF, 0x12); // hi → target = $1234
    mem.write8(0x0200, 0x01);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x1234);
}
 
#[test]
fn test_tcall_ret_round_trip() {
    let (mut cpu, mut mem) = make();
    mem.write8(0xFFDE, 0x00);
    mem.write8(0xFFDF, 0x08); // target = $0800
    mem.write8(0x0200, 0x01); // TCALL 0
    mem.write8(0x0800, 0x6F); // RET → back to $0201
    let sp_before = cpu.regs.sp;
    cpu.step(&mut mem); // TCALL
    cpu.step(&mut mem); // RET
    assert_eq!(cpu.regs.pc, 0x0201);
    assert_eq!(cpu.regs.sp, sp_before);
}
 
#[test]
fn test_tcall_all_16_vector_addresses() {
    // Verify each n maps to the correct vector address
    let expected: [(u8, u16); 16] = [
        (0x01, 0xFFDE), (0x11, 0xFFDC), (0x21, 0xFFDA), (0x31, 0xFFD8),
        (0x41, 0xFFD6), (0x51, 0xFFD4), (0x61, 0xFFD2), (0x71, 0xFFD0),
        (0x81, 0xFFCE), (0x91, 0xFFCC), (0xA1, 0xFFCA), (0xB1, 0xFFC8),
        (0xC1, 0xFFC6), (0xD1, 0xFFC4), (0xE1, 0xFFC2), (0xF1, 0xFFC0),
    ];
 
    for (opcode, vector_addr) in expected {
        let (mut cpu, mut mem) = make();
        mem.write8(vector_addr,     0x34);
        mem.write8(vector_addr + 1, 0x12); // target = $1234 for each
        mem.write8(0x0200, opcode);
        cpu.step(&mut mem);
        assert_eq!(cpu.regs.pc, 0x1234,
            "opcode {opcode:#04X} must read vector at {vector_addr:#06X}");
    }
}

// ============================================================
// PUSH A ($2D) — push accumulator onto stack
// ============================================================
 
#[test]
fn test_push_a_writes_a_to_stack() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a  = 0xAB;
    cpu.regs.sp = 0xFE;
    mem.write8(0x0200, 0x2D);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x01FE), 0xAB, "A must be written to $01FE");
}
 
#[test]
fn test_push_a_decrements_sp() {
    let (mut cpu, mut mem) = make();
    let sp = cpu.regs.sp;
    mem.write8(0x0200, 0x2D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, sp.wrapping_sub(1));
}
 
#[test]
fn test_push_a_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x2D);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}
 
#[test]
fn test_push_a_does_not_modify_a() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x55;
    mem.write8(0x0200, 0x2D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x55, "PUSH must not modify A");
}
 
#[test]
fn test_push_a_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_N;
    mem.write8(0x0200, 0x2D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_C | FLAG_N);
}
 
#[test]
fn test_push_a_sp_wraps_from_00_to_ff() {
    let (mut cpu, mut mem) = make();
    cpu.regs.sp = 0x00;
    mem.write8(0x0200, 0x2D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, 0xFF);
}

// ============================================================
// POP A ($AE) — pop accumulator from stack
// ============================================================

#[test]
fn test_pop_a_reads_from_stack() {
    let (mut cpu, mut mem) = make();
    cpu.regs.sp = 0xFE;
    mem.write8(0x01FF, 0xAB); // value waiting on stack
    mem.write8(0x0200, 0xAE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0xAB);
}

#[test]
fn test_pop_a_increments_sp() {
    let (mut cpu, mut mem) = make();
    let sp = cpu.regs.sp;
    mem.write8(0x0200, 0xAE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, sp.wrapping_add(1));
}

#[test]
fn test_pop_a_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xAE);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_pop_a_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_N;
    mem.write8(0x0200, 0xAE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_C | FLAG_N);
}

#[test]
fn test_pop_a_sp_wraps_from_ff_to_00() {
    let (mut cpu, mut mem) = make();
    cpu.regs.sp = 0xFF;
    mem.write8(0x0100, 0x77); // value at $0100 (SP wraps to $00, reads $0100)
    mem.write8(0x0200, 0xAE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, 0x00);
    assert_eq!(cpu.regs.a, 0x77);
}

#[test]
fn test_push_a_pop_a_round_trip() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0xCD;
    mem.write8(0x0200, 0x2D); // PUSH A
    mem.write8(0x0201, 0xAE); // POP A
    cpu.step(&mut mem);       // PUSH — A=0xCD pushed
    cpu.regs.a = 0x00;        // clobber after push
    cpu.step(&mut mem);       // POP — restores 0xCD
    assert_eq!(cpu.regs.a, 0xCD);
}

// ============================================================
// PUSH X ($4D) — push X register onto stack
// ============================================================

#[test]
fn test_push_x_writes_x_to_stack() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x  = 0x12;
    cpu.regs.sp = 0xFE;
    mem.write8(0x0200, 0x4D);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x01FE), 0x12);
}

#[test]
fn test_push_x_decrements_sp() {
    let (mut cpu, mut mem) = make();
    let sp = cpu.regs.sp;
    mem.write8(0x0200, 0x4D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, sp.wrapping_sub(1));
}

#[test]
fn test_push_x_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x4D);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_push_x_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_N;
    mem.write8(0x0200, 0x4D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_C | FLAG_N);
}

// ============================================================
// POP X ($CE) — pop X register from stack
// ============================================================

#[test]
fn test_pop_x_reads_from_stack() {
    let (mut cpu, mut mem) = make();
    cpu.regs.sp = 0xFE;
    mem.write8(0x01FF, 0x34);
    mem.write8(0x0200, 0xCE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x34);
}

#[test]
fn test_pop_x_increments_sp() {
    let (mut cpu, mut mem) = make();
    let sp = cpu.regs.sp;
    mem.write8(0x0200, 0xCE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, sp.wrapping_add(1));
}

#[test]
fn test_pop_x_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xCE);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_pop_x_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_Z;
    mem.write8(0x0200, 0xCE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_C | FLAG_Z);
}

#[test]
fn test_push_x_pop_x_round_trip() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0xEF;
    mem.write8(0x0200, 0x4D); // PUSH X
    mem.write8(0x0201, 0xCE); // POP X
    cpu.step(&mut mem);       // PUSH
    cpu.regs.x = 0x00;        // clobber after push
    cpu.step(&mut mem);       // POP
    assert_eq!(cpu.regs.x, 0xEF);
}

// ============================================================
// PUSH Y ($6D) — push Y register onto stack
// ============================================================

#[test]
fn test_push_y_writes_y_to_stack() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y  = 0x56;
    cpu.regs.sp = 0xFE;
    mem.write8(0x0200, 0x6D);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x01FE), 0x56);
}

#[test]
fn test_push_y_decrements_sp() {
    let (mut cpu, mut mem) = make();
    let sp = cpu.regs.sp;
    mem.write8(0x0200, 0x6D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, sp.wrapping_sub(1));
}

#[test]
fn test_push_y_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x6D);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_push_y_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_V;
    mem.write8(0x0200, 0x6D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_V);
}

// ============================================================
// POP Y ($EE) — pop Y register from stack
// ============================================================

#[test]
fn test_pop_y_reads_from_stack() {
    let (mut cpu, mut mem) = make();
    cpu.regs.sp = 0xFE;
    mem.write8(0x01FF, 0x78);
    mem.write8(0x0200, 0xEE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x78);
}

#[test]
fn test_pop_y_increments_sp() {
    let (mut cpu, mut mem) = make();
    let sp = cpu.regs.sp;
    mem.write8(0x0200, 0xEE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, sp.wrapping_add(1));
}

#[test]
fn test_pop_y_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xEE);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_pop_y_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_V;
    mem.write8(0x0200, 0xEE);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_C | FLAG_V);
}

#[test]
fn test_push_y_pop_y_round_trip() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x9A;
    mem.write8(0x0200, 0x6D); // PUSH Y
    mem.write8(0x0201, 0xEE); // POP Y
    cpu.step(&mut mem);       // PUSH
    cpu.regs.y = 0x00;        // clobber after push
    cpu.step(&mut mem);       // POP
    assert_eq!(cpu.regs.y, 0x9A);
}

// ============================================================
// PUSH PSW ($0D) — push processor status word onto stack
// ============================================================

#[test]
fn test_push_psw_writes_psw_to_stack() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_N | FLAG_Z;
    cpu.regs.sp  = 0xFE;
    mem.write8(0x0200, 0x0D);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x01FE), FLAG_C | FLAG_N | FLAG_Z);
}

#[test]
fn test_push_psw_decrements_sp() {
    let (mut cpu, mut mem) = make();
    let sp = cpu.regs.sp;
    mem.write8(0x0200, 0x0D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, sp.wrapping_sub(1));
}

#[test]
fn test_push_psw_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x0D);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_push_psw_preserves_psw() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0xFF;
    mem.write8(0x0200, 0x0D);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, 0xFF, "PUSH PSW must not modify PSW");
}

// ============================================================
// POP PSW ($8E) — pop processor status word from stack
// ============================================================

#[test]
fn test_pop_psw_restores_all_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.sp  = 0xFE;
    mem.write8(0x01FF, 0xFF); // all flags set
    mem.write8(0x0200, 0x8E);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, 0xFF);
}

#[test]
fn test_pop_psw_increments_sp() {
    let (mut cpu, mut mem) = make();
    let sp = cpu.regs.sp;
    mem.write8(0x0200, 0x8E);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.sp, sp.wrapping_add(1));
}

#[test]
fn test_pop_psw_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x8E);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_pop_psw_clears_all_flags() {
    // POP PSW of $00 must clear every flag
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = 0xFF;
    cpu.regs.sp  = 0xFE;
    mem.write8(0x01FF, 0x00);
    mem.write8(0x0200, 0x8E);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, 0x00);
}

#[test]
fn test_push_psw_pop_psw_round_trip() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_C | FLAG_V | FLAG_N;
    mem.write8(0x0200, 0x0D); // PUSH PSW
    mem.write8(0x0201, 0x8E); // POP PSW
    cpu.step(&mut mem);       // PUSH
    cpu.regs.psw = 0x00;      // clobber after push
    cpu.step(&mut mem);       // POP
    assert_eq!(cpu.regs.psw, FLAG_C | FLAG_V | FLAG_N);
}

#[test]
fn test_all_registers_push_pop_independent() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a   = 0x11;
    cpu.regs.x   = 0x22;
    cpu.regs.y   = 0x33;
    cpu.regs.psw = FLAG_C;
    mem.write8(0x0200, 0x2D); // PUSH A
    mem.write8(0x0201, 0x4D); // PUSH X
    mem.write8(0x0202, 0x6D); // PUSH Y
    mem.write8(0x0203, 0x0D); // PUSH PSW
    mem.write8(0x0204, 0x8E); // POP PSW
    mem.write8(0x0205, 0xEE); // POP Y
    mem.write8(0x0206, 0xCE); // POP X
    mem.write8(0x0207, 0xAE); // POP A

    // push all four
    for _ in 0..4 { cpu.step(&mut mem); }

    // clobber after all pushes are done
    cpu.regs.a   = 0x00;
    cpu.regs.x   = 0x00;
    cpu.regs.y   = 0x00;
    cpu.regs.psw = 0x00;

    // pop all four
    for _ in 0..4 { cpu.step(&mut mem); }

    assert_eq!(cpu.regs.psw, FLAG_C, "PSW must be restored");
    assert_eq!(cpu.regs.y,   0x33,   "Y must be restored");
    assert_eq!(cpu.regs.x,   0x22,   "X must be restored");
    assert_eq!(cpu.regs.a,   0x11,   "A must be restored");
}

// ============================================================
// SLEEP ($EF) / STOP ($FF) — halt instructions
// Both need to be implemented
// ============================================================

#[test]
#[should_panic(expected = "SLEEP: halt until interrupt")]
fn test_sleep_panics_with_todo() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xEF);
    cpu.step(&mut mem);
}

#[test]
#[should_panic(expected = "STOP: permanent halt")]
fn test_stop_panics_with_todo() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0xFF);
    cpu.step(&mut mem);
}