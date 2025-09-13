use crate::memory::Memory;

#[derive(Default)]
pub struct Registers {
    pub a: u8,     // Accumulator
    pub x: u8,     // Index X
    pub y: u8,     // Index Y
    pub sp: u8,    // Stack Pointer
    pub pc: u16,   // Program Counter
    pub psw: u8,   // Processor Status Word (flags)
}

// Processor status flags
pub const FLAG_C: u8 = 0x01; // Carry
pub const FLAG_Z: u8 = 0x02; // Zero
pub const FLAG_I: u8 = 0x04; // Interrupt Disable
pub const FLAG_H: u8 = 0x08; // Half-Carry
pub const FLAG_B: u8 = 0x10; // Break
pub const FLAG_P: u8 = 0x20; // Direct Page
pub const FLAG_V: u8 = 0x40; // Overflow
pub const FLAG_N: u8 = 0x80; // Negative

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

            // Immediate loads
            0xA9 => self.inst_lda_imm(mem),
            0xA2 => self.inst_ldx_imm(mem),
            0xA0 => self.inst_ldy_imm(mem),

            0x8D => self.inst_sta_abs(mem),
            0x8E => self.inst_stx_abs(mem),
            0x8F => self.inst_sty_abs(mem),
            0xAD => self.inst_lda_abs(mem),
            0xAE => self.inst_ldx_abs(mem),
            0xAF => self.inst_ldy_abs(mem),

            0xA5 => self.inst_lda_dp(mem),
            0xA6 => self.inst_ldx_dp(mem),
            0xA7 => self.inst_ldy_dp(mem),
            0x85 => self.inst_sta_dp(mem),
            0x86 => self.inst_stx_dp(mem),
            0x87 => self.inst_sty_dp(mem),

            0x69 => self.inst_adc_imm(mem),
            0xC9 => self.inst_cmp_imm(mem),

            0xE9 => self.inst_sbc_imm(mem), // SBC #imm
            0x29 => self.inst_and_imm(mem), // AND #imm
            0x09 => self.inst_ora_imm(mem), // ORA #imm
            0x49 => self.inst_eor_imm(mem), // EOR #imm

            _ => unimplemented!("Opcode {:02X} not yet implemented", opcode),
        }        
    }

    // Flag helpers
    pub fn set_flag(&mut self, mask: u8, value: bool) {
        if value {
            self.regs.psw |= mask;
        } else {
            self.regs.psw &= !mask;
        }
    }

    pub fn get_flag(&self, mask: u8) -> bool {
        (self.regs.psw & mask) != 0
    }      

    fn set_zn_flags(&mut self, value: u8) {
        self.set_flag(FLAG_Z, value == 0);
        self.set_flag(FLAG_N, (value & 0x80) != 0);
    }

    // Implemented instructions
    fn inst_mov_a_x(&mut self) {
        self.regs.a = self.regs.x;
        self.cycles += 2;
    }
    fn inst_mov_a_y(&mut self) {
        self.regs.a = self.regs.y;
        self.cycles += 2;
    }
    fn inst_mov_x_a(&mut self) {
        self.regs.x = self.regs.a;
        self.cycles += 2;
    }
    fn inst_mov_y_a(&mut self) {
        self.regs.y = self.regs.a;
        self.cycles += 2;
    }
    fn inst_nop(&mut self) {
        self.cycles += 2;
    }

    pub fn inst_lda_imm(&mut self, mem: &mut Memory) {
        let value = mem.read8(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.regs.a = value;
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    pub fn inst_ldx_imm(&mut self, mem: &mut Memory) {
        let value = mem.read8(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.regs.x = value;
        self.set_zn_flags(self.regs.x);
        self.cycles += 2;
    }

    pub fn inst_ldy_imm(&mut self, mem: &mut Memory) {
        let value = mem.read8(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.regs.y = value;
        self.set_zn_flags(self.regs.y);
        self.cycles += 2;
    }

    pub fn inst_sta_abs(&mut self, mem: &mut Memory) {
        let addr = mem.read16(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(2);
        mem.write8(addr, self.regs.a);
        self.cycles += 4;
    }

    pub fn inst_stx_abs(&mut self, mem: &mut Memory) {
        let addr = mem.read16(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(2);
        mem.write8(addr, self.regs.x);
        self.cycles += 4;
    }

    pub fn inst_sty_abs(&mut self, mem: &mut Memory) {
        let addr = mem.read16(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(2);
        mem.write8(addr, self.regs.y);
        self.cycles += 4;
    }

    pub fn inst_lda_abs(&mut self, mem: &mut Memory) {
        let addr = mem.read16(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(2);
        self.regs.a = mem.read8(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 4;
    }

    pub fn inst_ldx_abs(&mut self, mem: &mut Memory) {
        let addr = mem.read16(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(2);
        self.regs.x = mem.read8(addr);
        self.set_zn_flags(self.regs.x);
        self.cycles += 4;
    }

    pub fn inst_ldy_abs(&mut self, mem: &mut Memory) {
        let addr = mem.read16(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(2);
        self.regs.y = mem.read8(addr);
        self.set_zn_flags(self.regs.y);
        self.cycles += 4;
    }

    // Load accumulator from direct page
    pub fn inst_lda_dp(&mut self, mem: &mut Memory) {
        let addr = mem.read8(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.regs.a = mem.read8(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 3; // Direct page is faster than absolute
    }

    pub fn inst_ldx_dp(&mut self, mem: &mut Memory) {
        let addr = mem.read8(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.regs.x = mem.read8(addr);
        self.set_zn_flags(self.regs.x);
        self.cycles += 3;
    }

    pub fn inst_ldy_dp(&mut self, mem: &mut Memory) {
        let addr = mem.read8(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.regs.y = mem.read8(addr);
        self.set_zn_flags(self.regs.y);
        self.cycles += 3;
    }

    // Store accumulator into direct page
    pub fn inst_sta_dp(&mut self, mem: &mut Memory) {
        let addr = mem.read8(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        mem.write8(addr, self.regs.a);
        self.cycles += 3;
    }

    pub fn inst_stx_dp(&mut self, mem: &mut Memory) {
        let addr = mem.read8(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        mem.write8(addr, self.regs.x);
        self.cycles += 3;
    }

    pub fn inst_sty_dp(&mut self, mem: &mut Memory) {
        let addr = mem.read8(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        mem.write8(addr, self.regs.y);
        self.cycles += 3;
    }

    pub fn inst_adc_imm(&mut self, mem: &mut Memory) {
        let value = mem.read8(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);

        let carry_in = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let result = self.regs.a as u16 + value as u16 + carry_in as u16;

        // Update flags
        self.set_flag(FLAG_C, result > 0xFF);
        let result_u8 = result as u8;
        self.set_zn_flags(result_u8);

        // Overflow flag
        self.set_flag(
            FLAG_V,
            (!(self.regs.a ^ value) & (self.regs.a ^ result_u8) & 0x80) != 0,
        );

        self.regs.a = result_u8;
        self.cycles += 2;
    }

    /// Compare memory with accumulator (sets flags only)
    pub fn inst_cmp_imm(&mut self, mem: &mut Memory) {
        let value = mem.read8(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);

        let result = self.regs.a.wrapping_sub(value);

        self.set_flag(FLAG_C, self.regs.a >= value);
        self.set_zn_flags(result);

        self.cycles += 2;
    }

    pub fn inst_sbc_imm(&mut self, mem: &mut Memory) {
        let value = mem.read8(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);

        let carry_in = if self.get_flag(FLAG_C) { 0 } else { 1 }; // SPC700 uses inverted carry
        let result = self.regs.a as i16 - value as i16 - carry_in as i16;

        self.set_flag(FLAG_C, result >= 0);
        let result_u8 = result as u8;
        self.set_zn_flags(result_u8);
        self.set_flag(
            FLAG_V,
            ((self.regs.a ^ result_u8) & (self.regs.a ^ value) & 0x80) != 0,
        );

        self.regs.a = result_u8;
        self.cycles += 2;
    }

    /// Bitwise AND with accumulator
    pub fn inst_and_imm(&mut self, mem: &mut Memory) {
        let value = mem.read8(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);

        self.regs.a &= value;
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    /// Bitwise OR with accumulator
    pub fn inst_ora_imm(&mut self, mem: &mut Memory) {
        let value = mem.read8(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);

        self.regs.a |= value;
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    /// Bitwise XOR with accumulator
    pub fn inst_eor_imm(&mut self, mem: &mut Memory) {
        let value = mem.read8(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);

        self.regs.a ^= value;
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }
}
