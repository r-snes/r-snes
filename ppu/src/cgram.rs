//! Palette memory (CGRAM): 512 bytes, stored as 256 15-bit BGR555 colors.
//!
//! The CPU accesses it through CGADD ($2121) for the word address,
//! CGDATA ($2122) for writes and RDCGRAM ($213B) for reads.
//! Writes and reads go through a shared two-step latch (low byte, then high byte),
//! reset by any CGADD write. Bit 7 of the high byte is not stored
//! and reads back from the PPU2 open bus.

use crate::constants::CGRAM_SIZE;
use crate::registers::PPURegisters;
use crate::write_twice::BytePhase;
use common::u16_split::U16Split;

/// Palette memory, its word address and the open bus used by reads.
pub struct CGRAM {
    /// CGRAM content, stored as 16-bit words (one color per word).
    pub memory: [u16; CGRAM_SIZE / 2],
    /// Internal 8-bit word address (0-255).
    word_addr: u8,
}

impl Default for CGRAM {
    fn default() -> Self {
        Self::new()
    }
}

impl CGRAM {
    /// Creates a zeroed CGRAM.
    pub fn new() -> Self {
        Self {
            memory: [0; CGRAM_SIZE / 2],
            word_addr: 0,
        }
    }

    // ============================================================
    // $2121 - CGADD
    // ============================================================

    /// Writes CGADD ($2121): sets the word address and resets the latch to the low byte.
    pub fn write_addr(&mut self, PPURegisters { cgram_latch, .. }: &mut PPURegisters, value: u8) {
        self.word_addr = value;
        cgram_latch.reset();
    }

    // ============================================================
    // $2122 - CGDATA (Write-twice)
    // ============================================================

    /// Writes CGDATA ($2122): latches the low byte, then stores the full word on the second write.
    pub fn write_data(&mut self, PPURegisters { cgram_latch, .. }: &mut PPURegisters, value: u8) {
        if let Some((lo, hi)) = cgram_latch.write(value) {
            let word = &mut self.memory[self.word_addr as usize];
            *word.lo_mut() = lo;
            *word.hi_mut() = hi & 0x7F;
            self.word_addr = self.word_addr.wrapping_add(1);
        }
    }

    // ============================================================
    // $213B - CGDATAREAD
    // ============================================================

    /// Reads RDCGRAM ($213B): returns (value, driven bits mask); bit 7 of the high byte is left to the PPU2 open bus.
    pub fn read_data(&mut self, PPURegisters { cgram_latch, .. }: &mut PPURegisters) -> (u8, u8) {
        let word = self.memory[self.word_addr as usize];
        let (value, mask) = match cgram_latch.phase {
            BytePhase::Low => (*word.lo(), 0xFF),
            BytePhase::High => (*word.hi(), 0x7F),
        };

        if cgram_latch.phase.is_high() {
            self.word_addr = self.word_addr.wrapping_add(1);
        }
        cgram_latch.phase.flip();
        (value, mask)
    }

    // ============================================================
    // Helpers
    // ============================================================

