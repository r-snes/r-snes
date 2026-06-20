/// Jump/branch/misc instruction tests

use apu::cpu::{Spc700, FLAG_B, FLAG_I, FLAG_N, FLAG_P, FLAG_Z};
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
// JMP !a ($5F)
// ============================================================

#[test]
fn test_jmp_abs_jumps_to_target() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x5F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0500);
}

#[test]
fn test_jmp_abs_costs_3_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0200, 0x5F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 3);
}

#[test]
fn test_jmp_abs_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    mem.write8(0x0200, 0x5F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z);
}

// ============================================================
// JMP [!a+X] ($1F)
// ============================================================

#[test]
fn test_jmp_abs_x_ind_jumps_through_pointer() {
    let (mut cpu, mut mem) = make();
    cpu.regs.x = 0x02;
    mem.write8(0x0502, 0x00); // pointer lo at $0500+X
    mem.write8(0x0503, 0x06); // pointer hi -> target $0600
    mem.write8(0x0200, 0x1F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0600);
}

#[test]
fn test_jmp_abs_x_ind_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0500, 0x00);
    mem.write8(0x0501, 0x06);
    mem.write8(0x0200, 0x1F);
    mem.write8(0x0201, 0x00);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// RETI ($7F)
// ============================================================

#[test]
fn test_reti_restores_pc_and_psw() {
    let (mut cpu, mut mem) = make();
    // Manually push PCH, PCL, then PSW to simulate an interrupt entry
    cpu.regs.sp = 0xFF;
    mem.write8(0x01FF, 0x06); // PCH = $06
    mem.write8(0x01FE, 0x00); // PCL = $00 -> target $0600
    mem.write8(0x01FD, FLAG_N); // saved PSW
    cpu.regs.sp = 0xFC;

    mem.write8(0x0200, 0x7F);
    cpu.step(&mut mem);

    assert_eq!(cpu.regs.pc, 0x0600);
    assert_eq!(cpu.regs.psw, FLAG_N);
}

#[test]
fn test_reti_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.sp = 0xFC;
    mem.write8(0x01FD, 0x00);
    mem.write8(0x01FE, 0x00);
    mem.write8(0x01FF, 0x02);
    mem.write8(0x0200, 0x7F);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// CBNE dp,rel ($2E)
// ============================================================

#[test]
fn test_cbne_dp_branches_when_not_equal() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    mem.write8(0x0020, 0x20); // different from A
    mem.write8(0x0200, 0x2E);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_cbne_dp_does_not_branch_when_equal() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    mem.write8(0x0020, 0x10); // equal to A
    mem.write8(0x0200, 0x2E);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0203);
}

#[test]
fn test_cbne_dp_does_not_modify_a_or_memory() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    mem.write8(0x0020, 0x20);
    mem.write8(0x0200, 0x2E);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x10);
    assert_eq!(mem.read8(0x0020), 0x20);
}

#[test]
fn test_cbne_dp_not_taken_costs_5_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x2E);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 5);
}

#[test]
fn test_cbne_dp_taken_costs_7_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x2E);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 7);
}

// ============================================================
// CBNE dp+X,rel ($DE)
// ============================================================

#[test]
fn test_cbne_dp_x_branches_when_not_equal() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    cpu.regs.x = 0x02;
    mem.write8(0x0022, 0x20);
    mem.write8(0x0200, 0xDE);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_cbne_dp_x_wraps_within_page() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x10;
    cpu.regs.x = 0x02;
    mem.write8(0x0001, 0x20);
    mem.write8(0x0200, 0xDE);
    mem.write8(0x0201, 0xFF);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_cbne_dp_x_not_taken_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0xDE);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

#[test]
fn test_cbne_dp_x_taken_costs_8_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.a = 0x01;
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0xDE);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 8);
}

// ============================================================
// DBNZ dp,rel ($6E)
// ============================================================

#[test]
fn test_dbnz_dp_decrements_and_branches_when_nonzero() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x02);
    mem.write8(0x0200, 0x6E);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x01);
    assert_eq!(cpu.regs.pc, 0x0208);
}

