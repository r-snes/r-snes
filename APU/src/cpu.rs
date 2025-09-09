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
            0xE8 => self.inst_mov_a_x(), // Example
            _ => unimplemented!("Opcode {:02X} not yet implemented", opcode),
        }
    }

    fn inst_mov_a_x(&mut self) {
        self.regs.a = self.regs.x;
        self.cycles += 2; // Example cycle count
    }
}
