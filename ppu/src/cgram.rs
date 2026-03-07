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

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // $2121 — CGADD
    // ============================================================

    // This test verifies that writing to CGADD correctly sets the byte address and resets the byte phase.
    #[test]
    fn test_write_addr_sets_byte_address() {
        let mut cgram = CGRAM::new();

        cgram.write_addr(5);

        assert_eq!(cgram.byte_addr, 10); // 5 << 1
        assert_eq!(cgram.byte_phase, 0);
    }

    // ============================================================
    // $2122 — CGDATA (Write)
    // ============================================================

    // This test verifies that writing low then high bytes correctly stores a 16-bit color in CGRAM.
    #[test]
    fn test_write_data_low_then_high() {
        let mut cgram = CGRAM::new();

        cgram.write_addr(0x02);

        cgram.write_data(0x34); // low
        cgram.write_data(0x12); // high

        assert_eq!(cgram.memory[4], 0x34);
        assert_eq!(cgram.memory[5], 0x12 & 0x7F);
    }

    // This test verifies that the high byte write masks out bit 7 (unused bit 15).
    #[test]
    fn test_write_high_byte_masks_bit7() {
        let mut cgram = CGRAM::new();

        cgram.write_addr(0);

        cgram.write_data(0xAA);
        cgram.write_data(0xFF); // bit 7 should be masked

        assert_eq!(cgram.memory[0], 0xAA);
        assert_eq!(cgram.memory[1], 0x7F);
    }

    // This test verifies that the internal byte address wraps correctly at the 9-bit boundary.
    #[test]
    fn test_byte_addr_wraps_9bit() {
        let mut cgram = CGRAM::new();

        cgram.byte_addr = 0x1FF;
        cgram.byte_phase = 0;

        cgram.write_data(0x11);

        assert_eq!(cgram.byte_addr, 0);
    }

    // ============================================================
    // $213B — CGDATAREAD
    // ============================================================

    // This test verifies that sequential reads return low then high bytes in the correct order.
    #[test]
    fn test_read_data_low_high_sequence() {
        let mut cgram = CGRAM::new();

        cgram.memory[0] = 0x78;
        cgram.memory[1] = 0x56;

        cgram.write_addr(0);

        let low = cgram.read_data();
        let high = cgram.read_data();

        assert_eq!(low, 0x78);
        assert_eq!(high & 0x7F, 0x56);
    }

    // This test verifies that bit 7 of the high byte read comes from the PPU open bus.
    #[test]
    fn test_open_bus_bit7_on_high_read() {
        let mut cgram = CGRAM::new();

        cgram.memory[0] = 0x80;
        cgram.memory[1] = 0x22;

        cgram.write_addr(0);

        cgram.read_data();
        let high = cgram.read_data();

        assert_eq!(high & 0x80, 0x80);
    }

    // ============================================================
    // OTHERS
    // ============================================================

    // This test verifies that current_word correctly reconstructs a 16-bit value from CGRAM.
    #[test]
    fn test_current_word_reads_correct_value() {
        let mut cgram = CGRAM::new();

        cgram.memory[10] = 0xCD;
        cgram.memory[11] = 0xAB;

        let word = cgram.current_word(5);

        assert_eq!(word, 0xABCD);
    }

    // $2121 & $2122
    // This test verifies that sequential writes correctly store multiple words in memory.
    #[test]
    fn test_sequential_write_multiple_words() {
        let mut cgram = CGRAM::new();

        cgram.write_addr(0);

        cgram.write_data(0x11);
        cgram.write_data(0x22);

        cgram.write_data(0x33);
        cgram.write_data(0x44);

        assert_eq!(cgram.memory[0], 0x11);
        assert_eq!(cgram.memory[1], 0x22 & 0x7F);
        assert_eq!(cgram.memory[2], 0x33);
        assert_eq!(cgram.memory[3], 0x44 & 0x7F);
    }
}
