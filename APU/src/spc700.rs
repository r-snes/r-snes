pub struct Spc700 {
    // 8-bit registers
    pub a: u8,  // Accumulator
    pub x: u8,  // Index X
    pub y: u8,  // Index Y
    pub sp: u8, // Stack Pointer
    pub pc: u16, // Program Counter

    // Status flags (bitfield)
    pub psw: u8,

    // 64 KB memory
    pub ram: [u8; 65536],
}

impl Spc700 {
    pub fn new() -> Self {
        Spc700 {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xff,
            pc: 0x0000,
            psw: 0b0000_0000,
            ram: [0; 65536],
        }
    }

    pub fn reset(&mut self) {
        self.sp = 0xff;
        self.psw = 0b0000_0000;
        self.pc = self.read_word(0xfffe); // reset vector at 0xFFFE
    }

    pub fn step(&mut self) {
        let opcode = self.fetch_byte();
        self.execute(opcode);
    }

    pub fn fetch_byte(&mut self) -> u8 {
        let byte = self.read_byte(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        let low = self.read_byte(addr) as u16;
        let high = self.read_byte(addr.wrapping_add(1)) as u16;
        (high << 8) | low
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        self.ram[addr as usize] = value;
    }

    pub fn execute(&mut self, opcode: u8) {
        match opcode {
            0x00 => { // NOP
                // Do nothing
            }
            0xE8 => { // MOV A 
                let imm = self.fetch_byte();
                self.a = imm;
                self.update_zero_neg_flags(self.a);
            }
            0xE4 => { // MOV X
                let imm = self.fetch_byte();
                self.x = imm;
                self.update_zero_neg_flags(self.x);
            }
            0xEC => { // MOV Y
                let imm = self.fetch_byte();
                self.y = imm;
                self.update_zero_neg_flags(self.y);
            }
            0xC4 => { // MOV (addr), A
                let addr = self.fetch_byte() as u16;
                self.write_byte(addr, self.a);
            }
            0xE5 => { // MOV A, (addr)
                let addr = self.fetch_byte() as u16;
                self.a = self.read_byte(addr);
                self.update_zero_neg_flags(self.a);
            }
            0x5F => { // JMP addr
                let low = self.fetch_byte() as u16;
                let high = self.fetch_byte() as u16;
                self.pc = (high << 8) | low;
            }
            0x2F => { // BRA (Branch Always)
                let offset = self.fetch_byte() as i8;
                self.pc = self.pc.wrapping_add(offset as u16);
            }
            _ => {
                println!("Unknown opcode: 0x{:02X}", opcode);
            }
        }
    }    

    fn update_zero_neg_flags(&mut self, value: u8) {
        if value == 0 {
            self.psw |= 0x02; // Set Zero flag
        } else {
            self.psw &= !0x02; // Clear Zero flag
        }
        if value & 0x80 != 0 {
            self.psw |= 0x80; // Set Negative flag
        } else {
            self.psw &= !0x80; // Clear Negative flag
        }
    }
}
