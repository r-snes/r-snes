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
    assert_eq!(cpu.regs.pc, 0x0201);
    assert_eq!(cpu.cycles, 2);
}

#[test]
fn test_mov_a_x() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.x = 0x42;
    mem.write8(0x0200, 0x7D); // MOV A,X
    cpu.regs.pc = 0x0200;

    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x42);
    assert_eq!(cpu.cycles, 2);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_mov_a_y() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.y = 0x37;
    mem.write8(0x0200, 0xDD); // MOV A,Y
    cpu.regs.pc = 0x0200;

    cpu.step(&mut mem);
    assert_eq!(cpu.regs.a, 0x37);
    assert_eq!(cpu.cycles, 2);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_mov_x_a() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.a = 0x55;
    mem.write8(0x0200, 0x5D); // MOV X,A
    cpu.regs.pc = 0x0200;

    cpu.step(&mut mem);
    assert_eq!(cpu.regs.x, 0x55);
    assert_eq!(cpu.cycles, 2);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_mov_y_a() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.a = 0x99;
    mem.write8(0x0200, 0xFD); // MOV Y,A
    cpu.regs.pc = 0x0200;

    cpu.step(&mut mem);
    assert_eq!(cpu.regs.y, 0x99);
    assert_eq!(cpu.cycles, 2);
    assert_eq!(cpu.regs.pc, 0x0201);
}

#[test]
fn test_lda_imm_step() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();

    cpu.regs.pc = 0x0200;
    mem.write8(0x0200, 0xE8); // LDA #imm
    mem.write8(0x0201, 0x42);

    cpu.step(&mut mem);

    assert_eq!(cpu.regs.a, 0x42);
    assert!(!cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_ldx_imm_negative() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();

    cpu.regs.pc = 0x0200;
    mem.write8(0x0200, 0xCD); // LDX #imm
    mem.write8(0x0201, 0x80);

    cpu.step(&mut mem);

    assert_eq!(cpu.regs.x, 0x80);
    assert!(cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_Z));
}

#[test]
fn test_ldy_imm_zero() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();

    cpu.regs.pc = 0x0200;
    mem.write8(0x0200, 0x8D); // LDY #imm
    mem.write8(0x0201, 0x00);

    cpu.step(&mut mem);

    assert_eq!(cpu.regs.y, 0x00);
    assert!(cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_sta_abs() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();
    cpu.regs.pc = 0x0200;
    cpu.regs.a = 0x55;

    mem.write8(0x0200, 0xC5); // STA abs (MOV !a,A)
    mem.write16(0x0201, 0x1234); // address (little-endian via write16)

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
    mem.write8(0x200, 0xc9); 
    mem.write16(0x201, 0x4321);

    cpu.step(&mut mem);

    assert_eq!(mem.read8(0x4321), 0xAA);
}

#[test]
fn test_sty_abs() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();
    cpu.regs.pc = 0x0200;
    cpu.regs.y = 0x99;

    mem.write8(0x0200, 0xCC); // STY abs
    mem.write16(0x0201, 0x5678);

    cpu.step(&mut mem);

    assert_eq!(mem.read8(0x5678), 0x99);
}

#[test]
fn test_lda_abs() {
    let mut mem = Memory::new();
    let mut cpu = Spc700::new();

    cpu.regs.pc = 0x200;

    // LDA $1234
    mem.write8(0x200, 0xE5); 
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
    mem.write8(0x200, 0xE9); 
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
    mem.write8(0x200, 0xEC); 
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

    mem.write8(0x200, 0xE4); // LDA dp
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

    cpu.regs.pc = 0x0200;
    cpu.regs.a = 0x99;

    mem.write8(0x0200, 0xC4); // MOV d,A (replacing STA)
    mem.write8(0x0201, 0x20); // direct page $20

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

#[test]
fn test_sbc_no_borrow() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.pc = 0x200;
    cpu.regs.a = 0x50;
    cpu.set_flag(FLAG_C, true); // carry set -> no borrow
    mem.write8(0x200, 0x10); // SBC #$10

    cpu.inst_sbc_imm(&mut mem);

    assert_eq!(cpu.regs.a, 0x40);
    assert!(cpu.get_flag(FLAG_C));
    assert!(!cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
    assert!(!cpu.get_flag(FLAG_V)); // no signed overflow should occur
}

#[test]
fn test_sbc_with_overflow() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();

    cpu.regs.pc = 0x200;
    cpu.regs.a = 0x80; // -128 in signed 8-bit
    cpu.set_flag(FLAG_C, true); // carry set -> no borrow
    mem.write8(0x200, 0x01); // SBC #$01

    cpu.inst_sbc_imm(&mut mem);

    assert_eq!(cpu.regs.a, 0x7F); // result wraps to +127
    assert!(cpu.get_flag(FLAG_C)); // carry remains set (no borrow)
    assert!(!cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N)); // positive result
    assert!(cpu.get_flag(FLAG_V));  // signed overflow triggered
}

#[test]
fn test_and_sets_flags() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.pc = 0x200;
    cpu.regs.a = 0b10101010;
    mem.write8(0x200, 0b11001100);

    cpu.inst_and_imm(&mut mem);

    assert_eq!(cpu.regs.a, 0b10001000);
    assert!(!cpu.get_flag(FLAG_Z));
    assert!(cpu.get_flag(FLAG_N)); // negative bit set
}

#[test]
fn test_ora_sets_flags() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.pc = 0x200;
    cpu.regs.a = 0b10101010;
    mem.write8(0x200, 0b01010101);

    cpu.inst_ora_imm(&mut mem);

    assert_eq!(cpu.regs.a, 0b11111111);
    assert!(!cpu.get_flag(FLAG_Z));
    assert!(cpu.get_flag(FLAG_N));
}

#[test]
fn test_eor_sets_flags() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();
    cpu.regs.pc = 0x200;
    cpu.regs.a = 0b11110000;
    mem.write8(0x200, 0b10101010);

    cpu.inst_eor_imm(&mut mem);

    assert_eq!(cpu.regs.a, 0b01011010);
    assert!(!cpu.get_flag(FLAG_Z));
    assert!(!cpu.get_flag(FLAG_N));
}

#[test]
fn test_adc_overflow_flag_set() {
    let mut cpu = Spc700::new();
    let mut mem = Memory::new();

    cpu.regs.pc = 0x200;
    cpu.regs.a = 0x50; // +80 signed
    mem.write8(0x200, 0x50); // ADC #$50 (+80 signed)

    cpu.inst_adc_imm(&mut mem);

    // 0x50 + 0x50 = 0xA0
    assert_eq!(cpu.regs.a, 0xA0);

    // Check flags
    assert!(!cpu.get_flag(FLAG_C)); // no carry out of 8 bits
    assert!(cpu.get_flag(FLAG_V));  // signed overflow (80 + 80 = -96)
    assert!(cpu.get_flag(FLAG_N));  // result has high bit set
    assert!(!cpu.get_flag(FLAG_Z)); // result != 0
}
