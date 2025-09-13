use apu::{Memory, Spc700};
use apu::cpu::FLAG_N;
use apu::cpu::FLAG_Z;
use apu::cpu::FLAG_C;
use apu::cpu::FLAG_V;

#[test]
fn test_nop() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    mem.write8(0x0200, 0x00); // NOP
    cpu.regs.pc = 0x0200;

    cpu.step(&mut mem);
    assert_eq!(cpu.regs.pc, 0x0201, "PC should advance by 1");
    assert_eq!(cpu.cycles, 2, "NOP should take 2 cycles");
}

#[test]
fn test_mov_a_x() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.x = 0x42;
    mem.write8(0x0200, 0xE8); // MOV A,X
    cpu.regs.pc = 0x0200;

    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x42, "A should equal X");
    assert_eq!(cpu.cycles, 2);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_mov_a_y() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.y = 0x37;
    mem.write8(0x0200, 0xF8); // MOV A,Y
    cpu.regs.pc = 0x0200;

    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x37, "A should equal Y");
    assert_eq!(cpu.cycles, 2);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_mov_x_a() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.a = 0x55;
    mem.write8(0x0200, 0x8B); // MOV X,A
    cpu.regs.pc = 0x0200;

    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x55, "X should equal A");
    assert_eq!(cpu.cycles, 2);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_mov_y_a() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.a = 0x99;
    mem.write8(0x0200, 0x9B); // MOV Y,A
    cpu.regs.pc = 0x0200;

    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x99, "Y should equal A");
    assert_eq!(cpu.cycles, 2);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_lda_imm_step() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();

    // Place instruction at 0x200: LDA #$42
    cpu.regs.pc = 0x200;
    mem.write8(0x200, 0xA9); // LDA #imm opcode
    mem.write8(0x201, 0x42); // operand

    cpu.step(&mut mem);

    assert_eq!(cpu.regs.a, 0x42);
    assert!(!cpu.get_flag(0x02)); // not zero
    assert!(!cpu.get_flag(0x80)); // not negative
}


#[test]
fn test_ldx_imm_negative() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();
    cpu.regs.pc = 0x200;
    mem.write8(0x200, 0x80); // sets negative flag

    cpu.inst_ldx_imm(&mut mem);

    assert_eq!(cpu.regs.x, 0x80);
    assert!(cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_ldy_imm_zero() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();
    cpu.regs.pc = 0x200;
    mem.write8(0x200, 0x00); // sets zero flag

    cpu.inst_ldy_imm(&mut mem);

    assert_eq!(cpu.regs.y, 0x00);
    assert!(cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_sta_abs() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();

    cpu.regs.pc = 0x200;
    cpu.regs.a = 0x55;

    // STA $1234
    mem.write8(0x200, 0x8D); // opcode
    mem.write16(0x201, 0x1234); // address

    cpu.step(&mut mem);

    assert_eq!(mem.read8(0x1234), 0x55);
}

#[test]
fn test_stx_abs() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();

    cpu.regs.pc = 0x200;
    cpu.regs.x = 0xAA;

    // STX $4321
    mem.write8(0x200, 0x8E); 
    mem.write16(0x201, 0x4321);

    cpu.step(&mut mem);

    assert_eq!(mem.read8(0x4321), 0xAA);
}

#[test]
fn test_sty_abs() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();

    cpu.regs.pc = 0x200;
    cpu.regs.y = 0x99;

    // STY $5678
    mem.write8(0x200, 0x8F); 
    mem.write16(0x201, 0x5678);

    cpu.step(&mut mem);

    assert_eq!(mem.read8(0x5678), 0x99);
}

#[test]
fn test_lda_abs() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();

    cpu.regs.pc = 0x200;

    // LDA $1234
    mem.write8(0x200, 0xAD); 
    mem.write16(0x201, 0x1234);
    mem.write8(0x1234, 0x77);

    cpu.step(&mut mem);

    assert_eq!(cpu.regs.a, 0x77);
    assert!(!cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_ldx_abs() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();

    cpu.regs.pc = 0x200;

    // LDX $4321
    mem.write8(0x200, 0xAE); 
    mem.write16(0x201, 0x4321);
    mem.write8(0x4321, 0x80); // will set negative flag

    cpu.step(&mut mem);

    assert_eq!(cpu.regs.x, 0x80);
    assert!(cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_ldy_abs() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();

    cpu.regs.pc = 0x200;

    // LDY $5678
    mem.write8(0x200, 0xAF); 
    mem.write16(0x201, 0x5678);
    mem.write8(0x5678, 0x00); // will set zero flag

    cpu.step(&mut mem);

    assert_eq!(cpu.regs.y, 0x00);
    assert!(cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_lda_dp() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();
    cpu.regs.pc = 0x200;

    mem.write8(0x200, 0xA5); // LDA dp
    mem.write8(0x201, 0x10); // direct page $0010
    mem.write8(0x0010, 0x55);

    cpu.step(&mut mem);

    assert_eq!(cpu.regs.a, 0x55);
    assert!(!cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_sta_dp() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();
    cpu.regs.pc = 0x200;
    cpu.regs.a = 0x99;

    mem.write8(0x200, 0x85); // STA dp
    mem.write8(0x201, 0x20); // direct page $0020

    cpu.step(&mut mem);

    assert_eq!(mem.read8(0x0020), 0x99);
}


#[test]
fn test_adc_no_carry() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();

    cpu.regs.pc = 0x200;
    cpu.regs.a = 0x10;
    mem.write8(0x200, 0x05); // ADC #$05

    cpu.inst_adc_imm(&mut mem);

    assert_eq!(cpu.regs.a, 0x15);
    assert!(!cpu.get_flag(FLAG_C));
    assert!(!cpu.get_flag(FLAG_V));
    assert!(!cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_adc_with_carry_and_overflow() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();

    cpu.regs.pc = 0x200;
    cpu.regs.a = 0x80;
    mem.write8(0x200, 0x80); // ADC #$80

    cpu.inst_adc_imm(&mut mem);

    assert_eq!(cpu.regs.a, 0x00); // 0x80 + 0x80 = 0x100 -> 0x00
    assert!(cpu.get_flag(FLAG_C));
    assert!(cpu.get_flag(FLAG_V));
    assert!(cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_cmp_greater() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();

    cpu.regs.pc = 0x200;
    cpu.regs.a = 0x50;
    mem.write8(0x200, 0x40); // CMP #$40

    cpu.inst_cmp_imm(&mut mem);

    assert!(cpu.get_flag(FLAG_C));
    assert!(!cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_cmp_equal() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();

    cpu.regs.pc = 0x200;
    cpu.regs.a = 0x42;
    mem.write8(0x200, 0x42); // CMP #$42

    cpu.inst_cmp_imm(&mut mem);

    assert!(cpu.get_flag(FLAG_C));
    assert!(cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_cmp_less() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();

    cpu.regs.pc = 0x200;
    cpu.regs.a = 0x30;
    mem.write8(0x200, 0x40); // CMP #$40

    cpu.inst_cmp_imm(&mut mem);

    assert!(!cpu.get_flag(FLAG_C));
    assert!(!cpu.get_flag(FLAG_Z));
    assert!(cpu.get_flag(FLAG_N));
}
