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
            0x00 => self.inst_nop(), // NOP
        
            // Register moves
            0x7D => self.inst_mov_a_x(), // MOV A, X
            0xDD => self.inst_mov_a_y(), // MOV A, Y
            0x5D => self.inst_mov_x_a(), // MOV X, A
            0xFD => self.inst_mov_y_a(), // MOV Y, A
        
            // Immediate loads
            0xE8 => self.inst_lda_imm(mem), // LDA #imm
            0xCD => self.inst_ldx_imm(mem), // LDX #imm
            0x8D => self.inst_ldy_imm(mem), // LDY #imm
        
            // Absolute loads
            0xE5 => self.inst_lda_abs(mem), // MOV A, !a
            0xE9 => self.inst_ldx_abs(mem), // MOV X, !a
            0xEC => self.inst_ldy_abs(mem), // MOV Y, !a
        
            // Direct Page loads
            0xE4 => self.inst_lda_dp(mem), // MOV A, d
            0xF8 => self.inst_ldx_dp(mem), // MOV X, d
            0xEB => self.inst_ldy_dp(mem), // MOV Y, d
        
            // Stores
            0xC4 => self.inst_sta_dp(mem),  // MOV d, A
            0xC5 => self.inst_sta_abs(mem), // MOV !a, A
            0xC9 => self.inst_stx_abs(mem), // MOV !a, X
            0xCC => self.inst_sty_abs(mem), // MOV !a, Y
        
            // Arithmetic & logic
            0x88 => self.inst_adc_imm(mem), // ADC #imm
            0xA8 => self.inst_sbc_imm(mem), // SBC #imm
            0x68 => self.inst_cmp_imm(mem), // CMP #imm
            0x28 => self.inst_and_imm(mem), // AND #imm
            0x08 => self.inst_ora_imm(mem), // ORA #imm
            0x48 => self.inst_eor_imm(mem), // EOR #imm
        
            // Catch-all
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

    fn dp_base(&self) -> u16 {
        if self.get_flag(FLAG_P) {
            0x0100
        } else {
            0x0000
        }
    }

    /// Reads the next byte from memory and increments the program counter.
    fn read_immediate(&mut self, mem: &Memory) -> u8 {
        let value = mem.read8(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        value
    }

    /// Read a 16-bit little-endian immediate (two bytes) and advance PC by 2.
    fn read_immediate16(&mut self, mem: &mut Memory) -> u16 {
        let lo = self.read_immediate(mem) as u16;
        let hi = self.read_immediate(mem) as u16;
        lo | (hi << 8)
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

    pub fn inst_lda_imm(&mut self, mem: &Memory) {
        let value = self.read_immediate(mem);
        self.regs.a = value;
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    pub fn inst_ldx_imm(&mut self, mem: &Memory) {
        let value = self.read_immediate(mem);
        self.regs.x = value;
        self.set_zn_flags(self.regs.x);
        self.cycles += 2;
    }

    pub fn inst_ldy_imm(&mut self, mem: &Memory) {
        let value = self.read_immediate(mem);
        self.regs.y = value;
        self.set_zn_flags(self.regs.y);
        self.cycles += 2;
    }

    pub fn inst_sta_abs(&mut self, mem: &mut Memory) {
        // Read the 16-bit address in little-endian
        let lo = mem.read8(self.regs.pc) as u16;
        let hi = mem.read8(self.regs.pc.wrapping_add(1)) as u16;
        let addr = lo | (hi << 8);
    
        // Move PC past the operand
        self.regs.pc = self.regs.pc.wrapping_add(2);
    
        // Store A into memory
        mem.write8(addr, self.regs.a);
    
        // Increment cycles
        self.cycles += 4;
    }
    
    pub fn inst_stx_abs(&mut self, mem: &mut Memory) {
        let lo = mem.read8(self.regs.pc) as u16;
        let hi = mem.read8(self.regs.pc.wrapping_add(1)) as u16;
        let addr = lo | (hi << 8);
    
        self.regs.pc = self.regs.pc.wrapping_add(2);
    
        mem.write8(addr, self.regs.x);
        self.cycles += 4;
    }
    
    pub fn inst_sty_abs(&mut self, mem: &mut Memory) {
        let lo = mem.read8(self.regs.pc) as u16;
        let hi = mem.read8(self.regs.pc.wrapping_add(1)) as u16;
        let addr = lo | (hi << 8);
    
        self.regs.pc = self.regs.pc.wrapping_add(2);
    
        mem.write8(addr, self.regs.y);
        self.cycles += 4;
    }

    pub fn inst_lda_abs(&mut self, mem: &mut Memory) {
        let lo = mem.read8(self.regs.pc) as u16;
        let hi = mem.read8(self.regs.pc.wrapping_add(1)) as u16;
        let addr = lo | (hi << 8);
    
        self.regs.pc = self.regs.pc.wrapping_add(2);
        self.regs.a = mem.read8(addr as u16);
        self.set_zn_flags(self.regs.a);
        self.cycles += 4;
    }
    
    pub fn inst_ldx_abs(&mut self, mem: &mut Memory) {
        let lo = mem.read8(self.regs.pc) as u16;
        let hi = mem.read8(self.regs.pc.wrapping_add(1)) as u16;
        let addr = lo | (hi << 8);
    
        self.regs.pc = self.regs.pc.wrapping_add(2);
        self.regs.x = mem.read8(addr as u16);
        self.set_zn_flags(self.regs.x);
        self.cycles += 4;
    }
    
    pub fn inst_ldy_abs(&mut self, mem: &mut Memory) {
        let lo = mem.read8(self.regs.pc) as u16;
        let hi = mem.read8(self.regs.pc.wrapping_add(1)) as u16;
        let addr = lo | (hi << 8);
    
        self.regs.pc = self.regs.pc.wrapping_add(2);
        self.regs.y = mem.read8(addr as u16);
        self.set_zn_flags(self.regs.y);
        self.cycles += 4;
    }    

    // Load accumulator from direct page
    pub fn inst_lda_dp(&mut self, mem: &mut Memory) {
        let offset = mem.read8(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
    
        let addr = self.dp_base() | offset;
        self.regs.a = mem.read8(addr);
    
        self.set_zn_flags(self.regs.a);
        self.cycles += 3;
    }
    
    pub fn inst_ldx_dp(&mut self, mem: &mut Memory) {
        let offset = mem.read8(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
    
        let addr = self.dp_base() | offset;
        self.regs.x = mem.read8(addr);
    
        self.set_zn_flags(self.regs.x);
        self.cycles += 3;
    }
    
    pub fn inst_ldy_dp(&mut self, mem: &mut Memory) {
        let offset = mem.read8(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
    
        let addr = self.dp_base() | offset;
        self.regs.y = mem.read8(addr);
    
        self.set_zn_flags(self.regs.y);
        self.cycles += 3;
    }
    
    pub fn inst_sta_dp(&mut self, mem: &mut Memory) {
        let offset = mem.read8(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
    
        let addr = self.dp_base() | offset;
        mem.write8(addr, self.regs.a);
    
        self.cycles += 3;
    }
    
    pub fn inst_stx_dp(&mut self, mem: &mut Memory) {
        let offset = mem.read8(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
    
        let addr = self.dp_base() | offset;
        mem.write8(addr, self.regs.x);
    
        self.cycles += 3;
    }
    
    pub fn inst_sty_dp(&mut self, mem: &mut Memory) {
        let offset = mem.read8(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
    
        let addr = self.dp_base() | offset;
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
