use crate::constants::VRAM_SIZE;
use crate::registers::PPURegisters;
use common::u16_split::U16Split;

pub type RawVRAM = [u16; VRAM_SIZE / 2];

pub struct VRAM {
    pub memory: Box<RawVRAM>, // VRAM stored as u16 words
    pub vram_latch: u16, // word latch for reads
}

impl VRAM {
    pub fn new() -> Self {
        Self {
            memory: Box::new([0; _]),
            vram_latch: 0,
        }
    }

    // ============================================================
    // Address increment logic
    // ============================================================

    pub fn increment_amount(regs: &PPURegisters) -> u16 {
        match regs.vmain & 0b11 {
            0 => 1,
            1 => 32,
            2 | 3 => 128,
            _ => unreachable!(),
        }
    }

    pub fn increment_after_low(regs: &PPURegisters) -> bool {
        (regs.vmain & 0x80) == 0
    }

    pub fn increment_after_high(regs: &PPURegisters) -> bool {
        (regs.vmain & 0x80) != 0
    }

    fn increment_vmadd(regs: &mut PPURegisters) {
        regs.vmadd = regs.vmadd.wrapping_add(Self::increment_amount(regs)) & 0x7FFF;
    }

    // ============================================================
    // VMADD ($2116 / $2117)
    // ============================================================

    pub fn write_vmadd(&mut self, PPURegisters { vmadd, .. }: &mut PPURegisters, addr: u16) {
        *vmadd = addr & 0x7FFF;
        self.load_latch(*vmadd);
    }

    pub fn write_vmadd_low(&mut self, PPURegisters { vmadd, .. }: &mut PPURegisters, value: u8) {
        *vmadd.lo_mut() = value;
        self.load_latch(*vmadd);
    }

    pub fn write_vmadd_high(&mut self, PPURegisters { vmadd, .. }: &mut PPURegisters, value: u8) {
        *vmadd.hi_mut() = value & 0x7F;
        self.load_latch(*vmadd);
    }

    // ============================================================
    // VRAM DATA WRITE ($2118 / $2119)
    // ============================================================

    pub fn write_vmdata(&mut self, regs: &mut PPURegisters, value: u16) {
        let addr = (regs.vmadd & 0x7FFF) as usize;
        *self.memory[addr].lo_mut() = *value.lo();

        if Self::increment_after_low(regs) {
            Self::increment_vmadd(regs);
        }

        let addr = (regs.vmadd & 0x7FFF) as usize;
        *self.memory[addr].hi_mut() = *value.hi();

        if Self::increment_after_high(regs) {
            Self::increment_vmadd(regs);
        }
    }

    pub fn write_vmdatal(&mut self, regs: &mut PPURegisters, value: u8) {
        let addr = (regs.vmadd & 0x7FFF) as usize;
        *self.memory[addr].lo_mut() = value;

        if Self::increment_after_low(regs) {
            Self::increment_vmadd(regs);
        }
    }

    pub fn write_vmdatah(&mut self, regs: &mut PPURegisters, value: u8) {
        let addr = (regs.vmadd & 0x7FFF) as usize;
        *self.memory[addr].hi_mut() = value;

        if Self::increment_after_high(regs) {
            Self::increment_vmadd(regs);
        }
    }

    // ============================================================
    // VRAM DATA READ ($2139 / $213A)
    // ============================================================

    pub fn read_vmdata(&mut self, regs: &mut PPURegisters) -> u16 {
        let lo = *self.vram_latch.lo();

        if Self::increment_after_low(regs) {
            Self::increment_vmadd(regs);
            self.load_latch(regs.vmadd);
        }

        let hi = *self.vram_latch.hi();

        if Self::increment_after_high(regs) {
            Self::increment_vmadd(regs);
            self.load_latch(regs.vmadd);
        }

        (lo as u16) | ((hi as u16) << 8)
    }

    pub fn read_vmdatal(&mut self, regs: &mut PPURegisters) -> u8 {
        let value = *self.vram_latch.lo();

        if Self::increment_after_low(regs) {
            Self::increment_vmadd(regs);
            self.load_latch(regs.vmadd);
        }

        value
    }

    pub fn read_vmdatah(&mut self, regs: &mut PPURegisters) -> u8 {
        let value = *self.vram_latch.hi();

        if Self::increment_after_high(regs) {
            Self::increment_vmadd(regs);
            self.load_latch(regs.vmadd);
        }

        value
    }

    // ============================================================
    // Helpers
    // ============================================================

