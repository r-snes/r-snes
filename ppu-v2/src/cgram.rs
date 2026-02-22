use crate::constants::CGRAM_SIZE;

pub struct CGRAM {
    pub memory: [u8; CGRAM_SIZE],

    byte_addr: u16, // Internal 9-bit byte address (0–511)
    byte_phase: u8, // 0 = low byte phase, 1 = high byte phase
    write_latch: u8,

    pub ppu_open_bus: u8, // bit 7 used during high-byte read
}

impl CGRAM {
    pub fn new() -> Self {
        Self {
            memory: [0; CGRAM_SIZE],
            byte_addr: 0,
            byte_phase: 0,
            write_latch: 0,
            ppu_open_bus: 0,
        }
    }

    // ============================================================
    // $2121 — CGADD
    // ============================================================

    pub fn write_addr(&mut self, value: u8) {
        self.byte_addr = ((value as u16) << 1) & 0x1FF;
        self.byte_phase = 0;
    }

    // ============================================================
    // $2122 — CGDATA (Write)
    // ============================================================

    pub fn write_data(&mut self, value: u8) {
        if self.byte_phase == 0 {
            self.write_latch = value;
        } else {
            let low_addr = (self.byte_addr - 1) & 0x1FF;
            let high_addr = self.byte_addr & 0x1FF;

            self.memory[low_addr as usize] = self.write_latch;
            self.memory[high_addr as usize] = value & 0x7F; // bit 15 unused
        }
        self.byte_addr = (self.byte_addr + 1) & 0x1FF;
        self.byte_phase ^= 1;
        self.ppu_open_bus = value;
    }

    // ============================================================
    // $213B — CGDATAREAD
    // ============================================================

    pub fn read_data(&mut self) -> u8 {
        let addr = self.byte_addr & 0x1FF;
        let mut value = self.memory[addr as usize];

        if self.byte_phase == 1 {
            // High byte read -> bit 7 comes from PPU open bus
            value = (value & 0x7F) | (self.ppu_open_bus & 0x80);
        }
        self.byte_addr = (self.byte_addr + 1) & 0x1FF;
        self.byte_phase ^= 1;
        self.ppu_open_bus = value;

        value
    }

    // ============================================================
    // Helpers
    // ============================================================

    pub fn current_word(&self, word_index: u8) -> u16 {
        let base = ((word_index as u16) << 1) & 0x1FF;
        let low = self.memory[base as usize] as u16;
        let high = self.memory[(base + 1) as usize] as u16;
        (high << 8) | low
    }
}
