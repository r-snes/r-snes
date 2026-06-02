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

    fn vmadd(vmaddl: u8, vmaddh: u8) -> u16 {
        (vmaddl as u16) | ((vmaddh as u16 & 0x7F) << 8)
    }

    fn set_vmadd(addr: u16, vmaddl: &mut u8, vmaddh: &mut u8) {
        *vmaddl = (addr & 0xFF) as u8;
        *vmaddh = ((addr >> 8) & 0x7F) as u8;
    }

    pub fn increment_amount(vmain: u8) -> u16 {
        match vmain & 0b11 {
            0 => 1,
            1 => 32,
            2 | 3 => 128,
            _ => unreachable!(),
        }
    }

    pub fn increment_after_low(vmain: u8) -> bool {
        (vmain & 0x80) == 0
    }

    pub fn increment_after_high(vmain: u8) -> bool {
        (vmain & 0x80) != 0
    }

    fn increment_vmadd(vmain: u8, vmaddl: &mut u8, vmaddh: &mut u8) {
        let addr = (Self::vmadd(*vmaddl, *vmaddh) + Self::increment_amount(vmain)) & 0x7FFF;
        Self::set_vmadd(addr, vmaddl, vmaddh);
    }

    // ============================================================
    // VMADD ($2116 / $2117)
    // ============================================================
    
    pub fn write_vmadd(&mut self, PPURegisters { vmaddl, vmaddh, .. }: &mut PPURegisters, addr: u16) {
        *vmaddl = *addr.lo();
        self.load_latch(*vmaddl, *vmaddh);

        *vmaddh = *addr.hi() & 0x7F;
        self.load_latch(*vmaddl, *vmaddh);
    }

    pub fn write_vmadd_low(&mut self, PPURegisters { vmaddl, vmaddh, .. }: &mut PPURegisters, value: u8) {
        *vmaddl = value;
        self.load_latch(*vmaddl, *vmaddh);
    }

    pub fn write_vmadd_high(&mut self, PPURegisters { vmaddl, vmaddh, .. }: &mut PPURegisters, value: u8) {
        *vmaddh = value & 0x7F;
        self.load_latch(*vmaddl, *vmaddh);
    }

    // ============================================================
    // VRAM DATA WRITE ($2118 / $2119)
    // ============================================================

    pub fn write_vmdata(&mut self, PPURegisters { vmain, vmaddl, vmaddh, .. }: &mut PPURegisters, value: u16) {
        let addr = Self::vmadd(*vmaddl, *vmaddh) as usize;
        *self.memory[addr].lo_mut() = *value.lo();

        if Self::increment_after_low(*vmain) {
            Self::increment_vmadd(*vmain, vmaddl, vmaddh);
        }

        let addr = Self::vmadd(*vmaddl, *vmaddh) as usize;
        *self.memory[addr].hi_mut() = *value.hi();

        if Self::increment_after_high(*vmain) {
            Self::increment_vmadd(*vmain, vmaddl, vmaddh);
        }
    }

    pub fn write_vmdatal(&mut self, PPURegisters { vmain, vmaddl, vmaddh, .. }: &mut PPURegisters, value: u8) {
        let addr = Self::vmadd(*vmaddl, *vmaddh) as usize;
        *self.memory[addr].lo_mut() = value;

        if Self::increment_after_low(*vmain) {
            Self::increment_vmadd(*vmain, vmaddl, vmaddh);
        }
    }

    pub fn write_vmdatah(&mut self, PPURegisters { vmain, vmaddl, vmaddh, .. }: &mut PPURegisters, value: u8) {
        let addr = Self::vmadd(*vmaddl, *vmaddh) as usize;
        *self.memory[addr].hi_mut() = value;

        if Self::increment_after_high(*vmain) {
            Self::increment_vmadd(*vmain, vmaddl, vmaddh);
        }
    }

    // ============================================================
    // VRAM DATA READ ($2139 / $213A)
    // ============================================================

    pub fn read_vmdata(&mut self, PPURegisters { vmain, vmaddl, vmaddh, .. }: &mut PPURegisters) -> u16 {
        let lo = *self.vram_latch.lo();

        if Self::increment_after_low(*vmain) {
            Self::increment_vmadd(*vmain, vmaddl, vmaddh);
            self.load_latch(*vmaddl, *vmaddh);
        }

        let hi = *self.vram_latch.hi();

        if Self::increment_after_high(*vmain) {
            Self::increment_vmadd(*vmain, vmaddl, vmaddh);
            self.load_latch(*vmaddl, *vmaddh);
        }

        (lo as u16) | ((hi as u16) << 8)
    }

    pub fn read_vmdatal(&mut self, PPURegisters { vmain, vmaddl, vmaddh, .. }: &mut PPURegisters) -> u8 {
        let value = *self.vram_latch.lo();

        if Self::increment_after_low(*vmain) {
            Self::increment_vmadd(*vmain, vmaddl, vmaddh);
            self.load_latch(*vmaddl, *vmaddh);
        }

        value
    }

    pub fn read_vmdatah(&mut self, PPURegisters { vmain, vmaddl, vmaddh, .. }: &mut PPURegisters) -> u8 {
        let value = *self.vram_latch.hi();

        if Self::increment_after_high(*vmain) {
            Self::increment_vmadd(*vmain, vmaddl, vmaddh);
            self.load_latch(*vmaddl, *vmaddh);
        }

        value
    }

    // ============================================================
    // Helpers
    // ============================================================

    pub fn load_latch(&mut self, vmaddl: u8, vmaddh: u8) {
        self.vram_latch = self.memory[Self::vmadd(vmaddl, vmaddh) as usize];
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

    fn make_regs(vmain: u8, vmaddl: u8, vmaddh: u8) -> PPURegisters {
        let mut regs = PPURegisters::new();
        regs.vmain = vmain;
        regs.vmaddl = vmaddl;
        regs.vmaddh = vmaddh;
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

    /// vmain bits[1:0] == 0b00 -> increment amount must be 1.
    #[test]
    fn test_increment_amount_1() {
        assert_eq!(VRAM::increment_amount(0b00), 1);
    }

    /// vmain bits[1:0] == 0b01 -> increment amount must be 32.
    #[test]
    fn test_increment_amount_32() {
        assert_eq!(VRAM::increment_amount(0b01), 32);
    }

    /// vmain bits[1:0] == 0b10 -> increment amount must be 128.
    #[test]
    fn test_increment_amount_128_variant2() {
        assert_eq!(VRAM::increment_amount(0b10), 128);
    }

    /// vmain bits[1:0] == 0b11 -> increment amount must also be 128.
    #[test]
    fn test_increment_amount_128_variant3() {
        assert_eq!(VRAM::increment_amount(0b11), 128);
    }

    /// Upper bits of vmain must be masked out; only bits[1:0] matter.
    #[test]
    fn test_increment_amount_ignores_upper_bits() {
        // 0xFC & 0b11 == 0b00 -> 1
        assert_eq!(VRAM::increment_amount(0xFC), 1);
        // 0x85 & 0b11 == 0b01 -> 32
        assert_eq!(VRAM::increment_amount(0x85), 32);
        // 0x86 & 0b11 == 0b10 -> 128
        assert_eq!(VRAM::increment_amount(0x86), 128);
    }

    // ============================================================
    // increment_after_low / increment_after_high
    // ============================================================

    /// When bit 7 of vmain is 0, increment must occur after the low byte access.
    #[test]
    fn test_increment_after_low_true_when_bit7_clear() {
        assert!(VRAM::increment_after_low(0x00));
        assert!(VRAM::increment_after_low(0x01));
        assert!(VRAM::increment_after_low(0x7F));
    }

    /// When bit 7 of vmain is 1, increment must NOT occur after the low byte access.
    #[test]
    fn test_increment_after_low_false_when_bit7_set() {
        assert!(!VRAM::increment_after_low(0x80));
        assert!(!VRAM::increment_after_low(0xFF));
    }

    /// When bit 7 of vmain is 1, increment must occur after the high byte access.
    #[test]
    fn test_increment_after_high_true_when_bit7_set() {
        assert!(VRAM::increment_after_high(0x80));
        assert!(VRAM::increment_after_high(0xFF));
    }

    /// When bit 7 of vmain is 0, increment must NOT occur after the high byte access.
    #[test]
    fn test_increment_after_high_false_when_bit7_clear() {
        assert!(!VRAM::increment_after_high(0x00));
        assert!(!VRAM::increment_after_high(0x7F));
    }

    // ============================================================
    // write_vmadd ($2116 / $2117)
    // ============================================================

    /// Writing to $2116 (VMADDL) must update vmaddl and reload the latch from the new address.
    #[test]
    fn test_write_vmadd_low_updates_register_and_latch() {
        let mut vram = VRAM::new();
        // Pre-load a known word at address 0x0005
        vram.memory[0x0005] = 0xABCD;
        let mut regs = make_regs(0, 0x00, 0x00);
 
        vram.write_vmadd_low(&mut regs, 0x05);
 
        assert_eq!(regs.vmaddl, 0x05);
        assert_eq!(regs.vmaddh, 0x00);
        assert_eq!(vram.vram_latch, 0xABCD);
    }

    /// Writing to $2117 (VMADDH) must strip bit 7, update vmaddh, and reload the latch.
    #[test]
    fn test_write_vmadd_high_masks_bit7_and_reloads_latch() {
        let mut vram = VRAM::new();
        // Address 0x0100 (vmaddh=0x01, vmaddl=0x00)
        vram.memory[0x0100] = 0x1234;
        let mut regs = make_regs(0, 0x00, 0x00);

        // Pass 0x81 – bit 7 must be masked off, resulting in vmaddh = 0x01 
        vram.write_vmadd_high(&mut regs, 0x81);

        assert_eq!(regs.vmaddh, 0x01);
        assert_eq!(vram.vram_latch, 0x1234);
    }

    /// write_vmadd_high must not modify vmaddl.
    #[test]
    fn test_write_vmadd_high_does_not_touch_vmaddl() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(0, 0x42, 0x00);
 
        vram.write_vmadd_high(&mut regs, 0x01);
 
        assert_eq!(regs.vmaddl, 0x42);
    }

    /// write_vmadd must set both vmaddl and vmaddh and reload the latch once.
    #[test]
    fn test_write_vmadd_sets_both_bytes_and_reloads_latch() {
        let mut vram = VRAM::new();
        vram.memory[0x0123] = 0xDEAD;
        let mut regs = make_regs(0, 0x00, 0x00);
 
        vram.write_vmadd(&mut regs, 0x0123);
 
        assert_eq!(regs.vmaddl, 0x23);
        assert_eq!(regs.vmaddh, 0x01);
        assert_eq!(vram.vram_latch, 0xDEAD);
    }

    /// write_vmadd must mask bit 7 of the high byte, exactly as write_vmadd_high does.
    #[test]
    fn test_write_vmadd_masks_high_bit7() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(0, 0x00, 0x00);
 
        vram.write_vmadd(&mut regs, 0xFF00);

        // 0xFF & 0x7F == 0x7F
        assert_eq!(regs.vmaddh, 0x7F);
    }

    /// write_vmadd must produce the same final state as write_vmadd_low followed by write_vmadd_high.
    #[test]
    fn test_write_vmadd_equivalent_to_separate_writes() {
        let mut vram_a = VRAM::new();
        let mut vram_b = VRAM::new();
        vram_a.memory[0x0245] = 0x1234;
        vram_b.memory[0x0245] = 0x1234;
 
        let mut regs_a = make_regs(0, 0x00, 0x00);
        vram_a.write_vmadd(&mut regs_a, 0x0245);
 
        let mut regs_b = make_regs(0, 0x00, 0x00);
        vram_b.write_vmadd_low(&mut regs_b, 0x45);
        vram_b.write_vmadd_high(&mut regs_b, 0x02);
 
        assert_eq!(regs_a.vmaddl, regs_b.vmaddl);
        assert_eq!(regs_a.vmaddh, regs_b.vmaddh);
        assert_eq!(vram_a.vram_latch, vram_b.vram_latch);
    }

    /// write_vmadd at address 0 must latch memory[0].
    #[test]
    fn test_write_vmadd_address_zero() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0xBEEF;
        let mut regs = make_regs(0, 0xFF, 0xFF);
 
        vram.write_vmadd(&mut regs, 0x0000);
 
        assert_eq!(regs.vmaddl, 0x00);
        assert_eq!(regs.vmaddh, 0x00);
        assert_eq!(vram.vram_latch, 0xBEEF);
    }

    // ============================================================
    // write_vmdatal ($2118)
    // ============================================================

    /// Writing to $2118 must update the low byte of the word at the current VRAM address.
    #[test]
    fn test_write_vmdatal_writes_low_byte() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x03, 0x00);
 
        vram.write_vmdatal(&mut regs, 0xBE);

        // The high byte should still be 0; only lo changed.
        assert_eq!(vram.memory[0x0003] & 0x00FF, 0xBE);
    }

    /// After a low byte write with vmain bit7=0, the address must increment by the configured amount.
    #[test]
    fn test_write_vmdatal_increments_address_after_low() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x00, 0x00);
 
        vram.write_vmdatal(&mut regs, 0xFF);

        // Address should have advanced by 1 -> vmaddl == 1
        assert_eq!(regs.vmaddl, 0x01);
        assert_eq!(regs.vmaddh, 0x00);
    }

    /// After a low byte write with vmain bit7=1, the address must NOT increment.
    #[test]
    fn test_write_vmdatal_no_increment_when_high_mode() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x00, 0x00);
 
        vram.write_vmdatal(&mut regs, 0xFF);
 
        assert_eq!(regs.vmaddl, 0x00);
        assert_eq!(regs.vmaddh, 0x00);
    }

    /// write_vmdatal with increment-by-32 must advance the address by 32.
    #[test]
    fn test_write_vmdatal_increment_by_32() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC32_AFTER_LOW, 0x00, 0x00);
 
        vram.write_vmdatal(&mut regs, 0x00);
 
        let addr = (regs.vmaddl as u16) | ((regs.vmaddh as u16) << 8);
        assert_eq!(addr, 32);
    }

    /// write_vmdatal with increment-by-128 must advance the address by 128.
    #[test]
    fn test_write_vmdatal_increment_by_128() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC128_AFTER_LOW, 0x00, 0x00);
 
        vram.write_vmdatal(&mut regs, 0x00);
 
        let addr = (regs.vmaddl as u16) | ((regs.vmaddh as u16) << 8);
        assert_eq!(addr, 128);
    }

    // ============================================================
    // write_vmdatah ($2119)
    // ============================================================

    /// Writing to $2119 must update the high byte of the word at the current VRAM address.
    #[test]
    fn test_write_vmdatah_writes_high_byte() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x03, 0x00);
 
        vram.write_vmdatah(&mut regs, 0xEF);
 
        assert_eq!((vram.memory[0x0003] >> 8) as u8, 0xEF);
    }

    /// After a high byte write with vmain bit7=1, the address must increment.
    #[test]
    fn test_write_vmdatah_increments_address_after_high() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x00, 0x00);
 
        vram.write_vmdatah(&mut regs, 0xFF);
 
        assert_eq!(regs.vmaddl, 0x01);
        assert_eq!(regs.vmaddh, 0x00);
    }

    /// After a high byte write with vmain bit7=0, the address must NOT increment.
    #[test]
    fn test_write_vmdatah_no_increment_when_low_mode() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x00, 0x00);
 
        vram.write_vmdatah(&mut regs, 0xFF);
 
        assert_eq!(regs.vmaddl, 0x00);
        assert_eq!(regs.vmaddh, 0x00);
    }

    /// write_vmdatah with increment-by-32 must advance the address by 32.
    #[test]
    fn test_write_vmdatah_increment_by_32() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC32_AFTER_HIGH, 0x00, 0x00);
 
        vram.write_vmdatah(&mut regs, 0x00);
 
        let addr = (regs.vmaddl as u16) | ((regs.vmaddh as u16) << 8);
        assert_eq!(addr, 32);
    }

    
    /// write_vmdatah with increment-by-128 must advance the address by 128.
    #[test]
    fn test_write_vmdatah_increment_by_128() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC128_AFTER_HIGH, 0x00, 0x00);

        vram.write_vmdatah(&mut regs, 0x00);

        let addr = (regs.vmaddl as u16) | ((regs.vmaddh as u16) << 8);
        assert_eq!(addr, 128);
    }

    /// A paired low+high write at the same address must produce the expected full 16-bit word.
    #[test]
    fn test_write_low_then_high_builds_full_word() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x00, 0x00);
 
        vram.write_vmdatal(&mut regs, 0xCD);
        vram.write_vmdatah(&mut regs, 0xAB);

        assert_eq!(vram.memory[0x0000], 0xABCD);
        // Address must have advanced exactly once (after the high write)
        assert_eq!(regs.vmaddl, 0x01);
    }

    // ============================================================
    // write_vmdata lo+hi combined ($2118 / $2119)
    // ============================================================

    /// write_vmdata with vmain bit7=1 must write both bytes to the same word address.
    #[test]
    fn test_write_vmdata_writes_full_word_in_high_mode() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x05, 0x00);
 
        vram.write_vmdata(&mut regs, 0xABCD);
 
        assert_eq!(vram.memory[0x0005], 0xABCD);
        assert_eq!(regs.vmaddl, 0x06);
        assert_eq!(regs.vmaddh, 0x00);
    }

    /// write_vmdata with vmain bit7=1 must produce the same result as write_vmdatal + write_vmdatah.
    #[test]
    fn test_write_vmdata_equivalent_to_separate_writes_high_mode() {
        let mut vram_a = VRAM::new();
        let mut vram_b = VRAM::new();
 
        let mut regs_a = make_regs(VMAIN_INC1_AFTER_HIGH, 0x00, 0x00);
        vram_a.write_vmdata(&mut regs_a, 0x1234);
 
        let mut regs_b = make_regs(VMAIN_INC1_AFTER_HIGH, 0x00, 0x00);
        vram_b.write_vmdatal(&mut regs_b, 0x34);
        vram_b.write_vmdatah(&mut regs_b, 0x12);
 
        assert_eq!(vram_a.memory[0x0000], vram_b.memory[0x0000]);
        assert_eq!(regs_a.vmaddl, regs_b.vmaddl);
        assert_eq!(regs_a.vmaddh, regs_b.vmaddh);
    }

    /// write_vmdata with vmain bit7=0 must write lo and hi to different words,
    /// because the address increments after the low byte write.
    #[test]
    fn test_write_vmdata_low_and_high_go_to_different_words_in_low_mode() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x00, 0x00);
 
        vram.write_vmdata(&mut regs, 0xABCD);
 
        assert_eq!(vram.memory[0x0000] & 0x00FF, 0xCD);
        assert_eq!((vram.memory[0x0001] >> 8) as u8, 0xAB);
        assert_eq!(regs.vmaddl, 0x01);
    }

    /// write_vmdata with vmain bit7=0 must produce the same result as separate low/high writes.
    #[test]
    fn test_write_vmdata_equivalent_to_separate_writes_low_mode() {
        let mut vram_a = VRAM::new();
        let mut vram_b = VRAM::new();
 
        let mut regs_a = make_regs(VMAIN_INC1_AFTER_LOW, 0x00, 0x00);
        vram_a.write_vmdata(&mut regs_a, 0x5678);
 
        let mut regs_b = make_regs(VMAIN_INC1_AFTER_LOW, 0x00, 0x00);
        vram_b.write_vmdatal(&mut regs_b, 0x78);
        vram_b.write_vmdatah(&mut regs_b, 0x56);
 
        assert_eq!(vram_a.memory[0x0000], vram_b.memory[0x0000]);
        assert_eq!(vram_a.memory[0x0001], vram_b.memory[0x0001]);
        assert_eq!(regs_a.vmaddl, regs_b.vmaddl);
        assert_eq!(regs_a.vmaddh, regs_b.vmaddh);
    }

    /// write_vmdata with increment-by-32 must advance the address by 32 after the high byte.
    #[test]
    fn test_write_vmdata_increment_by_32() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC32_AFTER_HIGH, 0x00, 0x00);

        vram.write_vmdata(&mut regs, 0x0000);

        let addr = (regs.vmaddl as u16) | ((regs.vmaddh as u16) << 8);
        assert_eq!(addr, 32);
    }

    /// write_vmdata with increment-by-128 must advance the address by 128 after the high byte.
    #[test]
    fn test_write_vmdata_increment_by_128() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC128_AFTER_HIGH, 0x00, 0x00);

        vram.write_vmdata(&mut regs, 0x0000);

        let addr = (regs.vmaddl as u16) | ((regs.vmaddh as u16) << 8);
        assert_eq!(addr, 128);
    }

    // ============================================================
    // read_vmdatal ($2139)
    // ============================================================

    /// read_vmdatal must return the low byte that was previously latched (pre-fetch behaviour).
    #[test]
    fn test_read_vmdatal_returns_latched_low_byte() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0x1234;
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x00, 0x00);

        // Load the latch manually (simulates what write_vmadd_low would do)
        vram.load_latch(regs.vmaddl, regs.vmaddh);

        let val = vram.read_vmdatal(&mut regs);

        // latch was 0x1234 -> lo byte == 0x34
        assert_eq!(val, 0x34);
    }

    /// After read_vmdatal with vmain bit7=0, the address must increment and the latch refreshed.
    /// The SNES pre-fetch model: the latch is reloaded with the *next* word after increment,
    /// so vram_latch holds the word at the new address only after the read completes.
    #[test]
    fn test_read_vmdatal_increments_and_refreshes_latch() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0x1234;
        vram.memory[0x0001] = 0x5678;
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x00, 0x00);

        vram.load_latch(regs.vmaddl, regs.vmaddh); // latch = 0x1234

        // First read: returns lo of latch(0x1234)=0x34, then increments addr to 0x0001
        // and reloads latch with memory[0x0001]=0x5678
        let _ = vram.read_vmdatal(&mut regs);

        assert_eq!(regs.vmaddl, 0x01, "address must have incremented to 0x0001");
        assert_eq!(vram.vram_latch, 0x5678, "latch must hold word at new address 0x0001");
    }

    /// read_vmdatal with vmain bit7=1 must NOT increment the address or reload the latch.
    #[test]
    fn test_read_vmdatal_no_increment_when_high_mode() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0xBEEF;
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x00, 0x00);

        vram.load_latch(regs.vmaddl, regs.vmaddh);

        let _ = vram.read_vmdatal(&mut regs);

        assert_eq!(regs.vmaddl, 0x00);
        assert_eq!(vram.vram_latch, 0xBEEF);
    }

    // ============================================================
    // read_vmdatah ($213A)
    // ============================================================

    /// read_vmdatah must return the high byte that was previously latched.
    #[test]
    fn test_read_vmdatah_returns_latched_high_byte() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0xABCD;
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x00, 0x00);
        
        vram.load_latch(regs.vmaddl, regs.vmaddh);

        let val = vram.read_vmdatah(&mut regs);

        // latch was 0xABCD -> hi byte == 0xAB
        assert_eq!(val, 0xAB);
    }

    /// After read_vmdatah with vmain bit7=1, the address must increment and the latch refreshed.
    #[test]
    fn test_read_vmdatah_increments_and_refreshes_latch() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0xABCD;
        vram.memory[0x0001] = 0xDEAD;
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x00, 0x00);
        vram.load_latch(regs.vmaddl, regs.vmaddh); // latch = 0xABCD

        // First read: returns hi of latch(0xABCD)=0xAB, then increments addr to 0x0001
        // and reloads latch with memory[0x0001]=0xDEAD
        let _ = vram.read_vmdatah(&mut regs);

        assert_eq!(regs.vmaddl, 0x01, "address must have incremented to 0x0001");
        assert_eq!(vram.vram_latch, 0xDEAD, "latch must hold word at new address 0x0001");
    }

    /// read_vmdatah with vmain bit7=0 must NOT increment the address or reload the latch.
    #[test]
    fn test_read_vmdatah_no_increment_when_low_mode() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0xCAFE;
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0x00, 0x00);
        vram.load_latch(regs.vmaddl, regs.vmaddh);
 
        let _ = vram.read_vmdatah(&mut regs);
 
        assert_eq!(regs.vmaddl, 0x00);
        assert_eq!(vram.vram_latch, 0xCAFE);
    }

    // ============================================================
    // read_vmdata lo+hi combined ($2139 / $213A)
    // ============================================================

    /// read_vmdata with vmain bit7=1 must return the full latched word and increment once after.
    #[test]
    fn test_read_vmdata_returns_full_word_and_increments_in_high_mode() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0xABCD;
        vram.memory[0x0001] = 0x1234;
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x00, 0x00);
        vram.load_latch(regs.vmaddl, regs.vmaddh);
 
        let word = vram.read_vmdata(&mut regs);
 
        assert_eq!(word, 0xABCD);
        assert_eq!(regs.vmaddl, 0x01);
        assert_eq!(vram.vram_latch, 0x1234);
    }

    /// read_vmdata with vmain bit7=1 must produce the same result as read_vmdatal + read_vmdatah.
    #[test]
    fn test_read_vmdata_equivalent_to_separate_reads_high_mode() {
        let mut vram_a = VRAM::new();
        let mut vram_b = VRAM::new();
        vram_a.memory[0x0000] = 0xDEAD;
        vram_b.memory[0x0000] = 0xDEAD;
 
        let mut regs_a = make_regs(VMAIN_INC1_AFTER_HIGH, 0x00, 0x00);
        vram_a.load_latch(regs_a.vmaddl, regs_a.vmaddh);
        let word = vram_a.read_vmdata(&mut regs_a);
 
        let mut regs_b = make_regs(VMAIN_INC1_AFTER_HIGH, 0x00, 0x00);
        vram_b.load_latch(regs_b.vmaddl, regs_b.vmaddh);
        let lo = vram_b.read_vmdatal(&mut regs_b);
        let hi = vram_b.read_vmdatah(&mut regs_b);
 
        assert_eq!(word, (lo as u16) | ((hi as u16) << 8));
        assert_eq!(regs_a.vmaddl, regs_b.vmaddl);
        assert_eq!(vram_a.vram_latch, vram_b.vram_latch);
    }

    /// read_vmdata with vmain bit7=0 must produce the same result as read_vmdatal + read_vmdatah,
    /// including the latch reload between lo and hi reads.
    #[test]
    fn test_read_vmdata_equivalent_to_separate_reads_low_mode() {
        let mut vram_a = VRAM::new();
        let mut vram_b = VRAM::new();
        vram_a.memory[0x0000] = 0x00CD;
        vram_b.memory[0x0000] = 0x00CD;
        vram_a.memory[0x0001] = 0xAB00;
        vram_b.memory[0x0001] = 0xAB00;
 
        let mut regs_a = make_regs(VMAIN_INC1_AFTER_LOW, 0x00, 0x00);
        vram_a.load_latch(regs_a.vmaddl, regs_a.vmaddh);
        let word = vram_a.read_vmdata(&mut regs_a);
 
        let mut regs_b = make_regs(VMAIN_INC1_AFTER_LOW, 0x00, 0x00);
        vram_b.load_latch(regs_b.vmaddl, regs_b.vmaddh);
        let lo = vram_b.read_vmdatal(&mut regs_b);
        let hi = vram_b.read_vmdatah(&mut regs_b);
 
        assert_eq!(word, (lo as u16) | ((hi as u16) << 8));
        assert_eq!(regs_a.vmaddl, regs_b.vmaddl);
        assert_eq!(vram_a.vram_latch, vram_b.vram_latch);
    }

    /// read_vmdata with increment-by-32 must advance the address by 32.
    #[test]
    fn test_read_vmdata_increment_by_32() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC32_AFTER_HIGH, 0x00, 0x00);
        vram.load_latch(regs.vmaddl, regs.vmaddh);

        vram.read_vmdata(&mut regs);

        let addr = (regs.vmaddl as u16) | ((regs.vmaddh as u16) << 8);
        assert_eq!(addr, 32);
    }

    /// read_vmdata with increment-by-128 must advance the address by 128.
    #[test]
    fn test_read_vmdata_increment_by_128() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC128_AFTER_HIGH, 0x00, 0x00);
        vram.load_latch(regs.vmaddl, regs.vmaddh);

        vram.read_vmdata(&mut regs);

        let addr = (regs.vmaddl as u16) | ((regs.vmaddh as u16) << 8);
        assert_eq!(addr, 128);
    }

    /// A full write_vmdata + read_vmdata round-trip must return the original word.
    #[test]
    fn test_write_vmdata_read_vmdata_round_trip() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x10, 0x00);

        vram.write_vmdata(&mut regs, 0xCAFE);
        vram.write_vmadd(&mut regs, 0x0010);

        let word = vram.read_vmdata(&mut regs);

        assert_eq!(word, 0xCAFE);
    }

    // ============================================================
    // load_latch
    // ============================================================

    /// load_latch must copy the word at the composed address into vram_latch.
    #[test]
    fn test_load_latch_copies_word() {
        let mut vram = VRAM::new();
        vram.memory[0x0200] = 0xF00D;

        vram.load_latch(0x00, 0x02); // address = 0x0200

        assert_eq!(vram.vram_latch, 0xF00D);
    }

    /// load_latch at address 0 must latch the first word.
    #[test]
    fn test_load_latch_address_zero() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0x1111;

        vram.load_latch(0x00, 0x00);

        assert_eq!(vram.vram_latch, 0x1111);
    }

    // ============================================================
    // Address wrap-around
    // ============================================================

    /// The effective VRAM address is 15-bit (0x0000–0x7FFF); incrementing past 0x7FFF must wrap to 0x0000.
    #[test]
    fn test_address_wraps_at_0x7fff() {
        let mut vram = VRAM::new();
        vram.memory[0x0000] = 0xFFFF;
        let mut regs = make_regs(VMAIN_INC1_AFTER_LOW, 0xFF, 0x7F);

        // Write something at 0x7FFF
        vram.write_vmdatal(&mut regs, 0xAA);

        // Address must have wrapped to 0x0000
        assert_eq!(regs.vmaddl, 0x00);
        assert_eq!(regs.vmaddh, 0x00);
    }

    // ============================================================
    // Round-trip write / read
    // ============================================================

    /// Writing a full 16-bit word and reading it back must produce the original value.
    #[test]
    fn test_round_trip_write_then_read() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x10, 0x00);

        // Write low byte (no increment yet: high-byte mode)
        vram.write_vmdatal(&mut regs, 0x56);
        // Write high byte (increments here)
        vram.write_vmdatah(&mut regs, 0x78);

        // Reset address back to 0x0010 to read
        vram.write_vmadd_low(&mut regs, 0x10);
        vram.write_vmadd_high(&mut regs, 0x00);

        let lo = vram.read_vmdatal(&mut regs);
        let hi = vram.read_vmdatah(&mut regs);

        assert_eq!(lo, 0x56);
        assert_eq!(hi, 0x78);
    }

    /// Sequential writes at incrementing addresses must not corrupt adjacent words.
    #[test]
    fn test_sequential_writes_dont_corrupt_neighbours() {
        let mut vram = VRAM::new();
        let mut regs = make_regs(VMAIN_INC1_AFTER_HIGH, 0x00, 0x00);

        // Write 0xAABB at address 0, 0xCCDD at address 1, using low-byte increment mode.
        vram.write_vmdatal(&mut regs, 0xBB);
        vram.write_vmdatah(&mut regs, 0xAA);
        vram.write_vmdatal(&mut regs, 0xDD);
        vram.write_vmdatah(&mut regs, 0xCC);

        assert_eq!(vram.memory[0x0000], 0xAABB);
        assert_eq!(vram.memory[0x0001], 0xCCDD);
    }
}