    pub fn load_latch(&mut self, vmadd: u16) {
        self.vram_latch = self.memory[(vmadd & 0x7FFF) as usize];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Helpers
    // ============================================================

    // vmain = 0x00 -> increment by 1, increment after low byte write/read
    const VMAIN_INC1_AFTER_LOW: u8 = 0x00;
    // vmain = 0x01 -> increment by 32, increment after low byte write/read
    const VMAIN_INC32_AFTER_LOW: u8 = 0x01;
    // vmain = 0x02 -> increment by 128, increment after low byte write/read
    const VMAIN_INC128_AFTER_LOW: u8 = 0x02;
    // vmain = 0x80 -> increment by 1, increment after high byte write/read
    const VMAIN_INC1_AFTER_HIGH: u8 = 0x80;
    // vmain = 0x81 -> increment by 32, increment after high byte write/read
    const VMAIN_INC32_AFTER_HIGH: u8 = 0x81;
    // vmain = 0x83 -> increment by 128, increment after high byte write/read
    const VMAIN_INC128_AFTER_HIGH: u8 = 0x83;

    fn make_regs(vmain: u8, vmadd: u16) -> PPURegisters {
        let mut regs = PPURegisters::new();
        regs.vmain = vmain;
        regs.vmadd = vmadd;
        regs
    }

    // ============================================================
    // VRAM::new
    // ============================================================

    /// A freshly created VRAM must have all memory words zeroed and the latch at zero.
    #[test]
    fn test_new_memory_zeroed() {
        let vram = VRAM::new();
        assert!(vram.memory.iter().all(|&w| w == 0));
        assert_eq!(vram.vram_latch, 0);
    }

    // ============================================================
    // increment_amount
    // ============================================================

    /// vmain bits[1:0] select the increment: 0b00->1, 0b01->32, 0b10->128, 0b11->128.
    /// Upper bits must be ignored.
    #[test]
    fn test_increment_amount() {
        assert_eq!(VRAM::increment_amount(&make_regs(0b00, 0)), 1);
        assert_eq!(VRAM::increment_amount(&make_regs(0b01, 0)), 32);
        assert_eq!(VRAM::increment_amount(&make_regs(0b10, 0)), 128);
        assert_eq!(VRAM::increment_amount(&make_regs(0b11, 0)), 128);
        // Upper bits masked out
        assert_eq!(VRAM::increment_amount(&make_regs(0xFC, 0)), 1);   // 0xFC & 0b11 == 0b00
        assert_eq!(VRAM::increment_amount(&make_regs(0x85, 0)), 32);  // 0x85 & 0b11 == 0b01
        assert_eq!(VRAM::increment_amount(&make_regs(0x86, 0)), 128); // 0x86 & 0b11 == 0b10
    }

    // ============================================================
    // increment_after_low / increment_after_high
    // ============================================================

    /// increment_after_low is true iff bit 7 of vmain is clear; increment_after_high is the inverse.
    #[test]
    fn test_increment_after_low_and_high() {
        // bit 7 clear -> after low
        assert!(VRAM::increment_after_low(&make_regs(0x00, 0)));
        assert!(VRAM::increment_after_low(&make_regs(0x7F, 0)));
        assert!(!VRAM::increment_after_high(&make_regs(0x00, 0)));
        assert!(!VRAM::increment_after_high(&make_regs(0x7F, 0)));
        // bit 7 set -> after high
        assert!(!VRAM::increment_after_low(&make_regs(0x80, 0)));
        assert!(!VRAM::increment_after_low(&make_regs(0xFF, 0)));
        assert!(VRAM::increment_after_high(&make_regs(0x80, 0)));
        assert!(VRAM::increment_after_high(&make_regs(0xFF, 0)));
    }

    // ============================================================
    // write_vmadd ($2116 / $2117)
    // ============================================================
 
    /// write_vmadd_low updates the low byte of vmadd and reloads the latch;
    /// write_vmadd_high strips bit 7, updates the high byte, reloads the latch,
    /// and must not touch the low byte.
    #[test]
    fn test_write_vmadd_low_and_high_separate() {
        let mut vram = VRAM::new();
        vram.memory[0x0005] = 0xABCD;
        vram.memory[0x0142] = 0x1234; // address resulting from vmadd=0x0042, high write 0x81 & 0x7F = 0x01
 
        // Low byte write
        let mut regs = make_regs(0, 0x0000);
        vram.write_vmadd_low(&mut regs, 0x05);
        assert_eq!(regs.vmadd, 0x0005);
        assert_eq!(vram.vram_latch, 0xABCD);
 
        // High byte write: bit 7 masked, low byte untouched
        let mut regs = make_regs(0, 0x0042);
        vram.write_vmadd_high(&mut regs, 0x81);
        assert_eq!(regs.vmadd, 0x0142);
        assert_eq!(vram.vram_latch, 0x1234);
    }

    /// write_vmadd sets both bytes at once, masks bit 15, reloads the latch,
    /// and produces the same state as separate low/high writes.
    #[test]
    fn test_write_vmadd_combined() {
        let mut vram = VRAM::new();
        vram.memory[0x0123] = 0xDEAD;
        vram.memory[0x0245] = 0x1234;

        // Basic combined write
        let mut regs = make_regs(0, 0x0000);
        vram.write_vmadd(&mut regs, 0x0123);
        assert_eq!(regs.vmadd, 0x0123);
        assert_eq!(vram.vram_latch, 0xDEAD);

        // Bit 15 must be masked
        vram.write_vmadd(&mut regs, 0xFF00);
        assert_eq!(regs.vmadd, 0x7F00);

        // Equivalent to separate writes
        let mut regs_a = make_regs(0, 0x0000);
        vram.write_vmadd(&mut regs_a, 0x0245);
        let mut regs_b = make_regs(0, 0x0000);
        vram.write_vmadd_low(&mut regs_b, 0x45);
        vram.write_vmadd_high(&mut regs_b, 0x02);
        assert_eq!(regs_a.vmadd, regs_b.vmadd);
        assert_eq!(vram.vram_latch, 0x1234);

        // Address zero
        let mut regs = make_regs(0, 0x7FFF);
        vram.memory[0x0000] = 0xBEEF;
        vram.write_vmadd(&mut regs, 0x0000);
        assert_eq!(regs.vmadd, 0x0000);
        assert_eq!(vram.vram_latch, 0xBEEF);
    }

    // ============================================================
    // write_vmdatal ($2118)
    // ============================================================

    /// write_vmdatal updates the low byte at the current address.
    /// It increments after write iff vmain bit7=0, by the amount encoded in bits[1:0].
    #[test]
    fn test_write_vmdatal() {
        let mut vram = VRAM::new();

        // Writes correct byte
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x0003);
        vram.write_vmdatal(&mut regs, 0xBE);
        assert_eq!(vram.memory[0x0003] & 0x00FF, 0xBE);

        // Increments by 1 after low write (bit7=0)
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x0000);
        vram.write_vmdatal(&mut regs, 0xFF);
        assert_eq!(regs.vmadd, 0x0001);

