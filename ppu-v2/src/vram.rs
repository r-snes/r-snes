use crate::constants::VRAM_SIZE;

pub struct VRAM {
    memory: [u8; VRAM_SIZE],

    // Registers
    pub vmain: u8, // $2115
}

impl VRAM {
    pub fn new() -> Self {
        Self {
            memory: [0; VRAM_SIZE],
            vmain: 0,
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
}