    /// Returns the color at `word_index`, without side effects (used by rendering).
    pub fn read(&self, word_index: u8) -> u16 {
        self.memory[word_index as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registers::PPURegisters;

    // ============================================================
    // Helpers
    // ============================================================

    fn make_regs() -> PPURegisters {
        PPURegisters::new()
    }

    // ============================================================
    // CGRAM::new
    // ============================================================

    /// A freshly created CGRAM must have all memory zeroed.
    #[test]
    fn test_new_zeroed() {
        let cgram = CGRAM::new();
        assert!(cgram.memory.iter().all(|&w| w == 0));
    }

    // ============================================================
    // write_addr ($2121)
    // ============================================================

    /// write_addr sets the word address and resets the byte phase to Low,
    /// so the next write pair targets the new address.
    #[test]
    fn test_write_addr() {
        let mut cgram = CGRAM::new();
        let mut regs = make_regs();

        // Advance phase to High, then reset via write_addr
        cgram.write_data(&mut regs, 0xFF); // phase -> High
        cgram.write_addr(&mut regs, 0x42); // must reset to Low

        // One write in Low phase should not commit
        cgram.write_data(&mut regs, 0xBB);
        assert_eq!(cgram.memory[0x42], 0x0000);

        // Second write commits the pair to word 0x42
        cgram.write_data(&mut regs, 0x3F);
        assert_eq!(cgram.memory[0x42], 0x3FBB);
    }

    // ============================================================
    // write_data ($2122)
    // ============================================================

    /// write_data latches on the first write (Low phase) without touching memory,
    /// then commits lo+hi on the second write (High phase), masking bit 7 of hi.
    /// After commit, word_addr increments.
    #[test]
    fn test_write_data() {
        let mut cgram = CGRAM::new();
        let mut regs = make_regs();

        // Low phase: no commit
        cgram.write_data(&mut regs, 0xAB);
        assert_eq!(cgram.memory[0x00], 0x0000);

        // High phase: commit with bit 7 of hi masked
        cgram.write_data(&mut regs, 0xFF);
        assert_eq!(cgram.memory[0x00], 0x7FAB);

        // addr incremented: next pair goes to word 0x01
        cgram.write_data(&mut regs, 0x33);
        cgram.write_data(&mut regs, 0x44);
        assert_eq!(cgram.memory[0x01], 0x4433);
    }

    /// word_addr must wrap from 0xFF to 0x00 after a complete write at address 0xFF.
    #[test]
    fn test_write_data_word_addr_wraps() {
        let mut cgram = CGRAM::new();
        let mut regs = make_regs();

        cgram.write_addr(&mut regs, 0xFF);
        cgram.write_data(&mut regs, 0x12);
        cgram.write_data(&mut regs, 0x34);
        // After write at 0xFF, addr wraps to 0x00
        cgram.write_data(&mut regs, 0xAA);
        cgram.write_data(&mut regs, 0x55);
        assert_eq!(cgram.memory[0x00], 0x55AA);
    }

    /// Sequential writes across multiple words must not corrupt adjacent entries.
    #[test]
    fn test_write_data_sequential_words() {
        let mut cgram = CGRAM::new();
        let mut regs = make_regs();

        for i in 0u8..4 {
            cgram.write_data(&mut regs, i); // lo
            cgram.write_data(&mut regs, i + 0x10); // hi (bit 7 clear, no masking effect)
        }
        assert_eq!(cgram.memory[0x00], 0x1000);
        assert_eq!(cgram.memory[0x01], 0x1101);
        assert_eq!(cgram.memory[0x02], 0x1202);
        assert_eq!(cgram.memory[0x03], 0x1303);
    }

    // ============================================================
    // read_data ($213B)
    // ============================================================

    /// Low phase returns (lo, 0xFF); High phase returns (hi, 0x7F) since bit 7
    /// is not driven by CGRAM (it is PPU2 open bus, filled by the caller).
    /// word_addr increments only after the High phase read.
    #[test]
    fn test_read_data() {
        let mut cgram = CGRAM::new();
        let mut regs = make_regs();
        cgram.memory[0x00] = 0x1234;
        cgram.memory[0x01] = 0x2222;

        // Low phase: returns lo byte fully driven, addr stays
        let (lo, mask) = cgram.read_data(&mut regs);
        assert_eq!(lo, 0x34);
        assert_eq!(mask, 0xFF);

        // High phase: returns hi byte, bit 7 not driven
        let (hi, mask) = cgram.read_data(&mut regs);
        assert_eq!(hi, 0x12); // hi of 0x1234
        assert_eq!(mask, 0x7F);

        // addr incremented to 0x01 after High phase
        let (lo1, _) = cgram.read_data(&mut regs);
        assert_eq!(lo1, 0x22);
    }

    /// The high-byte read drives only bits 0-6: bit 7 is left to the PPU2 open bus.
    #[test]
    fn test_read_data_high_byte_masks_bit7() {
        let mut cgram = CGRAM::new();
        let mut regs = make_regs();
        cgram.memory[0x00] = 0x7F00; // hi = 0x7F

        let (_lo, lo_mask) = cgram.read_data(&mut regs); // Low phase
        let (hi, hi_mask) = cgram.read_data(&mut regs);
        assert_eq!(lo_mask, 0xFF);
        assert_eq!(hi_mask, 0x7F);
        assert_eq!(hi & hi_mask, 0x7F);
    }

    /// word_addr must wrap from 0xFF to 0x00 after a complete read at address 0xFF.
    #[test]
    fn test_read_data_word_addr_wraps() {
        let mut cgram = CGRAM::new();
        let mut regs = make_regs();
        cgram.write_addr(&mut regs, 0xFF);
        cgram.memory[0xFF] = 0x1234;
        cgram.memory[0x00] = 0x5678;

        let _ = cgram.read_data(&mut regs);
        let _ = cgram.read_data(&mut regs); // addr wraps to 0x00
        let (lo_next, _) = cgram.read_data(&mut regs);
        assert_eq!(lo_next, 0x78);
    }

    // ============================================================
    // read helper
    // ============================================================

    /// read() returns the raw 16-bit word at the given index with no side effects
    /// on word_addr or byte_phase.
    #[test]
    fn test_read_helper() {
        let mut cgram = CGRAM::new();
        let mut regs = make_regs();
        cgram.memory[0x10] = 0xBEEF;
        assert_eq!(cgram.read(0x10), 0xBEEF);

        // No side effects: subsequent write pair still targets the address set by write_addr
        cgram.write_addr(&mut regs, 0x05);
        let _ = cgram.read(0x05);
        cgram.write_data(&mut regs, 0xAB);
        cgram.write_data(&mut regs, 0x3F);
        assert_eq!(cgram.memory[0x05], 0x3FAB);
    }

    // ============================================================
    // Round-trip
    // ============================================================

    /// A value written via write_data must be recoverable via read_data at the same address.
    #[test]
    fn test_round_trip_write_then_read() {
        let mut cgram = CGRAM::new();
        let mut regs = make_regs();

        cgram.write_addr(&mut regs, 0x20);
        cgram.write_data(&mut regs, 0x56);
        cgram.write_data(&mut regs, 0x3A); // bit 7 clear

        cgram.write_addr(&mut regs, 0x20);
        let (lo, _) = cgram.read_data(&mut regs);
        let (hi, mask) = cgram.read_data(&mut regs);
        assert_eq!(lo, 0x56);
        assert_eq!(hi & mask, 0x3A);
    }
}