        // No increment when bit7=1
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0000);
        vram.write_vmdatal(&mut regs, 0xFF);
        assert_eq!(regs.vmadd, 0x0000);

        // Increment by 32 and 128
        let mut regs = make_regs(VMAIN_INC32_AFTER_LOW, 0x0000);
        vram.write_vmdatal(&mut regs, 0x00);
        assert_eq!(regs.vmadd, 32);

        let mut regs = make_regs(VMAIN_INC128_AFTER_LOW, 0x0000);
        vram.write_vmdatal(&mut regs, 0x00);
        assert_eq!(regs.vmadd, 128);
    }

    // ============================================================
    // write_vmdatah ($2119)
    // ============================================================

    /// write_vmdatah updates the high byte at the current address.
    /// It increments after write iff vmain bit7=1, by the amount encoded in bits[1:0].
    #[test]
    fn test_write_vmdatah() {
        let mut vram = VRAM::new();

        // Writes correct byte
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0003);
        vram.write_vmdatah(&mut regs, 0xEF);
        assert_eq!((vram.memory[0x0003] >> 8) as u8, 0xEF);

        // Increments by 1 after high write (bit7=1)
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0000);
        vram.write_vmdatah(&mut regs, 0xFF);
        assert_eq!(regs.vmadd, 0x0001);

        // No increment when bit7=0
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x0000);
        vram.write_vmdatah(&mut regs, 0xFF);
        assert_eq!(regs.vmadd, 0x0000);

        // Increment by 32 and 128
        let mut regs = make_regs(VMAIN_INC32_AFTER_HIGH, 0x0000);
        vram.write_vmdatah(&mut regs, 0x00);
        assert_eq!(regs.vmadd, 32);

        let mut regs = make_regs(VMAIN_INC128_AFTER_HIGH, 0x0000);
        vram.write_vmdatah(&mut regs, 0x00);
        assert_eq!(regs.vmadd, 128);
    }

    /// A paired low+high write (bit7=1 mode) must produce the expected full 16-bit word
    /// and increment exactly once after the high write.
    #[test]
    fn test_write_low_then_high_builds_full_word() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0000);

        vram.write_vmdatal(&mut regs, 0xCD);
        vram.write_vmdatah(&mut regs, 0xAB);

        assert_eq!(vram.memory[0x0000], 0xABCD);
        assert_eq!(regs.vmadd, 0x0001);
    }

    // ============================================================
    // write_vmdata lo+hi combined ($2118 / $2119)
    // ============================================================

    /// write_vmdata must produce the same result as separate low/high writes in both modes,
    /// and advance the address by the configured increment amount.
    #[test]
    fn test_write_vmdata() {
        let mut vram = VRAM::new();

        // High mode: both bytes go to the same word, address increments once after high
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0005);
        vram.write_vmdata(&mut regs, 0xABCD);
        assert_eq!(vram.memory[0x0005], 0xABCD);
        assert_eq!(regs.vmadd, 0x0006);

        // Equivalent to separate writes (high mode)
        let mut vram2 = VRAM::new();
        let mut regs_a = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0000);
        vram2.write_vmdata(&mut regs_a, 0x1234);
        let mut regs_b = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0000);
        let mut vram3 = VRAM::new();
        vram3.write_vmdatal(&mut regs_b, 0x34);
        vram3.write_vmdatah(&mut regs_b, 0x12);
        assert_eq!(vram2.memory[0x0000], vram3.memory[0x0000]);
        assert_eq!(regs_a.vmadd, regs_b.vmadd);

        // Low mode: lo goes to word 0, hi goes to word 1 (address increments after lo)
        let mut vram4 = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x0000);
        vram4.write_vmdata(&mut regs, 0xABCD);
        assert_eq!(vram4.memory[0x0000] & 0x00FF, 0xCD);
        assert_eq!((vram4.memory[0x0001] >> 8) as u8, 0xAB);
        assert_eq!(regs.vmadd, 0x0001);

        // Increment by 32 and 128 (high mode)
        let mut regs = make_regs(VMAIN_INC32_AFTER_HIGH, 0x0000);
        vram.write_vmdata(&mut regs, 0x0000);
        assert_eq!(regs.vmadd, 32);

        let mut regs = make_regs(VMAIN_INC128_AFTER_HIGH, 0x0000);
        vram.write_vmdata(&mut regs, 0x0000);
        assert_eq!(regs.vmadd, 128);
    }

    // ============================================================
    // read_vmdatal ($2139)
    // ============================================================

    /// read_vmdatal returns the latched low byte.
    /// In low mode (bit7=0) it increments and refreshes the latch; in high mode it does not.
    #[test]
    fn test_read_vmdatal() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0x1234;
        vram.memory[0x0001] = 0x5678;

        // Returns lo byte of latch
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x0000);
        vram.load_latch(regs.vmadd);
        assert_eq!(vram.read_vmdatal(&mut regs), 0x34);

        // After read: address incremented, latch refreshed with next word
        assert_eq!(regs.vmadd, 0x0001);
        assert_eq!(vram.vram_latch, 0x5678);

        // No increment in high mode
        vram.memory[0x0000] = 0xBEEF;
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0000);
        vram.load_latch(regs.vmadd);
        let _ = vram.read_vmdatal(&mut regs);
        assert_eq!(regs.vmadd, 0x0000);
        assert_eq!(vram.vram_latch, 0xBEEF);
    }

    // ============================================================
    // read_vmdatah ($213A)
    // ============================================================

    /// read_vmdatah returns the latched high byte.
    /// In high mode (bit7=1) it increments and refreshes the latch; in low mode it does not.
    #[test]
    fn test_read_vmdatah() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0xABCD;
        vram.memory[0x0001] = 0xDEAD;

        // Returns hi byte of latch
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0000);
        vram.load_latch(regs.vmadd);
        assert_eq!(vram.read_vmdatah(&mut regs), 0xAB);

        // After read: address incremented, latch refreshed
        assert_eq!(regs.vmadd, 0x0001);
        assert_eq!(vram.vram_latch, 0xDEAD);

        // No increment in low mode
        vram.memory[0x0000] = 0xCAFE;
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x0000);
        vram.load_latch(regs.vmadd);
        let _ = vram.read_vmdatah(&mut regs);
        assert_eq!(regs.vmadd, 0x0000);
        assert_eq!(vram.vram_latch, 0xCAFE);
    }

    // ============================================================
    // read_vmdata lo+hi combined ($2139 / $213A)
    // ============================================================

    /// read_vmdata must produce the same result as separate low/high reads in both modes,
    /// and advance the address by the configured increment amount.
    #[test]
    fn test_read_vmdata() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0xABCD;
        vram.memory[0x0001] = 0x1234;

        // High mode: returns full word, increments once after hi read
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0000);
        vram.load_latch(regs.vmadd);
        assert_eq!(vram.read_vmdata(&mut regs), 0xABCD);
        assert_eq!(regs.vmadd, 0x0001);
        assert_eq!(vram.vram_latch, 0x1234);

        // Equivalent to separate reads (high mode)
        vram.memory[0x0000] = 0xDEAD;
        let mut regs_a = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0000);
        vram.load_latch(regs_a.vmadd);
        let word = vram.read_vmdata(&mut regs_a);
        let mut regs_b = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0000);
        vram.load_latch(regs_b.vmadd);
        let lo = vram.read_vmdatal(&mut regs_b);
        let hi = vram.read_vmdatah(&mut regs_b);
        assert_eq!(word, (lo as u16) | ((hi as u16) << 8));
        assert_eq!(regs_a.vmadd, regs_b.vmadd);

        // Equivalent to separate reads (low mode)
        vram.memory[0x0000] = 0x00CD;
        vram.memory[0x0001] = 0xAB00;
        let mut regs_a = make_regs(VMAIN_INC1_AFTER_LOW, 0x0000);
        vram.load_latch(regs_a.vmadd);
        let word = vram.read_vmdata(&mut regs_a);
        let mut regs_b = make_regs(VMAIN_INC1_AFTER_LOW, 0x0000);
        vram.load_latch(regs_b.vmadd);
        let lo = vram.read_vmdatal(&mut regs_b);
        let hi = vram.read_vmdatah(&mut regs_b);
        assert_eq!(word, (lo as u16) | ((hi as u16) << 8));
        assert_eq!(regs_a.vmadd, regs_b.vmadd);

        // Increment by 32 and 128
        let mut regs = make_regs(VMAIN_INC32_AFTER_HIGH, 0x0000);
        vram.load_latch(regs.vmadd);
        vram.read_vmdata(&mut regs);
        assert_eq!(regs.vmadd, 32);

        let mut regs = make_regs(VMAIN_INC128_AFTER_HIGH, 0x0000);
        vram.load_latch(regs.vmadd);
        vram.read_vmdata(&mut regs);
        assert_eq!(regs.vmadd, 128);
    }

    // ============================================================
    // load_latch
    // ============================================================

    /// load_latch copies the word at the given address into vram_latch.
    #[test]
    fn test_load_latch() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0x1111;
        vram.memory[0x0200] = 0xF00D;

        vram.load_latch(0x0000);
        assert_eq!(vram.vram_latch, 0x1111);

        vram.load_latch(0x0200);
        assert_eq!(vram.vram_latch, 0xF00D);
    }

    // ============================================================
    // Address wrap-around
    // ============================================================

    /// The effective VRAM address is 15-bit (0x0000–0x7FFF);
    /// incrementing past 0x7FFF must wrap to 0x0000.
    #[test]
    fn test_address_wraps_at_0x7fff() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x7FFF);

        vram.write_vmdatal(&mut regs, 0xAA);

        assert_eq!(regs.vmadd, 0x0000);
    }

    // ============================================================
    // Round-trip write / read
    // ============================================================

    /// Writing a full 16-bit word and reading it back must produce the original value.
    #[test]
    fn test_round_trip_write_then_read() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0010);

        vram.write_vmdatal(&mut regs, 0x56);
        vram.write_vmdatah(&mut regs, 0x78);
        vram.write_vmadd(&mut regs, 0x0010);

        assert_eq!(vram.read_vmdatal(&mut regs), 0x56);
        assert_eq!(vram.read_vmdatah(&mut regs), 0x78);
    }

    /// write_vmdata + read_vmdata round-trip must return the original word.
    #[test]
    fn test_write_vmdata_read_vmdata_round_trip() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0010);

        vram.write_vmdata(&mut regs, 0xCAFE);
        vram.write_vmadd(&mut regs, 0x0010);

        assert_eq!(vram.read_vmdata(&mut regs), 0xCAFE);
    }

    /// Sequential writes at incrementing addresses must not corrupt adjacent words.
    #[test]
    fn test_sequential_writes_dont_corrupt_neighbours() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x0000);

        vram.write_vmdatal(&mut regs, 0xBB);
        vram.write_vmdatah(&mut regs, 0xAA);
        vram.write_vmdatal(&mut regs, 0xDD);
        vram.write_vmdatah(&mut regs, 0xCC);

        assert_eq!(vram.memory[0x0000], 0xAABB);
        assert_eq!(vram.memory[0x0001], 0xCCDD);
    }
}
