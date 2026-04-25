use crate::constants::CGRAM_SIZE;
use common::u16_split::U16Split;

/// Helper enum to keep track of the byte phase in the CGRAM
#[derive(Debug, PartialEq, Eq)]
enum BytePhase {
    /// Next read/write affects the low byte of the addressed word
    Low,

    /// Next read/write affects the high byte of the addressed word
    High,
}
use BytePhase::*;

impl BytePhase {
    fn flip(&mut self) {
        *self = match self {
            Low => High,
            High => Low,
        };
    }
}

pub struct CGRAM {
    pub memory: [u16; CGRAM_SIZE],

    word_addr: u8, // Internal 8-bit word address (0–255)
    byte_phase: BytePhase,
    write_latch: u8,

    pub ppu_open_bus: u8, // bit 7 used during high-byte read
}

impl CGRAM {
    pub fn new() -> Self {
        Self {
            memory: [0; CGRAM_SIZE],
            word_addr: 0,
            byte_phase: Low,
            write_latch: 0,
            ppu_open_bus: 0,
        }
    }

    // ============================================================
    // $2121 — CGADD
    // ============================================================

    pub fn write_addr(&mut self, value: u8) {
        self.word_addr = value;
        self.byte_phase = Low;
    }

    // ============================================================
    // $2122 — CGDATA (Write)
    // ============================================================

    pub fn write_data(&mut self, value: u8) {
        if self.byte_phase == Low {
            self.write_latch = value;
        } else {
            let word = &mut self.memory[self.word_addr as usize];
            *word.lo_mut() = self.write_latch;
            *word.hi_mut() = value & 0x7F;

            self.word_addr = self.word_addr.wrapping_add(1);
        }

        self.byte_phase.flip();
        self.ppu_open_bus = value;
    }

    // ============================================================
    // $213B — CGDATAREAD
    // ============================================================

    pub fn read_data(&mut self) -> u8 {
        let word = self.memory[self.word_addr as usize];
        let value = match self.byte_phase {
            Low => *word.lo(),
            // bit 7 of high byte comes from open bus
            High => *word.hi() | (self.ppu_open_bus & 0x80),
        };

        if self.byte_phase == High {
            self.word_addr = self.word_addr.wrapping_add(1);
        }
        self.byte_phase.flip();
        self.ppu_open_bus = value;

        value
    }

    // ============================================================
    // Helpers
    // ============================================================

    pub fn read(&self, word_index: u8) -> u16 {
        self.memory[word_index as usize]
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
    fn test_write_addr_sets_word_address() {
        let mut cgram = CGRAM::new();

        cgram.byte_phase = High;

        cgram.write_addr(5);

        assert_eq!(cgram.word_addr, 5);
        assert_eq!(cgram.byte_phase, Low);
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

        assert_eq!(cgram.memory[2], 0x1234 & 0x7FFF);
    }

    // This test verifies that the high byte write masks out bit 7 (unused bit 15).
    #[test]
    fn test_write_high_byte_masks_bit7() {
        let mut cgram = CGRAM::new();

        cgram.write_addr(0);

        cgram.write_data(0xAA);
        cgram.write_data(0xFF); // bit 7 should be masked

        assert_eq!(cgram.memory[0], 0x7FAA);
    }

    // This test verifies that the internal word address wraps
    // correctly at the 8-bit boundary.
    #[test]
    fn test_word_addr_wraps_8bit() {
        let mut cgram = CGRAM::new();

        cgram.word_addr = 0xFF;
        cgram.byte_phase = High;

        cgram.write_data(0x11);

        assert_eq!(cgram.word_addr, 0);
    }

    // ============================================================
    // $213B — CGDATAREAD
    // ============================================================

    // This test verifies that sequential reads return low then high bytes in the correct order.
    #[test]
    fn test_read_data_low_high_sequence() {
        let mut cgram = CGRAM::new();

        cgram.memory[0] = 0x5678;

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

    // // This test verifies that current_word correctly reconstructs a 16-bit value from CGRAM.
    // #[test]
    // fn test_current_word_reads_correct_value() {
    //     let mut cgram = CGRAM::new();
    //
    //     cgram.memory[10] = 0xCD;
    //     cgram.memory[11] = 0xAB;
    //
    //     let word = cgram.current_word(5);
    //
    //     assert_eq!(word, 0xABCD);
    // }

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

        assert_eq!(cgram.memory[0], 0x2211 & 0x7FFF);
        assert_eq!(cgram.memory[1], 0x4433 & 0x7FFF);
    }
}
