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
        let opcode = mem.read8_mut(self.regs.pc);
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
        
            // Branches
            0x2F => self.inst_bra(mem), // BRA rel
            0xF0 => self.inst_beq(mem), // BEQ rel
            0xD0 => self.inst_bne(mem), // BNE rel
            0x10 => self.inst_bpl(mem), // BPL rel
            0x30 => self.inst_bmi(mem), // BMI rel
            0x50 => self.inst_bvc(mem), // BVC rel
            0x70 => self.inst_bvs(mem), // BVS rel
            0x90 => self.inst_bcc(mem), // BCC rel
            0xB0 => self.inst_bcs(mem), // BCS rel

            // Subroutine calls and returns
            0x3F => self.inst_call(mem), // CALL !abs
            0x6F => self.inst_ret(mem),  // RET
            0x4F => self.inst_pcall(mem), // PCALL !abs
            0x01 => self.inst_tcall(mem, 0),
            0x11 => self.inst_tcall(mem, 1),
            0x21 => self.inst_tcall(mem, 2),
            0x31 => self.inst_tcall(mem, 3),
            0x41 => self.inst_tcall(mem, 4),
            0x51 => self.inst_tcall(mem, 5),
            0x61 => self.inst_tcall(mem, 6),
            0x71 => self.inst_tcall(mem, 7),
            0x81 => self.inst_tcall(mem, 8),
            0x91 => self.inst_tcall(mem, 9),
            0xA1 => self.inst_tcall(mem, 10),
            0xB1 => self.inst_tcall(mem, 11),
            0xC1 => self.inst_tcall(mem, 12),
            0xD1 => self.inst_tcall(mem, 13),
            0xE1 => self.inst_tcall(mem, 14),
            0xF1 => self.inst_tcall(mem, 15),

            // Stack operations
            0x2D => self.inst_push_a(mem), // PUSH A
            0xAE => self.inst_pop_a(mem),  // POP A
            0x4D => self.inst_push_x(mem), // PUSH X
            0xCE => self.inst_pop_x(mem),  // POP X
            0x6D => self.inst_push_y(mem), // PUSH Y
            0xEE => self.inst_pop_y(mem),  // POP Y
            0x0D => self.inst_push_psw(mem), // PUSH PSW
            0x8E => self.inst_pop_psw(mem),  // POP PSW
            0xEF => self.inst_sleep(), // SLEEP
            0xFF => self.inst_stop(),  // STOP

            // increment/decrement
            0xBC => self.inst_inc_a(), // INC A
            0x9C => self.inst_dec_a(), // DEC A
            0x3D => self.inst_inc_x(), // INC X
            0x1D => self.inst_dec_x(), // DEC X
            0xFC => self.inst_inc_y(), // INC Y
            0xDC => self.inst_dec_y(), // DEC Y
            0xAB => self.inst_inc_dp(mem), // INC dp
            0x8B => self.inst_dec_dp(mem), // DEC dp
            0xAC => self.inst_inc_abs(mem),
            0x8C => self.inst_dec_abs(mem),

            0xCF => self.inst_mul(mem), // MUL YA
            0x9E => self.inst_div(mem), // DIV

            // Shift operations
            0x1C => self.inst_asl_a(), // ASL A,
            0x0B => self.inst_asl_dp(mem), // ASL dp
            0x1B => self.inst_asl_dp_x(mem), // ASL dp X
            0x0C => self.inst_asl_abs(mem), // ASL !abs
            0x5C => self.inst_lsr_a(), // LSR A
            0x4B => self.inst_lsr_dp(mem), // LSR dp
            0x5B => self.inst_lsr_dp_x(mem), // LSR dp X
            0x4C => self.inst_lsr_abs(mem), // LSR !abs

            // Rotate operations
            0x3C => self.inst_rol_a(), // ROL A
            0x2B => self.inst_rol_dp(mem), // ROL dp
            0x3B => self.inst_rol_dp_x(mem), // ROL dp X
            0x2C => self.inst_rol_abs(mem), // ROL !abs
            0x7C => self.inst_ror_a(), // ROR A
            0x6B => self.inst_ror_dp(mem), // ROR dp
            0x7B => self.inst_ror_dp_x(mem), // ROR dp X
            0x6C => self.inst_ror_abs(mem), // ROR !abs

            // Flag operations
            0x60 => self.inst_clrc(), // CLRC
            0x80 => self.inst_setc(), // SETC
            0xED => self.inst_notc(), // NOTC
            0x20 => self.inst_clrp(), // CLRP
            0x40 => self.inst_setp(), // SETP
            0xE0 => self.inst_clrv(), // CLRV
            0xA0 => self.inst_ei(), // EI
            0xC0 => self.inst_di(), // DI
            0xDF => self.inst_daa(), // DAA
            0xBE => self.inst_das(), // DAS
 
            // MOV — register indirect
            0xE6 => self.inst_mov_a_ix(mem),   // MOV A,(X)
            0xC6 => self.inst_mov_ix_a(mem),   // MOV (X),A

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

    /// Read the next byte from memory at PC and advance PC by 1.
    ///
    /// Uses `read8_mut` so that reads of `$FD–$FF` (timer counters)
    /// correctly clear the counter, matching hardware behaviour.
    fn read_immediate(&mut self, mem: &mut Memory) -> u8 {
        let value = mem.read8_mut(self.regs.pc);
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

    pub fn inst_lda_imm(&mut self, mem: &mut Memory) {
        self.regs.a = self.read_immediate(mem);
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    pub fn inst_ldx_imm(&mut self, mem: &mut Memory) {
        self.regs.x = self.read_immediate(mem);
        self.set_zn_flags(self.regs.x);
        self.cycles += 2;
    }

    pub fn inst_ldy_imm(&mut self, mem: &mut Memory) {
        self.regs.y = self.read_immediate(mem);
        self.set_zn_flags(self.regs.y);
        self.cycles += 2;
    }

    pub fn inst_sta_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        mem.write8(addr, self.regs.a);
        self.cycles += 4;
    }

    pub fn inst_stx_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        mem.write8(addr, self.regs.x);
        self.cycles += 4;
    }

    pub fn inst_sty_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        mem.write8(addr, self.regs.y);
        self.cycles += 4;
    }

    pub fn inst_lda_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        self.regs.a = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 4;
    }

    pub fn inst_ldx_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        self.regs.x = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.x);
        self.cycles += 4;
    }

    pub fn inst_ldy_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        self.regs.y = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.y);
        self.cycles += 4;
    }    

    // Load from direct page
    pub fn inst_lda_dp(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | offset;
        self.regs.a = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 3;
    }

    pub fn inst_ldx_dp(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | offset;
        self.regs.x = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.x);
        self.cycles += 3;
    }

    pub fn inst_ldy_dp(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | offset;
        self.regs.y = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.y);
        self.cycles += 3;
    }
    
    pub fn inst_sta_dp(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | offset;
        mem.write8(addr, self.regs.a);
    
        self.cycles += 3;
    }
    
    pub fn inst_stx_dp(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | offset;
        mem.write8(addr, self.regs.x);
    
        self.cycles += 3;
    }
    
    pub fn inst_sty_dp(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | offset;
        mem.write8(addr, self.regs.y);
    
        self.cycles += 3;
    }    

    pub fn inst_adc_imm(&mut self, mem: &mut Memory) {
        let value = self.read_immediate(mem);

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
        let value = self.read_immediate(mem);

        let result = self.regs.a.wrapping_sub(value);

        self.set_flag(FLAG_C, self.regs.a >= value);
        self.set_zn_flags(result);

        self.cycles += 2;
    }

    pub fn inst_sbc_imm(&mut self, mem: &mut Memory) {
        let value = self.read_immediate(mem);

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
        self.regs.a &= self.read_immediate(mem);
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    /// Bitwise OR with accumulator
    pub fn inst_ora_imm(&mut self, mem: &mut Memory) {
        self.regs.a |= self.read_immediate(mem);
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    /// Bitwise XOR with accumulator
    pub fn inst_eor_imm(&mut self, mem: &mut Memory) {
        self.regs.a ^= self.read_immediate(mem);
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    /// BRA rel — branch always.
    /// Reads a signed 8-bit offset relative to the instruction after BRA
    /// Always taken. 4 cycles.
    fn inst_bra(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as i8;
        self.regs.pc = self.regs.pc.wrapping_add(offset as u16);
        self.cycles += 4;
    }

    /// BEQ rel — branch if Zero flag is set (last result was zero).
    /// 4 cycles if taken, 2 cycles if not taken.
    fn inst_beq(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as i8;
        if self.get_flag(FLAG_Z) {
            self.regs.pc = self.regs.pc.wrapping_add(offset as u16);
            self.cycles += 4;
        } else {
            self.cycles += 2;
        }
    }

    /// BNE rel — branch if Zero flag is clear.
    /// 4 cycles if taken, 2 cycles if not taken.
    fn inst_bne(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as i8;
        if !self.get_flag(FLAG_Z) {
            self.regs.pc = self.regs.pc.wrapping_add(offset as u16);
            self.cycles += 4;
        } else {
            self.cycles += 2;
        }
    }

    /// BPL rel — branch if Negative flag is clear
    /// 4 cycles if taken, 2 cycles if not taken.
    fn inst_bpl(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as i8;
        if !self.get_flag(FLAG_N) {
            self.regs.pc = self.regs.pc.wrapping_add(offset as u16);
            self.cycles += 4;
        } else {
            self.cycles += 2;
        }
    }

    /// BMI rel — branch if Negative flag is set
    /// 4 cycles if taken, 2 cycles if not taken.
    fn inst_bmi(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as i8;
        if self.get_flag(FLAG_N) {
            self.regs.pc = self.regs.pc.wrapping_add(offset as u16);
            self.cycles += 4;
        } else {
            self.cycles += 2;
        }
    }

    /// BVC rel — branch if Overflow flag is clear.
    /// 4 cycles if taken, 2 cycles if not taken.
    fn inst_bvc(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as i8;
        if !self.get_flag(FLAG_V) {
            self.regs.pc = self.regs.pc.wrapping_add(offset as u16);
            self.cycles += 4;
        } else {
            self.cycles += 2;
        }
    }

    /// BVS rel — branch if Overflow flag is set.
    /// 4 cycles if taken, 2 cycles if not taken.
    fn inst_bvs(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as i8;
        if self.get_flag(FLAG_V) {
            self.regs.pc = self.regs.pc.wrapping_add(offset as u16);
            self.cycles += 4;
        } else {
            self.cycles += 2;
        }
    }

    /// BCC rel — branch if Carry flag is clear.
    /// 4 cycles if taken, 2 cycles if not taken.
    fn inst_bcc(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as i8;
        if !self.get_flag(FLAG_C) {
            self.regs.pc = self.regs.pc.wrapping_add(offset as u16);
            self.cycles += 4;
        } else {
            self.cycles += 2;
        }
    }

    /// BCS rel — branch if Carry flag is set.
    /// 4 cycles if taken, 2 cycles if not taken.
    fn inst_bcs(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as i8;
        if self.get_flag(FLAG_C) {
            self.regs.pc = self.regs.pc.wrapping_add(offset as u16);
            self.cycles += 4;
        } else {
            self.cycles += 2;
        }
    }

    // =========================================================
    // STACK HELPERS
    //
    // The SPC700 stack lives at $0100–$01FF regardless of FLAG_P.
    // SP points to the next free slot.
    // PUSH: write to $0100|SP, then decrement SP.
    // POP:  increment SP, then read from $0100|SP.
    // =========================================================
 
    fn stack_push(&mut self, mem: &mut Memory, value: u8) {
        mem.write8(0x0100 | self.regs.sp as u16, value);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
    }
 
    fn stack_pop(&mut self, mem: &mut Memory) -> u8 {
        self.regs.sp = self.regs.sp.wrapping_add(1);
        mem.read8_mut(0x0100 | self.regs.sp as u16)
    }
 
    // =========================================================
    // SUBROUTINE CALLS AND RETURNS
    // =========================================================
 
    /// CALL !abs — push PC (high then low), jump to 16-bit target.
    /// 3 bytes: opcode + 16-bit target address. 8 cycles.
    fn inst_call(&mut self, mem: &mut Memory) {
        let target = self.read_immediate16(mem);
        // Push return address: high byte first, then low byte.
        // RET will pop low then high to reconstruct PC correctly.
        let pc_hi = (self.regs.pc >> 8) as u8;
        let pc_lo = (self.regs.pc & 0xFF) as u8;
        self.stack_push(mem, pc_hi);
        self.stack_push(mem, pc_lo);
        self.regs.pc = target;
        self.cycles += 8;
    }

    /// RET — pop return address (low then high) and jump to it.
    ///
    /// 1 byte. 5 cycles.
    fn inst_ret(&mut self, mem: &mut Memory) {
        let lo = self.stack_pop(mem) as u16;
        let hi = self.stack_pop(mem) as u16;
        self.regs.pc = (hi << 8) | lo;
        self.cycles += 5;
    }

     /// PCALL u — push return address, jump to $FF00 + u.
    /// Used to call routines in the SPC700 high page without a full
    /// 16-bit address. 2 bytes: opcode + u. 6 cycles.
    fn inst_pcall(&mut self, mem: &mut Memory) {
        let u = self.read_immediate(mem);
        let target = 0xFF00 | u as u16;
        let pc_hi = (self.regs.pc >> 8) as u8;
        let pc_lo = (self.regs.pc & 0xFF) as u8;
        self.stack_push(mem, pc_hi);
        self.stack_push(mem, pc_lo);
        self.regs.pc = target;
        self.cycles += 6;
    }

    /// TCALL n — call via vector table entry at $FFDE - (n * 2).
    /// Reads the 16-bit target address from the table (little-endian),
    /// pushes the return address, and jumps to the target.
    /// 16 variants: n = 0–15, opcode = (n << 4) | 0x01.
    /// 1 byte. 8 cycles.
    fn inst_tcall(&mut self, mem: &mut Memory, n: u8) {
        let vector_addr = 0xFFDE_u16.wrapping_sub(n as u16 * 2);
        let lo = mem.read8_mut(vector_addr)     as u16;
        let hi = mem.read8_mut(vector_addr + 1) as u16;
        let target = (hi << 8) | lo;
        let pc_hi = (self.regs.pc >> 8) as u8;
        let pc_lo = (self.regs.pc & 0xFF) as u8;
        self.stack_push(mem, pc_hi);
        self.stack_push(mem, pc_lo);
        self.regs.pc = target;
        self.cycles += 8;
    }

    // =========================================================
    // STACK — PUSH / POP
    //
    // PUSH: write register to $0100|SP, decrement SP. 4 cycles.
    // POP:  increment SP, read from $0100|SP into register. 4 cycles.
    // Neither instruction affects any flags.
    // =========================================================
 
    /// PUSH A — push accumulator onto the stack. 4 cycles.
    fn inst_push_a(&mut self, mem: &mut Memory) {
        self.stack_push(mem, self.regs.a);
        self.cycles += 4;
    }

    /// POP A — pop accumulator from the stack. 4 cycles.
    fn inst_pop_a(&mut self, mem: &mut Memory) {
        self.regs.a = self.stack_pop(mem);
        self.cycles += 4;
    }

    /// PUSH X — push X register onto the stack. 4 cycles.
    fn inst_push_x(&mut self, mem: &mut Memory) {
        self.stack_push(mem, self.regs.x);
        self.cycles += 4;
    }

    /// POP X — pop X register from the stack. 4 cycles.
    fn inst_pop_x(&mut self, mem: &mut Memory) {
        self.regs.x = self.stack_pop(mem);
        self.cycles += 4;
    }

    /// PUSH Y — push Y register onto the stack. 4 cycles.
    fn inst_push_y(&mut self, mem: &mut Memory) {
        self.stack_push(mem, self.regs.y);
        self.cycles += 4;
    }

    /// POP Y — pop Y register from the stack. 4 cycles.
    fn inst_pop_y(&mut self, mem: &mut Memory) {
        self.regs.y = self.stack_pop(mem);
        self.cycles += 4;
    }

    /// PUSH PSW — push processor status word onto the stack. 4 cycles.
    fn inst_push_psw(&mut self, mem: &mut Memory) {
        self.stack_push(mem, self.regs.psw);
        self.cycles += 4;
    }

    /// POP PSW — pop processor status word from the stack. 4 cycles.
    /// Unlike POP A/X/Y, this restores all flags simultaneously.
    fn inst_pop_psw(&mut self, mem: &mut Memory) {
        self.regs.psw = self.stack_pop(mem);
        self.cycles += 4;
    }

    /// SLEEP — halt CPU until an interrupt fires.
    /// TODO: implement when interrupt handling is added (feature/ipl-boot-rom).
    fn inst_sleep(&mut self) {
        todo!("SLEEP: halt until interrupt")
    }

    /// STOP — halt CPU permanently.
    /// TODO: implement when interrupt handling is added (feature/ipl-boot-rom).
    fn inst_stop(&mut self) {
        todo!("STOP: permanent halt")
    }

    /// INC A — increment accumulator by 1. Sets N and Z. 2 cycles.
    fn inst_inc_a(&mut self) {
        self.regs.a = self.regs.a.wrapping_add(1);
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    /// DEC A — decrement accumulator by 1. Sets N and Z. 2 cycles.
    fn inst_dec_a(&mut self) {
        self.regs.a = self.regs.a.wrapping_sub(1);
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    /// INC X — increment X by 1. Sets N and Z. 2 cycles.
    fn inst_inc_x(&mut self) {
        self.regs.x = self.regs.x.wrapping_add(1);
        self.set_zn_flags(self.regs.x);
        self.cycles += 2;
    }

    /// DEC X — decrement X by 1. Sets N and Z. 2 cycles.
    fn inst_dec_x(&mut self) {
        self.regs.x = self.regs.x.wrapping_sub(1);
        self.set_zn_flags(self.regs.x);
        self.cycles += 2;
    }

    /// INC Y — increment Y by 1. Sets N and Z. 2 cycles.
    fn inst_inc_y(&mut self) {
        self.regs.y = self.regs.y.wrapping_add(1);
        self.set_zn_flags(self.regs.y);
        self.cycles += 2;
    }

    /// DEC Y — decrement Y by 1. Sets N and Z. 2 cycles.
    fn inst_dec_y(&mut self) {
        self.regs.y = self.regs.y.wrapping_sub(1);
        self.set_zn_flags(self.regs.y);
        self.cycles += 2;
    }

    /// INC dp — increment byte at direct page address. Sets N and Z. 5 cycles.
    fn inst_inc_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let val = mem.read8_mut(addr).wrapping_add(1);
        mem.write8(addr, val);
        self.set_zn_flags(val);
        self.cycles += 5;
    }

    /// DEC dp — decrement byte at direct page address. Sets N and Z. 5 cycles.
    fn inst_dec_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let val = mem.read8_mut(addr).wrapping_sub(1);
        mem.write8(addr, val);
        self.set_zn_flags(val);
        self.cycles += 5;
    }

    /// INC !abs — increment byte at absolute address. Sets N and Z. 6 cycles.
    fn inst_inc_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let val = mem.read8_mut(addr).wrapping_add(1);
        mem.write8(addr, val);
        self.set_zn_flags(val);
        self.cycles += 6;
    }

    /// DEC !abs — decrement byte at absolute address. Sets N and Z. 6 cycles.
    fn inst_dec_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let val = mem.read8_mut(addr).wrapping_sub(1);
        mem.write8(addr, val);
        self.set_zn_flags(val);
        self.cycles += 6;
    }

    /// MUL YA — unsigned 8×8 multiply: Y * A → YA.
    /// Y holds the high byte of the result, A the low byte.
    /// N and Z are set from Y (the high byte). 9 cycles.
    fn inst_mul(&mut self, _mem: &mut Memory) {
        let result = (self.regs.y as u16) * (self.regs.a as u16);
        self.regs.a = result as u8;
        self.regs.y = (result >> 8) as u8;
        self.set_zn_flags(self.regs.y);
        self.cycles += 9;
    }

    /// DIV YA,X — unsigned 16/8 divide: YA / X → A quotient, Y remainder.
    /// V is set if the quotient overflows (> $FF) or divisor is zero.
    /// H is set if (Y & $0F) >= (X & $0F).
    /// N and Z are set from the quotient (A). 12 cycles.
    fn inst_div(&mut self, _mem: &mut Memory) {
        let ya = ((self.regs.y as u16) << 8) | self.regs.a as u16;
        self.set_flag(FLAG_H, (self.regs.y & 0x0F) >= (self.regs.x & 0x0F));
        if self.regs.x == 0 {
            // Division by zero — quotient and remainder both $FF
            self.regs.a = 0xFF;
            self.regs.y = 0xFF;
            self.set_flag(FLAG_V, true);
        } else {
            let q = ya / self.regs.x as u16;
            let r = ya % self.regs.x as u16;
            self.set_flag(FLAG_V, q > 0xFF);
            self.regs.a = q as u8;
            self.regs.y = r as u8;
        }
        self.set_zn_flags(self.regs.a);
        self.cycles += 12;
    }

    /// ASL A — arithmetic shift left on accumulator.
    /// Bit 7 → C, 0 → bit 0. Sets N and Z from result. 2 cycles.
    fn inst_asl_a(&mut self) {
        self.set_flag(FLAG_C, (self.regs.a & 0x80) != 0);
        self.regs.a <<= 1;
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    /// ASL dp — arithmetic shift left on direct page byte.
    /// Bit 7 → C, 0 → bit 0. Sets N and Z from result. 5 cycles.
    fn inst_asl_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let val = mem.read8_mut(addr);
        self.set_flag(FLAG_C, (val & 0x80) != 0);
        let result = val << 1;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// ASL dp+X — arithmetic shift left on direct page + X byte.
    /// Bit 7 → C, 0 → bit 0. Sets N and Z from result. 5 cycles.
    fn inst_asl_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let val = mem.read8_mut(addr);
        self.set_flag(FLAG_C, (val & 0x80) != 0);
        let result = val << 1;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// ASL !abs — arithmetic shift left on absolute address byte.
    /// Bit 7 → C, 0 → bit 0. Sets N and Z from result. 6 cycles.
    fn inst_asl_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let val = mem.read8_mut(addr);
        self.set_flag(FLAG_C, (val & 0x80) != 0);
        let result = val << 1;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// LSR A — logical shift right on accumulator.
    /// Bit 0 → C, 0 → bit 7. Sets N and Z from result. 2 cycles.
    fn inst_lsr_a(&mut self) {
        self.set_flag(FLAG_C, (self.regs.a & 0x01) != 0);
        self.regs.a >>= 1;
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    /// LSR dp — logical shift right on direct page byte.
    /// Bit 0 → C, 0 → bit 7. Sets N and Z from result. 5 cycles.
    fn inst_lsr_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let val = mem.read8_mut(addr);
        self.set_flag(FLAG_C, (val & 0x01) != 0);
        let result = val >> 1;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// LSR dp+X — logical shift right on direct page + X byte.
    /// Bit 0 → C, 0 → bit 7. Sets N and Z from result. 5 cycles.
    fn inst_lsr_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let val = mem.read8_mut(addr);
        self.set_flag(FLAG_C, (val & 0x01) != 0);
        let result = val >> 1;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// LSR !abs — logical shift right on absolute address byte.
    /// Bit 0 → C, 0 → bit 7. Sets N and Z from result. 6 cycles.
    fn inst_lsr_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let val = mem.read8_mut(addr);
        self.set_flag(FLAG_C, (val & 0x01) != 0);
        let result = val >> 1;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// ROL A — rotate left through carry on accumulator.
    /// Old C → bit 0, bit 7 → new C. Sets N and Z from result. 2 cycles.
    fn inst_rol_a(&mut self) {
        let old_carry = self.get_flag(FLAG_C) as u8;
        self.set_flag(FLAG_C, (self.regs.a & 0x80) != 0);
        self.regs.a = (self.regs.a << 1) | old_carry;
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    /// ROL dp — rotate left through carry on direct page byte.
    /// Old C → bit 0, bit 7 → new C. Sets N and Z from result. 5 cycles.
    fn inst_rol_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let val = mem.read8_mut(addr);
        let old_carry = self.get_flag(FLAG_C) as u8;
        self.set_flag(FLAG_C, (val & 0x80) != 0);
        let result = (val << 1) | old_carry;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// ROL dp+X — rotate left through carry on direct page + X byte.
    /// Old C → bit 0, bit 7 → new C. Sets N and Z from result. 5 cycles.
    fn inst_rol_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let val = mem.read8_mut(addr);
        let old_carry = self.get_flag(FLAG_C) as u8;
        self.set_flag(FLAG_C, (val & 0x80) != 0);
        let result = (val << 1) | old_carry;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// ROL !abs — rotate left through carry on absolute address byte.
    /// Old C → bit 0, bit 7 → new C. Sets N and Z from result. 6 cycles.
    fn inst_rol_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let val = mem.read8_mut(addr);
        let old_carry = self.get_flag(FLAG_C) as u8;
        self.set_flag(FLAG_C, (val & 0x80) != 0);
        let result = (val << 1) | old_carry;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// ROR A — rotate right through carry on accumulator.
    /// Old C → bit 7, bit 0 → new C. Sets N and Z from result. 2 cycles.
    fn inst_ror_a(&mut self) {
        let old_carry = self.get_flag(FLAG_C) as u8;
        self.set_flag(FLAG_C, (self.regs.a & 0x01) != 0);
        self.regs.a = (self.regs.a >> 1) | (old_carry << 7);
        self.set_zn_flags(self.regs.a);
        self.cycles += 2;
    }

    /// ROR dp — rotate right through carry on direct page byte.
    /// Old C → bit 7, bit 0 → new C. Sets N and Z from result. 5 cycles.
    fn inst_ror_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let val = mem.read8_mut(addr);
        let old_carry = self.get_flag(FLAG_C) as u8;
        self.set_flag(FLAG_C, (val & 0x01) != 0);
        let result = (val >> 1) | (old_carry << 7);
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// ROR dp+X — rotate right through carry on direct page + X byte.
    /// Old C → bit 7, bit 0 → new C. Sets N and Z from result. 5 cycles.
    fn inst_ror_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let val = mem.read8_mut(addr);
        let old_carry = self.get_flag(FLAG_C) as u8;
        self.set_flag(FLAG_C, (val & 0x01) != 0);
        let result = (val >> 1) | (old_carry << 7);
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// ROR !abs — rotate right through carry on absolute address byte.
    /// Old C → bit 7, bit 0 → new C. Sets N and Z from result. 6 cycles.
    fn inst_ror_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let val = mem.read8_mut(addr);
        let old_carry = self.get_flag(FLAG_C) as u8;
        self.set_flag(FLAG_C, (val & 0x01) != 0);
        let result = (val >> 1) | (old_carry << 7);
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// CLRC — clear carry flag. 2 cycles.
    fn inst_clrc(&mut self) {
        self.set_flag(FLAG_C, false);
        self.cycles += 2;
    }

    /// SETC — set carry flag. 2 cycles.
    fn inst_setc(&mut self) {
        self.set_flag(FLAG_C, true);
        self.cycles += 2;
    }

    /// NOTC — complement carry flag (toggle C). 3 cycles.
    fn inst_notc(&mut self) {
        let c = !self.get_flag(FLAG_C);
        self.set_flag(FLAG_C, c);
        self.cycles += 3;
    }

    /// CLRP — clear direct page flag. Direct page base → $0000. 2 cycles.
    fn inst_clrp(&mut self) {
        self.set_flag(FLAG_P, false);
        self.cycles += 2;
    }

    /// SETP — set direct page flag. Direct page base → $0100. 2 cycles.
    fn inst_setp(&mut self) {
        self.set_flag(FLAG_P, true);
        self.cycles += 2;
    }

    /// CLRV — clear overflow (V) and half-carry (H) flags. 2 cycles.
    fn inst_clrv(&mut self) {
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_H, false);
        self.cycles += 2;
    }

    /// EI — enable interrupts (set FLAG_I). 3 cycles.
    fn inst_ei(&mut self) {
        self.set_flag(FLAG_I, true);
        self.cycles += 3;
    }

    /// DI — disable interrupts (clear FLAG_I). 3 cycles.
    fn inst_di(&mut self) {
        self.set_flag(FLAG_I, false);
        self.cycles += 3;
    }

    /// DAA — decimal adjust A after BCD addition.
    /// Adjusts A to a valid BCD value after ADC. Sets N, Z, and C. 3 cycles.
    fn inst_daa(&mut self) {
        let mut a = self.regs.a;
        if self.get_flag(FLAG_C) || a > 0x99 {
            a = a.wrapping_add(0x60);
            self.set_flag(FLAG_C, true);
        }
        if self.get_flag(FLAG_H) || (a & 0x0F) > 0x09 {
            a = a.wrapping_add(0x06);
        }
        self.regs.a = a;
        self.set_zn_flags(self.regs.a);
        self.cycles += 3;
    }

    /// DAS — decimal adjust A after BCD subtraction.
    /// Adjusts A to a valid BCD value after SBC. Sets N, Z, and C. 3 cycles.
    fn inst_das(&mut self) {
        let mut a = self.regs.a;
        if !self.get_flag(FLAG_C) || a > 0x99 {
            a = a.wrapping_sub(0x60);
            self.set_flag(FLAG_C, false);
        }
        if !self.get_flag(FLAG_H) || (a & 0x0F) > 0x09 {
            a = a.wrapping_sub(0x06);
        }
        self.regs.a = a;
        self.set_zn_flags(self.regs.a);
        self.cycles += 3;
    }

    /// MOV A,(X) — load A from address pointed to by X in direct page.
    /// Sets N and Z. 3 cycles.
    fn inst_mov_a_ix(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.regs.x as u16;
        self.regs.a = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 3;
    }

    /// MOV (X),A — store A to address pointed to by X in direct page.
    /// No flags affected. 4 cycles.
    fn inst_mov_ix_a(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.regs.x as u16;
        mem.write8(addr, self.regs.a);
        self.cycles += 4;
    }
}
