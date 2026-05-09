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
