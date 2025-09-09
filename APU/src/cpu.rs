use crate::memory::Memory;

#[derive(Default)]
pub struct Registers {
    pub a: u8,     // Accumulator
    pub x: u8,     // Index X
    pub y: u8,     // Index Y
    pub sp: u8,    // Stack Pointer
    pub pc: u16,   // Program Counter
    pub psw: u8,   // Processor Status Word
}

pub struct Spc700 {
    pub regs: Registers,
    pub cycles: u32,
}

impl Spc700 {
    pub fn new() -> Self {
        Self {
            regs: Registers::default(),
            cycles: 0,
        }
    }

    pub fn reset(&mut self, mem: &mut Memory) {
        self.regs.pc = mem.read16(0xFFFE); // Reset vector
        self.regs.sp = 0xFF;
        self.regs.psw = 0;
    }

    pub fn step(&mut self, mem: &mut Memory) {
        let opcode = mem.read8(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);

        match opcode {
            0x00 => self.inst_nop(),
            0xE8 => self.inst_mov_a_x(),
            0xF8 => self.inst_mov_a_y(),
            0x8B => self.inst_mov_x_a(),
            0x9B => self.inst_mov_y_a(),
            _ => unimplemented!("Opcode {:02X} not yet implemented", opcode),
        }        
    }

    fn inst_mov_a_x(&mut self) {
        self.regs.a = self.regs.x; // copy X into A
        self.cycles += 2; // SPC700: 2 cycles
    }
    fn inst_mov_a_y(&mut self) {
        self.regs.a = self.regs.y; // copy Y into A
        self.cycles += 2; // SPC700: 2 cycles
    }
    fn inst_mov_x_a(&mut self) {
        self.regs.x = self.regs.a; // copy A into X
        self.cycles += 2; // SPC700: 2 cycles
    }
    fn inst_mov_y_a(&mut self) {
        self.regs.y = self.regs.a; // copy A into Y
        self.cycles += 2; // SPC700: 2 cycles
    }

    fn inst_nop(&mut self) { // do nothing
        // NOP takes 2 cycles on the SPC700
        self.cycles += 2;
    }
    
}
