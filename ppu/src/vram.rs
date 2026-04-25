use crate::constants::VRAM_SIZE;
use common::u16_split::U16Split;

pub struct VRAM {
    pub memory: [u16; VRAM_SIZE],

    // Registers
    pub vmain: u8, // $2115
    pub vmadd: u16, // current word address (0–0x7FFF)

    // Internal state
    pub vram_latch: u16, // word latch for reads
}

impl VRAM {
    pub fn new() -> Self {
        Self {
            memory: [0; VRAM_SIZE],
            vmain: 0,
            vmadd: 0,
            vram_latch: 0,
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

    fn increment_vmadd(&mut self) {
        self.vmadd = (self.vmadd + self.increment_amount()) & 0x7FFF;
    }

    // ============================================================
    // VMADD ($2116 / $2117)
    // ============================================================

    pub fn write_vmadd_low(&mut self, value: u8) {
        *self.vmadd.lo_mut() = value;
        self.load_latch();
    }

    pub fn write_vmadd_high(&mut self, value: u8) {
        *self.vmadd.hi_mut() = value & 0x7F;
        self.load_latch();
    }

    // ============================================================
    // VRAM DATA WRITE ($2118 / $2119)
    // ============================================================

    pub fn write_vmdatal(&mut self, value: u8) {
        let word = &mut self.memory[self.vmadd as usize];
        *word.lo_mut() = value;

        if self.increment_after_low() {
            self.increment_vmadd();
        }
    }

    pub fn write_vmdatah(&mut self, value: u8) {
        let word = &mut self.memory[self.vmadd as usize];
        *word.hi_mut() = value;

        if self.increment_after_high() {
            self.increment_vmadd();
        }
    }

    // ============================================================
    // VRAM DATA READ ($2139 / $213A)
    // ============================================================

    pub fn read_vmdatal(&mut self) -> u8 {
        let value = *self.vram_latch.lo();

        if self.increment_after_low() {
            self.load_latch();
            self.increment_vmadd();
        }

        value
    }

    pub fn read_vmdatah(&mut self) -> u8 {
        let value = *self.vram_latch.hi();

        if self.increment_after_high() {
            self.load_latch();
            self.increment_vmadd();
        }

        value
    }

    // ============================================================
    // Helpers
    // ============================================================

    pub fn load_latch(&mut self) {
        self.vram_latch = self.memory[self.vmadd as usize];
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     // ============================================================
//     // $2115 - VMAIN
//     // ============================================================

//     // This test verifies that writing to VMAIN correctly stores the value.
//     #[test]
//     fn test_write_vmain_sets_value() {
//         let mut vram = VRAM::new();

//         vram.write_vmain(0xAB);

//         assert_eq!(vram.vmain, 0xAB);
//     }

//     // This test verifies that increment_amount() returns correct values for all modes.
//     #[test]
//     fn test_increment_amount_modes() {
//         let mut vram = VRAM::new();

//         vram.write_vmain(0b00);
//         assert_eq!(vram.increment_amount(), 1);

//         vram.write_vmain(0b01);
//         assert_eq!(vram.increment_amount(), 32);

//         vram.write_vmain(0b10);
//         assert_eq!(vram.increment_amount(), 128);

//         vram.write_vmain(0b11);
//         assert_eq!(vram.increment_amount(), 128);
//     }

//     // This test verifies increment_after_low/high flags based on bit 7 of VMAIN.
//     #[test]
//     fn test_increment_after_flags() {
//         let mut vram = VRAM::new();

//         vram.write_vmain(0x00);
//         assert!(vram.increment_after_low());
//         assert!(!vram.increment_after_high());

//         vram.write_vmain(0x80);
//         assert!(!vram.increment_after_low());
//         assert!(vram.increment_after_high());
//     }

//     // ============================================================
//     // VMADD ($2116 / $2117)
//     // ============================================================

//     // This test verifies that writing low and high bytes of VMADD sets vmadd correctly and loads latch.
//     #[test]
//     fn test_write_vmadd_low_high() {
//         let mut vram = VRAM::new();

//         vram.memory[0] = 0x12;
//         vram.memory[1] = 0x34;

//         vram.write_vmadd_low(0x00);
//         vram.write_vmadd_high(0x00);

//         // Latch should load bytes 0 and 1
//         assert_eq!(vram.vmadd, 0);
//         assert_eq!(vram.vram_latch, 0x3412);
//     }

//     fn test_vram_vmadd_wrap_around() {
//         let mut vram = VRAM::new();

//         vram.vmadd = 0x7FFF; // Set max value (0x7FFF)
//         vram.write_vmain(0x80);

//         vram.write_vmdatal(0x55); // low byte
//         assert_eq!(vram.vmadd, 0x7FFF); // VMADD should not increment yet

//         vram.write_vmdatah(0xAA); // high byte -> should increment and wrap
//         assert_eq!(vram.vmadd, 0); // VMADD wraps around to 0

//         let addr = (0x7FFF * 2) as usize;
//         assert_eq!(vram.memory[addr], 0x55);
//         assert_eq!(vram.memory[addr + 1], 0xAA);
//     }

//     // ============================================================
//     // VRAM DATA WRITE ($2118 / $2119)
//     // ============================================================

//     // This test verifies that writing to vmdatal/vmdatah updates memory and increments address according to VMAIN.
//     #[test]
//     fn test_write_vram_data() {
//         let mut vram = VRAM::new();

//         vram.write_vmain(0x80);
//         vram.write_vmadd_low(0x00);
//         vram.write_vmadd_high(0x00);

//         vram.write_vmdatal(0x11); // low byte
//         vram.write_vmdatah(0x22); // high byte

//         assert_eq!(vram.memory[0], 0x11); // Check memory contents
//         assert_eq!(vram.memory[1], 0x22);

//         assert_eq!(vram.vmadd, 1); // Check VMADD increment
//     }

//     // And $2115
//     // This test verifies increment behavior when bit 7 of VMAIN is set (increment after high byte).
//     #[test]
//     fn test_write_vram_data_increment_high() {
//         let mut vram = VRAM::new();

//         vram.write_vmain(0x80);
//         vram.write_vmadd_low(0x00);
//         vram.write_vmadd_high(0x00);

//         vram.write_vmdatal(0x11);
//         assert_eq!(vram.vmadd, 0); // Address should not increment yet

//         vram.write_vmdatah(0x22);
//         assert_eq!(vram.vmadd, 1); // Address should increment after high byte
//     }

//     // ============================================================
//     // VRAM DATA READ ($2139 / $213A)
//     // ============================================================

//     // And $2115
//     // This test verifies that reading vmdatal/vmdatah returns latch values and increments VMADD correctly.
//     #[test]
//     fn test_read_vram_data() {
//         let mut vram = VRAM::new();

//         vram.memory[0] = 0xAA;
//         vram.memory[1] = 0xBB;

//         vram.write_vmadd_low(0x00);
//         vram.write_vmadd_high(0x00);
//         vram.write_vmain(0x00); // increment after low

//         let low = vram.read_vmdatal();
//         let high = vram.read_vmdatah();

//         assert_eq!(low, 0xAA);
//         assert_eq!(high, 0xBB);

//         assert_eq!(vram.vmadd, 1); // VMADD should increment once (low + high)
//     }

//     // ============================================================
//     // Helpers
//     // ============================================================

//     // This test verifies that byte_address() returns the correct memory index.
//     #[test]
//     fn test_byte_address() {
//         let mut vram = VRAM::new();

//         vram.vmadd = 0x1234;
//         assert_eq!(vram.byte_address(), 0x1234 * 2);
//     }
// }
