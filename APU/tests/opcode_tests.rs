use apu::{Memory, Spc700};
use apu::cpu::FLAG_N;
use apu::cpu::FLAG_Z;


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
