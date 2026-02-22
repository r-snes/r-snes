use crate::constants::CGRAM_SIZE;

pub struct CGRAM {
    pub memory: [u8; CGRAM_SIZE],

    byte_addr: u16, // Internal 9-bit byte address (0–511)
    byte_phase: u8, // 0 = low byte phase, 1 = high byte phase
}

impl CGRAM {
    pub fn new() -> Self {
        Self {
            memory: [0; CGRAM_SIZE],
            byte_addr: 0,
            byte_phase: 0,
        }
    }

    // ============================================================
    // $2121 — CGADD
    // ============================================================

    pub fn write_addr(&mut self, value: u8) {
        self.byte_addr = ((value as u16) << 1) & 0x1FF;
        self.byte_phase = 0;
    }
}
