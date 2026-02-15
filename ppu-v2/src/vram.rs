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
}