#[test]
fn test_dbnz_dp_does_not_branch_when_result_zero() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x6E);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0x00);
    assert_eq!(cpu.regs.pc, 0x0203);
}

#[test]
fn test_dbnz_dp_wraps_from_00_to_ff() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x00);
    mem.write8(0x0200, 0x6E);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(mem.read8(0x0020), 0xFF);
    assert_eq!(cpu.regs.pc, 0x0208, "0xFF != 0, branch should be taken");
}

#[test]
fn test_dbnz_dp_does_not_modify_flags() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N | FLAG_Z;
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x6E);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.psw, FLAG_N | FLAG_Z);
}

#[test]
fn test_dbnz_dp_not_taken_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x01);
    mem.write8(0x0200, 0x6E);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

#[test]
fn test_dbnz_dp_taken_costs_8_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0x0020, 0x02);
    mem.write8(0x0200, 0x6E);
    mem.write8(0x0201, 0x20);
    mem.write8(0x0202, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 8);
}

// ============================================================
// DBNZ Y,rel ($FE)
// ============================================================

#[test]
fn test_dbnz_y_decrements_and_branches_when_nonzero() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x02;
    mem.write8(0x0200, 0xFE);
    mem.write8(0x0201, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x01);
    assert_eq!(cpu.regs.pc, 0x0207);
}

#[test]
fn test_dbnz_y_does_not_branch_when_result_zero() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x01;
    mem.write8(0x0200, 0xFE);
    mem.write8(0x0201, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x00);
    assert_eq!(cpu.regs.pc, 0x0202);
}

#[test]
fn test_dbnz_y_wraps_from_00_to_ff() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x00;
    mem.write8(0x0200, 0xFE);
    mem.write8(0x0201, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0xFF);
    assert_eq!(cpu.regs.pc, 0x0207);
}

#[test]
fn test_dbnz_y_not_taken_costs_4_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x01;
    mem.write8(0x0200, 0xFE);
    mem.write8(0x0201, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 4);
}

#[test]
fn test_dbnz_y_taken_costs_6_cycles() {
    let (mut cpu, mut mem) = make();
    cpu.regs.y = 0x02;
    mem.write8(0x0200, 0xFE);
    mem.write8(0x0201, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 6);
}

// ============================================================
// BRK ($0F)
// ============================================================

#[test]
fn test_brk_jumps_via_vector() {
    let (mut cpu, mut mem) = make();
    mem.write8(0xFFDE, 0x00);
    mem.write8(0xFFDF, 0x06); // BRK vector -> $0600
    mem.write8(0x0200, 0x0F);
    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0600);
}

#[test]
fn test_brk_sets_break_and_interrupt_disable_flags() {
    let (mut cpu, mut mem) = make();
    mem.write8(0xFFDE, 0x00);
    mem.write8(0xFFDF, 0x06);
    mem.write8(0x0200, 0x0F);
    cpu.step(&mut mem);
    assert!(cpu.get_flag(FLAG_B));
    assert!(cpu.get_flag(FLAG_I));
}

#[test]
fn test_brk_pushes_return_pc_and_psw() {
    let (mut cpu, mut mem) = make();
    cpu.regs.psw = FLAG_N;
    mem.write8(0xFFDE, 0x00);
    mem.write8(0xFFDF, 0x06);
    mem.write8(0x0200, 0x0F);
    cpu.step(&mut mem);

    // SP should have decremented by 3 (PCH, PCL, PSW)
    assert_eq!(cpu.regs.sp, 0xFC);
    // Stack holds, from top down: PSW, PCL, PCH
    assert_eq!(mem.read8(0x01FD), FLAG_N, "saved PSW");
    assert_eq!(mem.read8(0x01FE), 0x01, "PCL: return address low byte");
    assert_eq!(mem.read8(0x01FF), 0x02, "PCH: return address high byte");
}

#[test]
fn test_brk_costs_8_cycles() {
    let (mut cpu, mut mem) = make();
    mem.write8(0xFFDE, 0x00);
    mem.write8(0xFFDF, 0x06);
    mem.write8(0x0200, 0x0F);
    cpu.step(&mut mem);
    assert_eq!(cpu.cycles, 8);
}
