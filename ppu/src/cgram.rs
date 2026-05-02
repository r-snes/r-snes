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
    pub memory: [u16; CGRAM_SIZE / 2], // CGRAM stored as u16 words instead of bytes

    word_addr: u8, // Internal 8-bit word address (0–255)
    byte_phase: BytePhase,
    write_latch: u8,

    pub ppu_open_bus: u8, // bit 7 used during high-byte read
}

impl CGRAM {
    pub fn new() -> Self {
        Self {
            memory: [0; CGRAM_SIZE / 2],
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
    // CGRAM::new
    // ============================================================
    
    /// A freshly created CGRAM must have all memory zeroed, word_addr at 0,
    /// byte_phase at Low, and open bus at 0.
    #[test]
    fn test_new_zeroed() {
        let cgram = CGRAM::new();
        assert!(cgram.memory.iter().all(|&w| w == 0));
        assert_eq!(cgram.ppu_open_bus, 0);
    }

    // ============================================================
    // BytePhase::flip
    // ============================================================

    /// flip must toggle Low -> High.
    #[test]
    fn test_byte_phase_flip_low_to_high() {
        let mut phase = Low;
        phase.flip();
        assert_eq!(phase, High);
    }

    /// flip must toggle High -> Low.
    #[test]
    fn test_byte_phase_flip_high_to_low() {
        let mut phase = High;
        phase.flip();
        assert_eq!(phase, Low);
    }

    // ============================================================
    // write_addr ($2121)
    // ============================================================

    /// write_addr must set the word address and reset byte_phase to Low.
    #[test]
    fn test_write_addr_sets_word_address() {
        let mut cgram = CGRAM::new();
        // Put cgram in High phase first
        cgram.write_data(0x10);
        cgram.write_addr(0x42);
        // Only observable side-effect: next write goes to word 0x42 (low byte)
        cgram.write_data(0xAB);
        cgram.write_data(0x3F);
        assert_eq!(cgram.memory[0x42], 0x3FAB);
    }

    /// write_addr must reset byte_phase to Low even if previously in High phase.
    #[test]
    fn test_write_addr_resets_phase_to_low() {
        let mut cgram = CGRAM::new();
        cgram.write_data(0xFF); // phase -> High
        cgram.write_addr(0x00); // must reset to Low
        // Writing one byte should only latch (Low phase), not commit
        cgram.write_data(0xBB);
        assert_eq!(cgram.memory[0x00], 0x0000); // nothing committed yet
    }

    // ============================================================
    // write_data ($2122)
    // ============================================================

    /// First write (Low phase) must latch the byte without touching memory.
    #[test]
    fn test_write_data_low_phase_only_latches() {
        let mut cgram = CGRAM::new();
        cgram.write_data(0xAB);
        assert_eq!(cgram.memory[0x00], 0x0000);
    }

    /// Second write (High phase) must commit lo+hi to the current word, masking bit 7 of hi.
    #[test]
    fn test_write_data_low_then_high_commits_word() {
        let mut cgram = CGRAM::new();
        cgram.write_data(0xCD); // lo latch
        cgram.write_data(0xFF); // hi write — bit 7 masked -> 0x7F
        assert_eq!(cgram.memory[0x00], 0x7FCD);
    }

    /// After a complete low+high write, word_addr must increment by 1.
    #[test]
    fn test_write_data_increments_word_addr_after_high() {
        let mut cgram = CGRAM::new();
        cgram.write_data(0x11);
        cgram.write_data(0x22);
        // Next pair goes to word 0x01
        cgram.write_data(0x33);
        cgram.write_data(0x44);
        assert_eq!(cgram.memory[0x01], 0x4433);
    }

    /// High byte bit 7 must always be masked to 0 on write (CGRAM stores 15-bit colours).
    #[test]
    fn test_write_high_byte_masks_bit7() {
        let mut cgram = CGRAM::new();
        cgram.write_data(0x00);
        cgram.write_data(0xFF); // bit 7 must be stripped -> 0x7F
        assert_eq!((cgram.memory[0x00] >> 8) as u8, 0x7F);
    }

    /// write_data must update ppu_open_bus with the written value on every write.
    #[test]
    fn test_write_data_updates_open_bus() {
        let mut cgram = CGRAM::new();
        cgram.write_data(0xAB);
        assert_eq!(cgram.ppu_open_bus, 0xAB);
        cgram.write_data(0x3C);
        assert_eq!(cgram.ppu_open_bus, 0x3C);
    }

    /// word_addr must wrap from 0xFF back to 0x00 after a complete write at address 0xFF.
    #[test]
    fn test_write_data_word_addr_wraps() {
        let mut cgram = CGRAM::new();
        cgram.write_addr(0xFF);
        cgram.write_data(0x12);
        cgram.write_data(0x34);
        // After write at 0xFF, addr wraps to 0x00
        cgram.write_data(0xAA);
        cgram.write_data(0x55);
        assert_eq!(cgram.memory[0x00], 0x55AA);
    }

    /// Sequential writes across multiple words must not corrupt adjacent entries.
    #[test]
    fn test_sequential_write_multiple_words() {
        let mut cgram = CGRAM::new();
        for i in 0u8..4 {
            cgram.write_data(i);        // lo
            cgram.write_data(i + 0x10); // hi (bit 7 clear, no masking effect)
        }
        assert_eq!(cgram.memory[0x00], 0x1000);
        assert_eq!(cgram.memory[0x01], 0x1101);
        assert_eq!(cgram.memory[0x02], 0x1202);
        assert_eq!(cgram.memory[0x03], 0x1303);
    }

    // ============================================================
    // read_data ($213B)
    // ============================================================

    /// Reading in Low phase must return the low byte of the current word.
    #[test]
    fn test_read_data_low_phase_returns_lo_byte() {
        let mut cgram = CGRAM::new();
        cgram.memory[0x00] = 0x1234;
        let val = cgram.read_data();
        assert_eq!(val, 0x34);
    }

    /// Reading in High phase must return hi byte OR'd with open-bus bit 7.
    #[test]
    fn test_read_data_high_phase_returns_hi_with_open_bus_bit7() {
        let mut cgram = CGRAM::new();
        cgram.memory[0x00] = 0x1234;
        let _lo = cgram.read_data(); // Low phase — ppu_open_bus becomes 0x34
        // Simulate open bus bit 7 being set by a previous PPU operation
        cgram.ppu_open_bus = 0x80;
        let hi = cgram.read_data(); // High phase
        // hi byte of 0x1234 = 0x12; open bus bit7 = 0x80 -> 0x12 | 0x80 = 0x92
        assert_eq!(hi, 0x92);
    }

    /// Bit 7 of the high-byte read must come from open bus, not from CGRAM data.
    #[test]
    fn test_open_bus_bit7_on_high_read() {
        let mut cgram = CGRAM::new();
        cgram.memory[0x00] = 0x7F00; // hi = 0x7F (bit 7 clear in CGRAM)
        let _lo = cgram.read_data(); // Low phase — ppu_open_bus becomes 0x00
        // Force open bus bit 7 before the high read
        cgram.ppu_open_bus = 0x80;
        let hi = cgram.read_data();
        assert_eq!(hi & 0x80, 0x80); // bit 7 must come from open bus
    }

    /// After a complete low+high read, word_addr must increment by 1.
    #[test]
    fn test_read_data_increments_word_addr_after_high_phase() {
        let mut cgram = CGRAM::new();
        cgram.memory[0x00] = 0x1111;
        cgram.memory[0x01] = 0x2222;
        let _lo0 = cgram.read_data(); // Low  @ 0x00
        let _hi0 = cgram.read_data(); // High @ 0x00 -> addr increments to 0x01
        let lo1 = cgram.read_data();  // Low  @ 0x01
        assert_eq!(lo1, 0x22);
    }

    /// read_data must NOT increment word_addr after the Low phase read.
    #[test]
    fn test_read_data_no_increment_after_low_phase() {
        let mut cgram = CGRAM::new();
        cgram.memory[0x00] = 0xABCD;
        let _lo = cgram.read_data(); // Low phase — addr must stay at 0x00
        // High phase read should still be from word 0x00
        let hi = cgram.read_data();
        assert_eq!(hi & 0x7F, 0xAB & 0x7F);
    }

    /// read_data must update ppu_open_bus with the returned value.
    #[test]
    fn test_read_data_updates_open_bus() {
        let mut cgram = CGRAM::new();
        cgram.memory[0x00] = 0x1234;
        let lo = cgram.read_data();
        assert_eq!(cgram.ppu_open_bus, lo);
    }

    /// word_addr must wrap from 0xFF to 0x00 after a complete read at address 0xFF.
    #[test]
    fn test_read_data_word_addr_wraps() {
        let mut cgram = CGRAM::new();
        cgram.write_addr(0xFF);
        cgram.memory[0xFF] = 0x1234;
        cgram.memory[0x00] = 0x5678;
        let _lo = cgram.read_data();
        let _hi = cgram.read_data(); // addr wraps to 0x00
        let lo_next = cgram.read_data();
        assert_eq!(lo_next, 0x78);
    }

    // ============================================================
    // read helper
    // ============================================================

    /// read() must return the raw 16-bit word at the given index without side effects.
    #[test]
    fn test_read_helper_returns_raw_word() {
        let mut cgram = CGRAM::new();
        cgram.memory[0x10] = 0xBEEF;
        assert_eq!(cgram.read(0x10), 0xBEEF);
    }

    /// read() must not modify word_addr, byte_phase, or open_bus.
    #[test]
    fn test_read_helper_has_no_side_effects() {
        let mut cgram = CGRAM::new();
        cgram.memory[0x05] = 0x1234;
        cgram.write_addr(0x05);
        let _ = cgram.read(0x05);
        // If read() had side effects, the subsequent write_data sequence would go wrong
        cgram.write_data(0xAB);
        cgram.write_data(0x3F);
        assert_eq!(cgram.memory[0x05], 0x3FAB);
    }

    // ============================================================
    // Round-trip
    // ============================================================

    /// A value written via write_data must be recoverable via read_data at the same address.
    #[test]
    fn test_round_trip_write_then_read() {
        let mut cgram = CGRAM::new();
        cgram.write_addr(0x20);
        cgram.write_data(0x56); // lo
        cgram.write_data(0x3A); // hi (bit 7 clear)

        cgram.write_addr(0x20);
        let lo = cgram.read_data();
        let hi = cgram.read_data();

        assert_eq!(lo, 0x56);
        assert_eq!(hi & 0x7F, 0x3A);
    }
}
