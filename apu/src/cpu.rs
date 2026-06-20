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
            0xBF => self.inst_mov_a_ixp(mem),  // MOV A,(X)+
            0xAF => self.inst_mov_ixp_a(mem),  // MOV (X)+,A
            0xF4 => self.inst_mov_a_dp_x(mem), // MOV A,dp+X
            0xF5 => self.inst_mov_a_abs_x(mem), // MOV A,!abs+X
            0xF6 => self.inst_mov_a_abs_y(mem), // MOV A,!abs+Y
            0xE7 => self.inst_mov_a_dp_x_ind(mem), // MOV A,[dp+X]
            0xF7 => self.inst_mov_a_dp_ind_y(mem), // MOV A,[dp]+Y
            0xD4 => self.inst_mov_dp_x_a(mem), // MOV dp+X,A
            0xD5 => self.inst_mov_abs_x_a(mem), // MOV !abs+X,A
            0xD6 => self.inst_mov_abs_y_a(mem), // MOV !abs+Y,A
            0xC7 => self.inst_mov_dp_x_ind_a(mem), // MOV [dp+X],A
            0xD7 => self.inst_mov_dp_ind_y_a(mem), // MOV [dp]+Y,A
            0xF9 => self.inst_mov_x_dp_y(mem), // MOV X,dp+Y
            0xD9 => self.inst_mov_dp_y_x(mem), // MOV dp+Y,X
            0xFB => self.inst_mov_y_dp_x(mem), // MOV Y,dp+X
            0xDB => self.inst_mov_dp_x_y(mem), // MOV dp+X,Y
            0xCB => self.inst_mov_dp_y(mem), // MOV dp,Y
            0xD8 => self.inst_mov_dp_x(mem), // MOV dp,X
            0x8F => self.inst_mov_dp_imm(mem), // MOV dp,#imm
            0xFA => self.inst_mov_dp_dp(mem),  // MOV dp,dp
            0x9D => self.inst_mov_x_sp(), // MOV X,SP
            0xBD => self.inst_mov_sp_x(), // MOV SP,X

            // 16 bit moves
            0xBA => self.inst_movw_ya_dp(mem), // MOVW YA,dp
            0xDA => self.inst_movw_dp_ya(mem), // MOVW dp,YA
            0x7A => self.inst_addw(mem),       // ADDW YA,dp
            0x9A => self.inst_subw(mem),       // SUBW YA,dp
            0x5A => self.inst_cmpw(mem),       // CMPW YA,dp
            0x1A => self.inst_decw(mem),       // DECW dp
            0x3A => self.inst_incw(mem),       // INCW dp
            0xBB => self.inst_inc_dp_x(mem), // INC dp+X
            0x9B => self.inst_dec_dp_x(mem), // DEC dp+X

            // OR — all addressing modes
            0x04 => self.inst_or_a_dp(mem),
            0x05 => self.inst_or_a_abs(mem),
            0x06 => self.inst_or_a_ix(mem),
            0x07 => self.inst_or_a_dp_x_ind(mem),
            0x09 => self.inst_or_dp_dp(mem),
            0x14 => self.inst_or_a_dp_x(mem),
            0x15 => self.inst_or_a_abs_x(mem),
            0x16 => self.inst_or_a_abs_y(mem),
            0x17 => self.inst_or_a_dp_ind_y(mem),
            0x18 => self.inst_or_dp_imm(mem),
            0x19 => self.inst_or_ix_iy(mem),

            // AND — all addressing modes
            0x24 => self.inst_and_a_dp(mem),
            0x25 => self.inst_and_a_abs(mem),
            0x26 => self.inst_and_a_ix(mem),
            0x27 => self.inst_and_a_dp_x_ind(mem),
            0x29 => self.inst_and_dp_dp(mem),
            0x34 => self.inst_and_a_dp_x(mem),
            0x35 => self.inst_and_a_abs_x(mem),
            0x36 => self.inst_and_a_abs_y(mem),
            0x37 => self.inst_and_a_dp_ind_y(mem),
            0x38 => self.inst_and_dp_imm(mem),
            0x39 => self.inst_and_ix_iy(mem),

            // EOR — all addressing modes
            0x44 => self.inst_eor_a_dp(mem),
            0x45 => self.inst_eor_a_abs(mem),
            0x46 => self.inst_eor_a_ix(mem),
            0x47 => self.inst_eor_a_dp_x_ind(mem),
            0x49 => self.inst_eor_dp_dp(mem),
            0x54 => self.inst_eor_a_dp_x(mem),
            0x55 => self.inst_eor_a_abs_x(mem),
            0x56 => self.inst_eor_a_abs_y(mem),
            0x57 => self.inst_eor_a_dp_ind_y(mem),
            0x58 => self.inst_eor_dp_imm(mem),
            0x59 => self.inst_eor_ix_iy(mem),

            // CMP — all addressing modes
            0x64 => self.inst_cmp_a_dp(mem),
            0x65 => self.inst_cmp_a_abs(mem),
            0x66 => self.inst_cmp_a_ix(mem),
            0x67 => self.inst_cmp_a_dp_x_ind(mem),
            0x69 => self.inst_cmp_dp_dp(mem),
            0x74 => self.inst_cmp_a_dp_x(mem),
            0x75 => self.inst_cmp_a_abs_x(mem),
            0x76 => self.inst_cmp_a_abs_y(mem),
            0x77 => self.inst_cmp_a_dp_ind_y(mem),
            0x78 => self.inst_cmp_dp_imm(mem),
            0x79 => self.inst_cmp_ix_iy(mem),
            0xC8 => self.inst_cmp_x_imm(mem),
            0xAD => self.inst_cmp_y_imm(mem),
            0x3E => self.inst_cmp_x_dp(mem),
            0x1E => self.inst_cmp_x_abs(mem),
            0x7E => self.inst_cmp_y_dp(mem),
            0x5E => self.inst_cmp_y_abs(mem),

            // ADC — all addressing modes
            0x84 => self.inst_adc_a_dp(mem),
            0x85 => self.inst_adc_a_abs(mem),
            0x86 => self.inst_adc_a_ix(mem),
            0x87 => self.inst_adc_a_dp_x_ind(mem),
            0x89 => self.inst_adc_dp_dp(mem),
            0x94 => self.inst_adc_a_dp_x(mem),
            0x95 => self.inst_adc_a_abs_x(mem),
            0x96 => self.inst_adc_a_abs_y(mem),
            0x97 => self.inst_adc_a_dp_ind_y(mem),
            0x98 => self.inst_adc_dp_imm(mem),
            0x99 => self.inst_adc_ix_iy(mem),

            // SBC — all addressing modes
            0xA4 => self.inst_sbc_a_dp(mem),
            0xA5 => self.inst_sbc_a_abs(mem),
            0xA6 => self.inst_sbc_a_ix(mem),
            0xA7 => self.inst_sbc_a_dp_x_ind(mem),
            0xA9 => self.inst_sbc_dp_dp(mem),
            0xB4 => self.inst_sbc_a_dp_x(mem),
            0xB5 => self.inst_sbc_a_abs_x(mem),
            0xB6 => self.inst_sbc_a_abs_y(mem),
            0xB7 => self.inst_sbc_a_dp_ind_y(mem),
            0xB8 => self.inst_sbc_dp_imm(mem),
            0xB9 => self.inst_sbc_ix_iy(mem),

            // SET1/CLR1 — bit set/clear, direct page, 8 positions each
            0x02 => self.inst_set1(mem, 0),
            0x12 => self.inst_clr1(mem, 0),
            0x22 => self.inst_set1(mem, 1),
            0x32 => self.inst_clr1(mem, 1),
            0x42 => self.inst_set1(mem, 2),
            0x52 => self.inst_clr1(mem, 2),
            0x62 => self.inst_set1(mem, 3),
            0x72 => self.inst_clr1(mem, 3),
            0x82 => self.inst_set1(mem, 4),
            0x92 => self.inst_clr1(mem, 4),
            0xA2 => self.inst_set1(mem, 5),
            0xB2 => self.inst_clr1(mem, 5),
            0xC2 => self.inst_set1(mem, 6),
            0xD2 => self.inst_clr1(mem, 6),
            0xE2 => self.inst_set1(mem, 7),
            0xF2 => self.inst_clr1(mem, 7),

            // BBS/BBC — branch on bit set/clear, 8 positions each
            0x03 => self.inst_bbs_bbc(mem, 0, true),  // BBS d.0
            0x13 => self.inst_bbs_bbc(mem, 0, false), // BBC d.0
            0x23 => self.inst_bbs_bbc(mem, 1, true),
            0x33 => self.inst_bbs_bbc(mem, 1, false),
            0x43 => self.inst_bbs_bbc(mem, 2, true),
            0x53 => self.inst_bbs_bbc(mem, 2, false),
            0x63 => self.inst_bbs_bbc(mem, 3, true),
            0x73 => self.inst_bbs_bbc(mem, 3, false),
            0x83 => self.inst_bbs_bbc(mem, 4, true),
            0x93 => self.inst_bbs_bbc(mem, 4, false),
            0xA3 => self.inst_bbs_bbc(mem, 5, true),
            0xB3 => self.inst_bbs_bbc(mem, 5, false),
            0xC3 => self.inst_bbs_bbc(mem, 6, true),
            0xD3 => self.inst_bbs_bbc(mem, 6, false),
            0xE3 => self.inst_bbs_bbc(mem, 7, true),
            0xF3 => self.inst_bbs_bbc(mem, 7, false),

            // Bit manipulation opcodes
            0x0E => self.inst_tset1(mem),       // TSET1 !a
            0x4E => self.inst_tclr1(mem),       // TCLR1 !a
            0xAA => self.inst_mov1_c_mb(mem),   // MOV1 C,m.b
            0xCA => self.inst_mov1_mb_c(mem),   // MOV1 m.b,C
            0x0A => self.inst_or1_c_mb(mem),    // OR1 C,m.b
            0x2A => self.inst_or1_c_not_mb(mem),// OR1 C,/m.b
            0x4A => self.inst_and1_c_mb(mem),   // AND1 C,m.b
            0x6A => self.inst_and1_c_not_mb(mem), // AND1 C,/m.b
            0x8A => self.inst_eor1_c_mb(mem),   // EOR1 C,m.b
            0xEA => self.inst_not1_mb(mem),     // NOT1 m.b

            // Jumps and interrupts
            0x5F => self.inst_jmp_abs(mem),        // JMP !a
            0x1F => self.inst_jmp_abs_x_ind(mem),  // JMP [!a+X]
            0x7F => self.inst_reti(mem),           // RETI
            0x2E => self.inst_cbne_dp(mem),        // CBNE dp,rel
            0xDE => self.inst_cbne_dp_x(mem),      // CBNE dp+X,rel
            0x6E => self.inst_dbnz_dp(mem),        // DBNZ dp,rel
            0xFE => self.inst_dbnz_y(mem),         // DBNZ Y,rel
            0x0F => self.inst_brk(mem),            // BRK

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
        self.regs.a = self.adc_flags(self.regs.a, value);
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
        self.regs.a = self.sbc_flags(self.regs.a, value);
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

    /// MOV A,(X)+ — load A from dp address in X, then increment X.
    /// Sets N and Z. 4 cycles.
    fn inst_mov_a_ixp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.regs.x as u16;
        self.regs.a = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.regs.x = self.regs.x.wrapping_add(1);
        self.cycles += 4;
    }

    /// MOV (X)+,A — store A to dp address in X, then increment X.
    /// No flags affected. 4 cycles.
    fn inst_mov_ixp_a(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.regs.x as u16;
        mem.write8(addr, self.regs.a);
        self.regs.x = self.regs.x.wrapping_add(1);
        self.cycles += 4;
    }

    /// MOV A,dp+X — load A from direct page address + X.
    /// Sets N and Z. 4 cycles.
    fn inst_mov_a_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        self.regs.a = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 4;
    }

    /// MOV A,!abs+X — load A from absolute address + X.
    /// Sets N and Z. 5 cycles.
    fn inst_mov_a_abs_x(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.x as u16);
        self.regs.a = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 5;
    }

    /// MOV A,!abs+Y — load A from absolute address + Y.
    /// Sets N and Z. 5 cycles.
    fn inst_mov_a_abs_y(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.y as u16);
        self.regs.a = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 5;
    }

    /// MOV A,[dp+X] — load A from address stored at dp+X (indexed indirect).
    /// Reads 16-bit pointer from dp+X, then loads A from that address.
    /// Sets N and Z. 6 cycles.
    fn inst_mov_a_dp_x_ind(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let addr = (hi << 8) | lo;
        self.regs.a = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 6;
    }

    /// MOV A,[dp]+Y — load A from address stored at dp, indexed by Y (indirect indexed).
    /// Reads 16-bit pointer from dp, adds Y, then loads A from that address.
    /// Sets N and Z. 6 cycles.
    fn inst_mov_a_dp_ind_y(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | offset;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let base = (hi << 8) | lo;
        let addr = base.wrapping_add(self.regs.y as u16);
        self.regs.a = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 6;
    }

    /// MOV dp+X,A — store A to direct page address + X.
    /// No flags affected. 5 cycles.
    fn inst_mov_dp_x_a(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        mem.write8(addr, self.regs.a);
        self.cycles += 5;
    }

    /// MOV !abs+X,A — store A to absolute address + X.
    /// No flags affected. 6 cycles.
    fn inst_mov_abs_x_a(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.x as u16);
        mem.write8(addr, self.regs.a);
        self.cycles += 6;
    }

    /// MOV !abs+Y,A — store A to absolute address + Y.
    /// No flags affected. 6 cycles.
    fn inst_mov_abs_y_a(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.y as u16);
        mem.write8(addr, self.regs.a);
        self.cycles += 6;
    }

    /// MOV [dp+X],A — store A to address stored at dp+X (indexed indirect).
    /// Reads 16-bit pointer from dp+X, then stores A at that address.
    /// No flags affected. 7 cycles.
    fn inst_mov_dp_x_ind_a(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let addr = (hi << 8) | lo;
        mem.write8(addr, self.regs.a);
        self.cycles += 7;
    }

    /// MOV [dp]+Y,A — store A to address stored at dp, indexed by Y (indirect indexed).
    /// Reads 16-bit pointer from dp, adds Y, then stores A at that address.
    /// No flags affected. 7 cycles.
    fn inst_mov_dp_ind_y_a(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | offset;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let base = (hi << 8) | lo;
        let addr = base.wrapping_add(self.regs.y as u16);
        mem.write8(addr, self.regs.a);
        self.cycles += 7;
    }

    /// MOV X,dp+Y — load X from direct page address + Y.
    /// Sets N and Z. 4 cycles.
    fn inst_mov_x_dp_y(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.y as u16) & 0xFF;
        self.regs.x = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.x);
        self.cycles += 4;
    }

    /// MOV dp+Y,X — store X to direct page address + Y.
    /// No flags affected. 5 cycles.
    fn inst_mov_dp_y_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.y as u16) & 0xFF;
        mem.write8(addr, self.regs.x);
        self.cycles += 5;
    }

    /// MOV Y,dp+X — load Y from direct page address + X.
    /// Sets N and Z. 4 cycles.
    fn inst_mov_y_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        self.regs.y = mem.read8_mut(addr);
        self.set_zn_flags(self.regs.y);
        self.cycles += 4;
    }

    /// MOV dp+X,Y — store Y to direct page address + X.
    /// No flags affected. 5 cycles.
    fn inst_mov_dp_x_y(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        mem.write8(addr, self.regs.y);
        self.cycles += 5;
    }

    /// MOV dp,Y — store Y to direct page address.
    /// No flags affected. 4 cycles.
    fn inst_mov_dp_y(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        mem.write8(addr, self.regs.y);
        self.cycles += 4;
    }

    /// MOV dp,X — store X to direct page address.
    /// No flags affected. 4 cycles.
    fn inst_mov_dp_x(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        mem.write8(addr, self.regs.x);
        self.cycles += 4;
    }

    /// MOV dp,#imm — write immediate byte directly to direct page address.
    /// No flags affected. 5 cycles.
    fn inst_mov_dp_imm(&mut self, mem: &mut Memory) {
        let imm = self.read_immediate(mem);
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        mem.write8(addr, imm);
        self.cycles += 5;
    }

    /// MOV dp,dp — copy a byte from one direct page address to another.
    /// Operand order: source offset first, destination offset second.
    /// No flags affected. 5 cycles.
    fn inst_mov_dp_dp(&mut self, mem: &mut Memory) {
        let src_off = self.read_immediate(mem) as u16;
        let dst_off = self.read_immediate(mem) as u16;
        let base = self.dp_base();
        let val = mem.read8_mut(base | src_off);
        mem.write8(base | dst_off, val);
        self.cycles += 5;
    }

    /// MOV X,SP — copy stack pointer into X.
    /// Sets N and Z. 2 cycles.
    fn inst_mov_x_sp(&mut self) {
        self.regs.x = self.regs.sp;
        self.set_zn_flags(self.regs.x);
        self.cycles += 2;
    }

    /// MOV SP,X — copy X into stack pointer.
    /// No flags affected. 2 cycles.
    fn inst_mov_sp_x(&mut self) {
        self.regs.sp = self.regs.x;
        self.cycles += 2;
    }

    /// MOVW YA,dp — load 16-bit word from direct page into YA.
    /// Low byte → A, high byte → Y. Sets N from bit 15, Z if the full
    /// 16-bit value is zero. 5 cycles.
    fn inst_movw_ya_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        self.regs.a = mem.read8_mut(addr);
        self.regs.y = mem.read8_mut(addr.wrapping_add(1));
        let ya = ((self.regs.y as u16) << 8) | self.regs.a as u16;
        self.set_flag(FLAG_Z, ya == 0);
        self.set_flag(FLAG_N, (ya & 0x8000) != 0);
        self.cycles += 5;
    }

    /// MOVW dp,YA — store YA as a 16-bit word to direct page.
    /// A → low byte at dp, Y → high byte at dp+1. No flags affected. 5 cycles.
    fn inst_movw_dp_ya(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        mem.write8(addr, self.regs.a);
        mem.write8(addr.wrapping_add(1), self.regs.y);
        self.cycles += 5;
    }

    /// ADDW YA,dp — 16-bit add: YA + word(dp) → YA.
    /// Sets C, V, H, N, Z from the 16-bit result. 5 cycles.
    fn inst_addw(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let lo = mem.read8_mut(addr) as u16;
        let hi = mem.read8_mut(addr.wrapping_add(1)) as u16;
        let operand = (hi << 8) | lo;

        let ya = ((self.regs.y as u16) << 8) | self.regs.a as u16;
        let result = ya as u32 + operand as u32;

        self.set_flag(FLAG_C, result > 0xFFFF);
        self.set_flag(
            FLAG_V,
            (!(ya ^ operand) & (ya ^ result as u16) & 0x8000) != 0,
        );
        self.set_flag(FLAG_H, ((ya & 0x0FFF) + (operand & 0x0FFF)) > 0x0FFF);

        let result_u16 = result as u16;
        self.regs.a = result_u16 as u8;
        self.regs.y = (result_u16 >> 8) as u8;
        self.set_flag(FLAG_Z, result_u16 == 0);
        self.set_flag(FLAG_N, (result_u16 & 0x8000) != 0);
        self.cycles += 5;
    }

    /// SUBW YA,dp — 16-bit subtract: YA - word(dp) → YA.
    /// Sets C, V, H, N, Z from the 16-bit result. 5 cycles.
    fn inst_subw(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let lo = mem.read8_mut(addr) as u16;
        let hi = mem.read8_mut(addr.wrapping_add(1)) as u16;
        let operand = (hi << 8) | lo;

        let ya = ((self.regs.y as u16) << 8) | self.regs.a as u16;
        let result = ya as i32 - operand as i32;

        self.set_flag(FLAG_C, result >= 0);
        self.set_flag(
            FLAG_V,
            ((ya ^ operand) & (ya ^ result as u16) & 0x8000) != 0,
        );
        self.set_flag(FLAG_H, (ya & 0x0FFF) < (operand & 0x0FFF));

        let result_u16 = result as u16;
        self.regs.a = result_u16 as u8;
        self.regs.y = (result_u16 >> 8) as u8;
        self.set_flag(FLAG_Z, result_u16 == 0);
        self.set_flag(FLAG_N, (result_u16 & 0x8000) != 0);
        self.cycles += 5;
    }

    /// CMPW YA,dp — 16-bit compare: YA - word(dp), flags only (no write-back).
    /// Sets C, N, Z from the 16-bit result. 4 cycles.
    fn inst_cmpw(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let lo = mem.read8_mut(addr) as u16;
        let hi = mem.read8_mut(addr.wrapping_add(1)) as u16;
        let operand = (hi << 8) | lo;

        let ya = ((self.regs.y as u16) << 8) | self.regs.a as u16;
        let result = ya.wrapping_sub(operand);

        self.set_flag(FLAG_C, ya >= operand);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_N, (result & 0x8000) != 0);
        self.cycles += 4;
    }

    /// DECW dp — 16-bit decrement: word(dp) - 1 → word(dp).
    /// Sets N and Z from the 16-bit result. 6 cycles.
    fn inst_decw(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let lo = mem.read8_mut(addr) as u16;
        let hi = mem.read8_mut(addr.wrapping_add(1)) as u16;
        let value = ((hi << 8) | lo).wrapping_sub(1);

        mem.write8(addr, value as u8);
        mem.write8(addr.wrapping_add(1), (value >> 8) as u8);

        self.set_flag(FLAG_Z, value == 0);
        self.set_flag(FLAG_N, (value & 0x8000) != 0);
        self.cycles += 6;
    }

    /// INCW dp — 16-bit increment: word(dp) + 1 → word(dp).
    /// Sets N and Z from the 16-bit result. 6 cycles.
    fn inst_incw(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let lo = mem.read8_mut(addr) as u16;
        let hi = mem.read8_mut(addr.wrapping_add(1)) as u16;
        let value = ((hi << 8) | lo).wrapping_add(1);

        mem.write8(addr, value as u8);
        mem.write8(addr.wrapping_add(1), (value >> 8) as u8);

        self.set_flag(FLAG_Z, value == 0);
        self.set_flag(FLAG_N, (value & 0x8000) != 0);
        self.cycles += 6;
    }

    /// INC dp+X — increment byte at direct page address + X.
    /// Sets N and Z. 5 cycles.
    fn inst_inc_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let val = mem.read8_mut(addr).wrapping_add(1);
        mem.write8(addr, val);
        self.set_zn_flags(val);
        self.cycles += 5;
    }

    /// DEC dp+X — decrement byte at direct page address + X.
    /// Sets N and Z. 5 cycles.
    fn inst_dec_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let val = mem.read8_mut(addr).wrapping_sub(1);
        mem.write8(addr, val);
        self.set_zn_flags(val);
        self.cycles += 5;
    }

    // =========================================================
    // Shared dp-ALU operand helpers
    //
    // Used by OR/AND/EOR/CMP/ADC/SBC across their dp,dp / dp,#imm /
    // (X),(Y) addressing modes. All return (write_addr, dst_value, src_value)
    // so each instruction just applies its own operation to (dst, src) and
    // writes the result to write_addr.
    // =========================================================

    /// `OP dd,ds` — note the byte order in the instruction stream is
    /// src-offset first, dst-offset second (opposite of the "dd,ds" mnemonic
    /// reading order). Same convention as MOV dp,dp.
    fn read_dp_dp(&mut self, mem: &mut Memory) -> (u16, u8, u8) {
        let src_off = self.read_immediate(mem) as u16;
        let dst_off = self.read_immediate(mem) as u16;
        let base = self.dp_base();
        let src = mem.read8_mut(base | src_off);
        let dst = mem.read8_mut(base | dst_off);
        (base | dst_off, dst, src)
    }

    /// `OP dp,#imm` — immediate byte first, then the dp offset.
    fn read_dp_imm(&mut self, mem: &mut Memory) -> (u16, u8, u8) {
        let imm = self.read_immediate(mem);
        let dst_off = self.read_immediate(mem) as u16;
        let base = self.dp_base();
        let dst = mem.read8_mut(base | dst_off);
        (base | dst_off, dst, imm)
    }

    /// `OP (X),(Y)` — X supplies the destination address, Y the source.
    fn read_ix_iy(&mut self, mem: &mut Memory) -> (u16, u8, u8) {
        let base = self.dp_base();
        let x_addr = base | self.regs.x as u16;
        let y_addr = base | self.regs.y as u16;
        let x_val = mem.read8_mut(x_addr);
        let y_val = mem.read8_mut(y_addr);
        (x_addr, x_val, y_val)
    }

    fn inst_or_a_dp(&mut self, mem: &mut Memory) {
    let addr = self.dp_base() | self.read_immediate(mem) as u16;
    self.regs.a |= mem.read8_mut(addr);
    self.set_zn_flags(self.regs.a);
    self.cycles += 3;
    }

    fn inst_or_a_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        self.regs.a |= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 4;
    }

    fn inst_or_a_ix(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.regs.x as u16;
        self.regs.a |= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 3;
    }

    fn inst_or_a_dp_x_ind(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let addr = (hi << 8) | lo;
        self.regs.a |= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 6;
    }

    fn inst_or_dp_dp(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_dp_dp(mem);
        let result = dst | src;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 6;
    }

    fn inst_or_a_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        self.regs.a |= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 4;
    }

    fn inst_or_a_abs_x(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.x as u16);
        self.regs.a |= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 5;
    }

    fn inst_or_a_abs_y(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.y as u16);
        self.regs.a |= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 5;
    }

    fn inst_or_a_dp_ind_y(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | offset;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let base = (hi << 8) | lo;
        let addr = base.wrapping_add(self.regs.y as u16);
        self.regs.a |= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 6;
    }

    fn inst_or_dp_imm(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_dp_imm(mem);
        let result = dst | src;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    fn inst_or_ix_iy(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_ix_iy(mem);
        let result = dst | src;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    fn inst_and_a_dp(&mut self, mem: &mut Memory) {
    let addr = self.dp_base() | self.read_immediate(mem) as u16;
    self.regs.a &= mem.read8_mut(addr);
    self.set_zn_flags(self.regs.a);
    self.cycles += 3;
    }

    fn inst_and_a_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        self.regs.a &= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 4;
    }

    fn inst_and_a_ix(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.regs.x as u16;
        self.regs.a &= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 3;
    }

    fn inst_and_a_dp_x_ind(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let addr = (hi << 8) | lo;
        self.regs.a &= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 6;
    }

    fn inst_and_dp_dp(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_dp_dp(mem);
        let result = dst & src;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 6;
    }

    fn inst_and_a_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        self.regs.a &= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 4;
    }

    fn inst_and_a_abs_x(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.x as u16);
        self.regs.a &= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 5;
    }

    fn inst_and_a_abs_y(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.y as u16);
        self.regs.a &= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 5;
    }

    fn inst_and_a_dp_ind_y(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | offset;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let base = (hi << 8) | lo;
        let addr = base.wrapping_add(self.regs.y as u16);
        self.regs.a &= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 6;
    }

    fn inst_and_dp_imm(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_dp_imm(mem);
        let result = dst & src;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    fn inst_and_ix_iy(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_ix_iy(mem);
        let result = dst & src;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
        }

        fn inst_eor_a_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        self.regs.a ^= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 3;
    }

    fn inst_eor_a_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        self.regs.a ^= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 4;
    }

    fn inst_eor_a_ix(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.regs.x as u16;
        self.regs.a ^= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 3;
    }

    fn inst_eor_a_dp_x_ind(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let addr = (hi << 8) | lo;
        self.regs.a ^= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 6;
    }

    fn inst_eor_dp_dp(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_dp_dp(mem);
        let result = dst ^ src;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 6;
    }

    fn inst_eor_a_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        self.regs.a ^= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 4;
    }

    fn inst_eor_a_abs_x(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.x as u16);
        self.regs.a ^= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 5;
    }

    fn inst_eor_a_abs_y(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.y as u16);
        self.regs.a ^= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 5;
    }

    fn inst_eor_a_dp_ind_y(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | offset;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let base = (hi << 8) | lo;
        let addr = base.wrapping_add(self.regs.y as u16);
        self.regs.a ^= mem.read8_mut(addr);
        self.set_zn_flags(self.regs.a);
        self.cycles += 6;
    }

    fn inst_eor_dp_imm(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_dp_imm(mem);
        let result = dst ^ src;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    fn inst_eor_ix_iy(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_ix_iy(mem);
        let result = dst ^ src;
        mem.write8(addr, result);
        self.set_zn_flags(result);
        self.cycles += 5;
    }

    /// Shared CMP result handler — sets C (no borrow), Z, N. Never writes back.
    fn cmp_flags(&mut self, dst: u8, src: u8) {
        let result = dst.wrapping_sub(src);
        self.set_flag(FLAG_C, dst >= src);
        self.set_zn_flags(result);
    }

    fn inst_cmp_a_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let val = mem.read8_mut(addr);
        self.cmp_flags(self.regs.a, val);
        self.cycles += 3;
    }

    fn inst_cmp_a_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let val = mem.read8_mut(addr);
        self.cmp_flags(self.regs.a, val);
        self.cycles += 4;
    }

    fn inst_cmp_a_ix(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.regs.x as u16;
        let val = mem.read8_mut(addr);
        self.cmp_flags(self.regs.a, val);
        self.cycles += 3;
    }

    fn inst_cmp_a_dp_x_ind(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let addr = (hi << 8) | lo;
        let val = mem.read8_mut(addr);
        self.cmp_flags(self.regs.a, val);
        self.cycles += 6;
    }

    fn inst_cmp_dp_dp(&mut self, mem: &mut Memory) {
        let (_addr, dst, src) = self.read_dp_dp(mem);
        self.cmp_flags(dst, src);
        self.cycles += 6;
    }

    fn inst_cmp_a_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let val = mem.read8_mut(addr);
        self.cmp_flags(self.regs.a, val);
        self.cycles += 4;
    }

    fn inst_cmp_a_abs_x(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.x as u16);
        let val = mem.read8_mut(addr);
        self.cmp_flags(self.regs.a, val);
        self.cycles += 5;
    }

    fn inst_cmp_a_abs_y(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.y as u16);
        let val = mem.read8_mut(addr);
        self.cmp_flags(self.regs.a, val);
        self.cycles += 5;
    }

    fn inst_cmp_a_dp_ind_y(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | offset;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let base = (hi << 8) | lo;
        let addr = base.wrapping_add(self.regs.y as u16);
        let val = mem.read8_mut(addr);
        self.cmp_flags(self.regs.a, val);
        self.cycles += 6;
    }

    fn inst_cmp_dp_imm(&mut self, mem: &mut Memory) {
        let (_addr, dst, src) = self.read_dp_imm(mem);
        self.cmp_flags(dst, src);
        self.cycles += 5;
    }

    fn inst_cmp_ix_iy(&mut self, mem: &mut Memory) {
        let (_addr, dst, src) = self.read_ix_iy(mem);
        self.cmp_flags(dst, src);
        self.cycles += 5;
    }

    fn inst_cmp_x_imm(&mut self, mem: &mut Memory) {
        let val = self.read_immediate(mem);
        self.cmp_flags(self.regs.x, val);
        self.cycles += 2;
    }

    fn inst_cmp_y_imm(&mut self, mem: &mut Memory) {
        let val = self.read_immediate(mem);
        self.cmp_flags(self.regs.y, val);
        self.cycles += 2;
    }

    fn inst_cmp_x_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let val = mem.read8_mut(addr);
        self.cmp_flags(self.regs.x, val);
        self.cycles += 3;
    }

    fn inst_cmp_x_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let val = mem.read8_mut(addr);
        self.cmp_flags(self.regs.x, val);
        self.cycles += 4;
    }

    fn inst_cmp_y_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let val = mem.read8_mut(addr);
        self.cmp_flags(self.regs.y, val);
        self.cycles += 3;
    }

    fn inst_cmp_y_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let val = mem.read8_mut(addr);
        self.cmp_flags(self.regs.y, val);
        self.cycles += 4;
    }

    /// Shared ADC computation — adds src + dst + carry-in, sets C, V, H, Z, N.
    /// Returns the result byte for the caller to write back.
    fn adc_flags(&mut self, dst: u8, src: u8) -> u8 {
        let carry_in = if self.get_flag(FLAG_C) { 1u16 } else { 0u16 };
        let result = dst as u16 + src as u16 + carry_in;
        let result_u8 = result as u8;

        self.set_flag(FLAG_C, result > 0xFF);
        self.set_zn_flags(result_u8);
        self.set_flag(FLAG_V, (!(dst ^ src) & (dst ^ result_u8) & 0x80) != 0);
        // Half-carry: carry out of bit 3 into bit 4, including carry-in.
        self.set_flag(FLAG_H, ((dst & 0x0F) + (src & 0x0F) + carry_in as u8) > 0x0F);

        result_u8
    }

    fn inst_adc_a_dp(&mut self, mem: &mut Memory) {
    let addr = self.dp_base() | self.read_immediate(mem) as u16;
    let val = mem.read8_mut(addr);
    self.regs.a = self.adc_flags(self.regs.a, val);
    self.cycles += 3;
    }

    fn inst_adc_a_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let val = mem.read8_mut(addr);
        self.regs.a = self.adc_flags(self.regs.a, val);
        self.cycles += 4;
    }

    fn inst_adc_a_ix(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.regs.x as u16;
        let val = mem.read8_mut(addr);
        self.regs.a = self.adc_flags(self.regs.a, val);
        self.cycles += 3;
    }

    fn inst_adc_a_dp_x_ind(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let addr = (hi << 8) | lo;
        let val = mem.read8_mut(addr);
        self.regs.a = self.adc_flags(self.regs.a, val);
        self.cycles += 6;
    }

    fn inst_adc_dp_dp(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_dp_dp(mem);
        let result = self.adc_flags(dst, src);
        mem.write8(addr, result);
        self.cycles += 6;
    }

    fn inst_adc_a_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let val = mem.read8_mut(addr);
        self.regs.a = self.adc_flags(self.regs.a, val);
        self.cycles += 4;
    }

    fn inst_adc_a_abs_x(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.x as u16);
        let val = mem.read8_mut(addr);
        self.regs.a = self.adc_flags(self.regs.a, val);
        self.cycles += 5;
    }

    fn inst_adc_a_abs_y(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.y as u16);
        let val = mem.read8_mut(addr);
        self.regs.a = self.adc_flags(self.regs.a, val);
        self.cycles += 5;
    }

    fn inst_adc_a_dp_ind_y(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | offset;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let base = (hi << 8) | lo;
        let addr = base.wrapping_add(self.regs.y as u16);
        let val = mem.read8_mut(addr);
        self.regs.a = self.adc_flags(self.regs.a, val);
        self.cycles += 6;
    }

    fn inst_adc_dp_imm(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_dp_imm(mem);
        let result = self.adc_flags(dst, src);
        mem.write8(addr, result);
        self.cycles += 5;
    }

    fn inst_adc_ix_iy(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_ix_iy(mem);
        let result = self.adc_flags(dst, src);
        mem.write8(addr, result);
        self.cycles += 5;
    }

    /// Shared SBC computation — subtracts src + borrow-in from dst, sets
    /// C, V, H, Z, N. Returns the result byte for the caller to write back.
    fn sbc_flags(&mut self, dst: u8, src: u8) -> u8 {
        // SPC700 inverts carry semantics for subtraction: C=1 means "no borrow".
        let borrow_in = if self.get_flag(FLAG_C) { 0u8 } else { 1u8 };
        let result = dst as i16 - src as i16 - borrow_in as i16;
        let result_u8 = result as u8;

        self.set_flag(FLAG_C, result >= 0);
        self.set_zn_flags(result_u8);
        self.set_flag(FLAG_V, ((dst ^ result_u8) & (dst ^ src) & 0x80) != 0);
        // Half-borrow: H=1 means no half-borrow occurred (mirrors C's polarity).
        self.set_flag(FLAG_H, (dst & 0x0F) >= (src & 0x0F) + borrow_in);

        result_u8
    }

    fn inst_sbc_a_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let val = mem.read8_mut(addr);
        self.regs.a = self.sbc_flags(self.regs.a, val);
        self.cycles += 3;
    }

    fn inst_sbc_a_abs(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let val = mem.read8_mut(addr);
        self.regs.a = self.sbc_flags(self.regs.a, val);
        self.cycles += 4;
    }

    fn inst_sbc_a_ix(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.regs.x as u16;
        let val = mem.read8_mut(addr);
        self.regs.a = self.sbc_flags(self.regs.a, val);
        self.cycles += 3;
    }

    fn inst_sbc_a_dp_x_ind(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let addr = (hi << 8) | lo;
        let val = mem.read8_mut(addr);
        self.regs.a = self.sbc_flags(self.regs.a, val);
        self.cycles += 6;
    }

    fn inst_sbc_dp_dp(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_dp_dp(mem);
        let result = self.sbc_flags(dst, src);
        mem.write8(addr, result);
        self.cycles += 6;
    }

    fn inst_sbc_a_dp_x(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset + self.regs.x as u16) & 0xFF;
        let val = mem.read8_mut(addr);
        self.regs.a = self.sbc_flags(self.regs.a, val);
        self.cycles += 4;
    }

    fn inst_sbc_a_abs_x(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.x as u16);
        let val = mem.read8_mut(addr);
        self.regs.a = self.sbc_flags(self.regs.a, val);
        self.cycles += 5;
    }

    fn inst_sbc_a_abs_y(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let addr = base.wrapping_add(self.regs.y as u16);
        let val = mem.read8_mut(addr);
        self.regs.a = self.sbc_flags(self.regs.a, val);
        self.cycles += 5;
    }

    fn inst_sbc_a_dp_ind_y(&mut self, mem: &mut Memory) {
        let offset = self.read_immediate(mem) as u16;
        let ptr_addr = self.dp_base() | offset;
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        let base = (hi << 8) | lo;
        let addr = base.wrapping_add(self.regs.y as u16);
        let val = mem.read8_mut(addr);
        self.regs.a = self.sbc_flags(self.regs.a, val);
        self.cycles += 6;
    }

    fn inst_sbc_dp_imm(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_dp_imm(mem);
        let result = self.sbc_flags(dst, src);
        mem.write8(addr, result);
        self.cycles += 5;
    }

    fn inst_sbc_ix_iy(&mut self, mem: &mut Memory) {
        let (addr, dst, src) = self.read_ix_iy(mem);
        let result = self.sbc_flags(dst, src);
        mem.write8(addr, result);
        self.cycles += 5;
    }

    /// SET1 d.bit — set the given bit (0-7) at a direct page address.
    /// No flags affected. 4 cycles.
    fn inst_set1(&mut self, mem: &mut Memory, bit: u8) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let val = mem.read8_mut(addr) | (1 << bit);
        mem.write8(addr, val);
        self.cycles += 4;
    }

    /// CLR1 d.bit — clear the given bit (0-7) at a direct page address.
    /// No flags affected. 4 cycles.
    fn inst_clr1(&mut self, mem: &mut Memory, bit: u8) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let val = mem.read8_mut(addr) & !(1 << bit);
        mem.write8(addr, val);
        self.cycles += 4;
    }

    /// Shared BBS/BBC handler — reads the dp offset and signed branch
    /// displacement, tests the given bit, and branches if the bit's
    /// state matches `branch_if_set` (true for BBS, false for BBC).
    /// 5 cycles if not taken, 7 if taken.
    fn inst_bbs_bbc(&mut self, mem: &mut Memory, bit: u8, branch_if_set: bool) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let value = mem.read8_mut(addr);
        let offset = self.read_immediate(mem) as i8;
        let bit_is_set = (value & (1 << bit)) != 0;

        self.cycles += 5;
        if bit_is_set == branch_if_set {
            self.regs.pc = self.regs.pc.wrapping_add(offset as i16 as u16);
            self.cycles += 2;
        }
    }

    /// Decode the `m.bit` operand used by MOV1/AND1/OR1/EOR1/NOT1: a 16-bit
    /// immediate packs a 13-bit absolute address (low bits) and a 3-bit bit
    /// position (high bits).
    fn read_mem_bit(&mut self, mem: &mut Memory) -> (u16, u8) {
        let packed = self.read_immediate16(mem);
        let addr = packed & 0x1FFF;
        let bit = ((packed >> 13) & 0x07) as u8;
        (addr, bit)
    }

    /// TSET1 !a — test (A - mem) for N/Z (like a CMP), then OR A's bits
    /// into mem. 6 cycles.
    fn inst_tset1(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let data = mem.read8_mut(addr);
        let diff = self.regs.a.wrapping_sub(data);
        self.set_flag(FLAG_N, (diff & 0x80) != 0);
        self.set_flag(FLAG_Z, diff == 0);
        mem.write8(addr, data | self.regs.a);
        self.cycles += 6;
    }

    /// TCLR1 !a — test (A - mem) for N/Z, then clear A's bits in mem.
    /// 6 cycles.
    fn inst_tclr1(&mut self, mem: &mut Memory) {
        let addr = self.read_immediate16(mem);
        let data = mem.read8_mut(addr);
        let diff = self.regs.a.wrapping_sub(data);
        self.set_flag(FLAG_N, (diff & 0x80) != 0);
        self.set_flag(FLAG_Z, diff == 0);
        mem.write8(addr, data & !self.regs.a);
        self.cycles += 6;
    }

    /// MOV1 C,m.b — copy a memory bit into carry. No other flags affected.
    /// 4 cycles.
    fn inst_mov1_c_mb(&mut self, mem: &mut Memory) {
        let (addr, bit) = self.read_mem_bit(mem);
        let value = mem.read8_mut(addr);
        self.set_flag(FLAG_C, (value & (1 << bit)) != 0);
        self.cycles += 4;
    }

    /// MOV1 m.b,C — copy carry into a memory bit. No flags affected.
    /// 6 cycles.
    fn inst_mov1_mb_c(&mut self, mem: &mut Memory) {
        let (addr, bit) = self.read_mem_bit(mem);
        let mut value = mem.read8_mut(addr);
        if self.get_flag(FLAG_C) {
            value |= 1 << bit;
        } else {
            value &= !(1 << bit);
        }
        mem.write8(addr, value);
        self.cycles += 6;
    }

    /// OR1 C,m.b — C = C OR bit(m.b). 5 cycles.
    fn inst_or1_c_mb(&mut self, mem: &mut Memory) {
        let (addr, bit) = self.read_mem_bit(mem);
        let bit_set = (mem.read8_mut(addr) & (1 << bit)) != 0;
        self.set_flag(FLAG_C, self.get_flag(FLAG_C) || bit_set);
        self.cycles += 5;
    }

    /// OR1 C,/m.b — C = C OR NOT bit(m.b). 5 cycles.
    fn inst_or1_c_not_mb(&mut self, mem: &mut Memory) {
        let (addr, bit) = self.read_mem_bit(mem);
        let bit_set = (mem.read8_mut(addr) & (1 << bit)) != 0;
        self.set_flag(FLAG_C, self.get_flag(FLAG_C) || !bit_set);
        self.cycles += 5;
    }

    /// AND1 C,m.b — C = C AND bit(m.b). 4 cycles.
    fn inst_and1_c_mb(&mut self, mem: &mut Memory) {
        let (addr, bit) = self.read_mem_bit(mem);
        let bit_set = (mem.read8_mut(addr) & (1 << bit)) != 0;
        self.set_flag(FLAG_C, self.get_flag(FLAG_C) && bit_set);
        self.cycles += 4;
    }

    /// AND1 C,/m.b — C = C AND NOT bit(m.b). 4 cycles.
    fn inst_and1_c_not_mb(&mut self, mem: &mut Memory) {
        let (addr, bit) = self.read_mem_bit(mem);
        let bit_set = (mem.read8_mut(addr) & (1 << bit)) != 0;
        self.set_flag(FLAG_C, self.get_flag(FLAG_C) && !bit_set);
        self.cycles += 4;
    }

    /// EOR1 C,m.b — C = C XOR bit(m.b). 5 cycles.
    fn inst_eor1_c_mb(&mut self, mem: &mut Memory) {
        let (addr, bit) = self.read_mem_bit(mem);
        let bit_set = (mem.read8_mut(addr) & (1 << bit)) != 0;
        self.set_flag(FLAG_C, self.get_flag(FLAG_C) ^ bit_set);
        self.cycles += 5;
    }

    /// NOT1 m.b — invert the given bit directly in memory. No flags affected.
    /// 5 cycles.
    fn inst_not1_mb(&mut self, mem: &mut Memory) {
        let (addr, bit) = self.read_mem_bit(mem);
        let value = mem.read8_mut(addr) ^ (1 << bit);
        mem.write8(addr, value);
        self.cycles += 5;
    }

    /// JMP !a — absolute jump. 3 cycles.
    fn inst_jmp_abs(&mut self, mem: &mut Memory) {
        self.regs.pc = self.read_immediate16(mem);
        self.cycles += 3;
    }

    /// JMP [!a+X] — jump through a 16-bit pointer stored at !a+X. 6 cycles.
    fn inst_jmp_abs_x_ind(&mut self, mem: &mut Memory) {
        let base = self.read_immediate16(mem);
        let ptr_addr = base.wrapping_add(self.regs.x as u16);
        let lo = mem.read8_mut(ptr_addr) as u16;
        let hi = mem.read8_mut(ptr_addr.wrapping_add(1)) as u16;
        self.regs.pc = (hi << 8) | lo;
        self.cycles += 6;
    }

    /// RETI — return from interrupt: pop PSW, then pop PC.
    /// ASSUMES CALL pushes PCH then PCL (so this pops PCL then PCH) —
    /// verify this matches the existing RET/CALL convention. 6 cycles.
    fn inst_reti(&mut self, mem: &mut Memory) {
        self.regs.psw = self.stack_pop(mem);
        let lo = self.stack_pop(mem) as u16;
        let hi = self.stack_pop(mem) as u16;
        self.regs.pc = (hi << 8) | lo;
        self.cycles += 6;
    }

    /// CBNE dp,rel — compare A with dp, branch if not equal. No flags
    /// affected, no write-back. 5 cycles not taken, 7 taken.
    fn inst_cbne_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let value = mem.read8_mut(addr);
        let offset = self.read_immediate(mem) as i8;
        self.cycles += 5;
        if self.regs.a != value {
            self.regs.pc = self.regs.pc.wrapping_add(offset as i16 as u16);
            self.cycles += 2;
        }
    }

    /// CBNE dp+X,rel — compare A with dp+X, branch if not equal.
    /// 6 cycles not taken, 8 taken.
    fn inst_cbne_dp_x(&mut self, mem: &mut Memory) {
        let offset_dp = self.read_immediate(mem) as u16;
        let addr = self.dp_base() | (offset_dp + self.regs.x as u16) & 0xFF;
        let value = mem.read8_mut(addr);
        let offset = self.read_immediate(mem) as i8;
        self.cycles += 6;
        if self.regs.a != value {
            self.regs.pc = self.regs.pc.wrapping_add(offset as i16 as u16);
            self.cycles += 2;
        }
    }

    /// DBNZ dp,rel — decrement dp byte, branch if result != 0.
    /// No flags affected. 6 cycles not taken, 8 taken.
    fn inst_dbnz_dp(&mut self, mem: &mut Memory) {
        let addr = self.dp_base() | self.read_immediate(mem) as u16;
        let value = mem.read8_mut(addr).wrapping_sub(1);
        mem.write8(addr, value);
        let offset = self.read_immediate(mem) as i8;
        self.cycles += 6;
        if value != 0 {
            self.regs.pc = self.regs.pc.wrapping_add(offset as i16 as u16);
            self.cycles += 2;
        }
    }

    /// DBNZ Y,rel — decrement Y, branch if result != 0. No flags affected.
    /// 4 cycles not taken, 6 taken.
    fn inst_dbnz_y(&mut self, mem: &mut Memory) {
        self.regs.y = self.regs.y.wrapping_sub(1);
        let offset = self.read_immediate(mem) as i8;
        self.cycles += 4;
        if self.regs.y != 0 {
            self.regs.pc = self.regs.pc.wrapping_add(offset as i16 as u16);
            self.cycles += 2;
        }
    }

    /// BRK — software break: push PC then PSW, set I and B, jump via the
    /// vector at $FFDE (same vector slot as TCALL 0). 8 cycles.
    fn inst_brk(&mut self, mem: &mut Memory) {
        let pc = self.regs.pc;
        self.stack_push(mem, (pc >> 8) as u8);
        self.stack_push(mem, pc as u8);
        self.stack_push(mem, self.regs.psw);
        self.set_flag(FLAG_B, true);
        self.set_flag(FLAG_I, true);
        self.regs.pc = mem.read16(0xFFDE);
        self.cycles += 8;
    }
}
