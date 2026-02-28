use crate::constants::VRAM_SIZE;

pub struct VRAM {
    pub memory: [u8; VRAM_SIZE],

    // Registers
    pub vmain: u8, // $2115
    pub vmadd: u16, // current word address (0–0x7FFF)

    // Internal state
    pub vram_latch: u16, // word latch for reads
}

impl VRAM {
    pub fn new() -> Self {
        Self {
            memory: [0; VRAM_SIZE],
            vmain: 0,
            vmadd: 0,
            vram_latch: 0,
        }
    }

    // ============================================================
    // $2115 - VMAIN
    // ============================================================

    pub fn write_vmain(&mut self, value: u8) {
        self.vmain = value;
    }

    // ============================================================
    // Address increment logic
    // ============================================================

    pub fn increment_amount(&self) -> u16 {
        match self.vmain & 0b11 {
            0 => 1,       // increment by 1 word
            1 => 32,      // increment by 32 words
            2 | 3 => 128, // increment by 128 words
            _ => unreachable!(),
        }
    }

    pub fn increment_after_low(&self) -> bool {
        (self.vmain & 0x80) == 0
    }

    pub fn increment_after_high(&self) -> bool {
        (self.vmain & 0x80) != 0
    }
    
    fn increment_vmadd(&mut self) {
        self.vmadd = (self.vmadd + self.increment_amount()) & 0x7FFF;
    }

    // ============================================================
    // VMADD ($2116 / $2117)
    // ============================================================

    pub fn write_vmadd_low(&mut self, value: u8) {
        self.vmadd = (self.vmadd & 0x7F00) | value as u16;
        self.vmadd &= 0x7FFF;
        self.load_latch();
    }

    pub fn write_vmadd_high(&mut self, value: u8) {
        self.vmadd = ((value as u16 & 0x7F) << 8) | (self.vmadd & 0x00FF);
        self.vmadd &= 0x7FFF;
        self.load_latch();
    }

    // ============================================================
    // VRAM DATA WRITE ($2118 / $2119)
    // ============================================================

    pub fn write_vmdatal(&mut self, value: u8) {
        let addr = self.byte_address();
        self.memory[addr] = value;
        
        self.load_latch();

        if self.increment_after_low() {
            self.increment_vmadd();
        }
    }
    
    pub fn write_vmdatah(&mut self, value: u8) {
        let addr = self.byte_address();
        self.memory[addr + 1] = value;

        self.load_latch();

        if self.increment_after_high() {
            self.increment_vmadd();
        }
    }

    // ============================================================
    // VRAM DATA READ ($2139 / $213A)
    // ============================================================

    pub fn read_vmdatal(&mut self) -> u8 {
        let value = (self.vram_latch & 0x00FF) as u8;

        if self.increment_after_low() {
            self.load_latch();
            self.increment_vmadd();
        }

        value
    }

    pub fn read_vmdatah(&mut self) -> u8 {
        let value = (self.vram_latch >> 8) as u8;

        if self.increment_after_high() {
            self.load_latch();
            self.increment_vmadd();
        }

        value
    }

    // ============================================================
    // Helpers
    // ============================================================

    pub fn byte_address(&self) -> usize {
        ((self.vmadd & 0x7FFF) as usize) * 2
    }

    pub fn load_latch(&mut self) {
        let addr = self.byte_address();
        let low = self.memory[addr];
        let high = self.memory[addr + 1];
        self.vram_latch = (high as u16) << 8 | low as u16;
    }
}
