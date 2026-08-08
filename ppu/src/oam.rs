// OAM layout:
// Table 1 ($000-$1FF): 128 sprites * 4 bytes
// Table 2 ($200-$21F): 128 sprites * 2 bits (packed, 4 sprites per byte)
const OAM_TABLE1_SIZE: usize = 512;
const OAM_TABLE2_SIZE: usize = 32;
const OAM_SIZE: usize = OAM_TABLE1_SIZE + OAM_TABLE2_SIZE;

pub struct OAM {
    /// Raw OAM data: 512 bytes table 1 + 32 bytes table 2.
    data: [u8; OAM_SIZE],

    /// Internal byte address (0-543 range, masked to 9 bits on write).
    /// Set by OAMADDL/H ($2102/$2103).
    word_addr: u16,

    /// Low-byte write buffer for table 1 (write-twice mechanism).
    /// Table 1 writes commit in pairs: first write latches lo, second commits lo+hi.
    write_latch: u8,

    /// True when the next OAMDATA write is the high byte of a table 1 pair.
    write_phase_high: bool,
}

impl Default for OAM {
    fn default() -> Self {
        Self::new()
    }
}

impl OAM {
    pub fn new() -> Self {
        Self {
            data: [0; OAM_SIZE],
            word_addr: 0,
            write_latch: 0,
            write_phase_high: false,
        }
    }

    // ============================================================
    // $2102/$2103 - OAMADDL/OAMADDH
    // ============================================================

    /// Set the OAM byte address from the OAMADDL/H register pair (9-bit).
    /// Resets the write-twice latch.
    pub fn write_addr(&mut self, oamadd: u16) {
        self.word_addr = oamadd & 0x01FF;
        self.write_phase_high = false;
    }

    // ============================================================
    // $2104 - OAMDATA (write)
    // ============================================================

    /// Write one byte to OAM via the data port.
    ///
    /// Table 1 ($000-$1FF): buffered in pairs. The first write latches the
    /// low byte; the second commits both bytes and advances the address by 2.
    /// Table 2 ($200-$21F): committed immediately, one byte at a time.
    pub fn write_data(&mut self, value: u8) {
        let addr = self.word_addr as usize;

        if addr < OAM_TABLE1_SIZE {
            // Table 1: write-twice (lo then hi)
            if !self.write_phase_high {
                self.write_latch = value;
                self.write_phase_high = true;
            } else {
                let byte_addr = addr & !1; // align to even
                self.data[byte_addr] = self.write_latch;
                self.data[byte_addr + 1] = value;
                self.write_phase_high = false;
                self.word_addr = (self.word_addr + 2) & 0x01FF;
            }
        } else {
            // Table 2: single-byte write, committed immediately
            let byte_addr = OAM_TABLE1_SIZE + (addr - OAM_TABLE1_SIZE) % OAM_TABLE2_SIZE;
            self.data[byte_addr] = value;
            self.word_addr = OAM_TABLE1_SIZE as u16
                + ((self.word_addr - OAM_TABLE1_SIZE as u16 + 1) % OAM_TABLE2_SIZE as u16);
        }
    }

    // ============================================================
    // $2138 - OAMDATAREAD (read)
    // ============================================================

    /// Read one byte from OAM via the data port.
    /// Reads are single-byte and advance the address by 1.
    pub fn read_data(&mut self) -> u8 {
        let addr = self.word_addr as usize % OAM_SIZE;
        let value = self.data[addr];
        self.word_addr = (self.word_addr + 1) % OAM_SIZE as u16;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // write_addr sets the byte address (masked to 9 bits) and resets the latch.
    #[test]
    fn test_write_addr() {
        let mut oam = OAM::new();
        oam.write_phase_high = true;
        oam.write_addr(0xFFFF);
        assert_eq!(oam.word_addr, 0x01FF);
        assert!(!oam.write_phase_high);
    }

    // Table 1: first write latches, second commits lo+hi and advances by 2.
    #[test]
    fn test_write_data_table1_write_twice() {
        let mut oam = OAM::new();
        oam.write_addr(0x0000);

        oam.write_data(0xCD); // lo latched
        assert_eq!(oam.data[0], 0x00);
        assert!(oam.write_phase_high);
        assert_eq!(oam.write_latch, 0xCD);

        oam.write_data(0xEF); // commit
        assert_eq!(oam.data[0], 0xCD);
        assert_eq!(oam.data[1], 0xEF);
        assert_eq!(oam.word_addr, 2);
        assert!(!oam.write_phase_high);
    }

    // Table 2: writes commit immediately, one byte at a time.
    #[test]
    fn test_write_data_table2_immediate() {
        let mut oam = OAM::new();
        oam.write_addr(OAM_TABLE1_SIZE as u16);
        oam.write_data(0b10110001);
        assert_eq!(oam.data[OAM_TABLE1_SIZE], 0b10110001);
    }

    // read_data returns bytes in order and advances the address.
    #[test]
    fn test_read_data_advances_address() {
        let mut oam = OAM::new();
        oam.data[0] = 0xAA;
        oam.data[1] = 0xBB;
        oam.write_addr(0x0000);
        assert_eq!(oam.read_data(), 0xAA);
        assert_eq!(oam.word_addr, 1);
        assert_eq!(oam.read_data(), 0xBB);
        assert_eq!(oam.word_addr, 2);
    }
}
