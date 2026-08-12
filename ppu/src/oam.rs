// OAM layout:
// Table 1 ($000-$1FF): 128 sprites * 4 bytes
// Table 2 ($200-$21F): 128 sprites * 2 bits (packed, 4 sprites per byte)
const OAM_TABLE1_SIZE: usize = 512;
const OAM_TABLE2_SIZE: usize = 32;
const OAM_SIZE: usize = OAM_TABLE1_SIZE + OAM_TABLE2_SIZE;

/// Decoded sprite attributes from OAM.
#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    /// X position (9-bit signed: sprites past 255 wrap to the left of the screen)
    pub x: i16,
    /// Y position (top edge of the sprite)
    pub y: u8,
    /// Raw tile number (0-255) from OAM. The final VRAM address is
    /// `chr_base + tile * 16`, with multi-tile sprites wrapping the low nibble
    /// (horizontal) and high nibble (vertical) independently.
    pub tile: u8,
    /// Base CHR word address for this sprite, with name-table selection applied.
    pub chr_base: u16,
    /// Name-table select bit (from table 1 attribute byte)
    pub name_table: u8,
    /// Palette number (0-7); sprites use CGRAM entries 128-255
    pub palette: u8,
    /// Priority (0-3)
    pub priority: u8,
    /// Horizontal flip
    pub flip_x: bool,
    /// Vertical flip
    pub flip_y: bool,
    /// Large size bit (the actual size comes from OBJSEL)
    pub large: bool,
}

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

    // ============================================================
    // Sprite decoding
    // ============================================================

    /// Decode sprite `index` (0-127) from OAM
    pub fn get_sprite(&self, index: u8, objsel: u8) -> Sprite {
        let i = index as usize;

        // Table 1: 4 bytes per sprite
        let x_lo = self.data[i * 4];
        let y = self.data[i * 4 + 1];
        let tile_lo = self.data[i * 4 + 2];
        let attr = self.data[i * 4 + 3]; // vhoopppN
        let flip_y = (attr & 0x80) != 0;
        let flip_x = (attr & 0x40) != 0;
        let priority = (attr >> 4) & 0x03;
        let palette = (attr >> 1) & 0x07;
        let name_table = attr & 0x01;

        // Table 2: 2 bits per sprite, packed 4 per byte
        let t2_byte = self.data[OAM_TABLE1_SIZE + i / 4];
        let t2_bits = (t2_byte >> ((i % 4) * 2)) & 0x03;
        let x_hi = t2_bits & 0x01; // bit 8 of X
        let large = (t2_bits & 0x02) != 0;

        // Reconstruct 9-bit X (256-511 => -256 to -1)
        let x_raw = ((x_hi as u16) << 8) | x_lo as u16;
        let x = if x_hi != 0 { x_raw as i16 - 512 } else { x_raw as i16 };

        // CHR base address from OBJSEL, plus name-table selection.
        // OBJSEL bits 2:0 = name base address (in 0x2000-word steps),
        // bits 4:3 = secondary name select (offset for name_table == 1).
        let name_base = (objsel & 0x07) as u16;
        let name_select = ((objsel >> 3) & 0x03) as u16;
        let chr_base = if name_table == 0 {
            name_base << 13
        } else {
            (name_base << 13).wrapping_add((name_select + 1) << 12)
        };

        Sprite {
            x,
            y,
            tile: tile_lo,
            chr_base,
            name_table,
            palette,
            priority,
            flip_x,
            flip_y,
            large,
        }
    }

    /// Return the sprite size (width, height) in pixels for a given sprite,
    /// according to the size mode encoded in OBJSEL bits 7:5.
    pub fn sprite_size(objsel: u8, large: bool) -> (u8, u8) {
        match (objsel >> 5) & 0x07 {
            0 => if large { (16, 16) } else { (8, 8) },
            1 => if large { (32, 32) } else { (8, 8) },
            2 => if large { (64, 64) } else { (8, 8) },
            3 => if large { (32, 32) } else { (16, 16) },
            4 => if large { (64, 64) } else { (16, 16) },
            5 => if large { (64, 64) } else { (32, 32) },
            // can't find documentation for 6 and 7, treating them as 8x8 / 16x16 for now
            _ => if large { (16, 16) } else { (8, 8) },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_oam() -> OAM {
        OAM::new()
    }

    // ============================================================
    // Register access
    // ============================================================

    // A freshly created OAM is fully zeroed.
    #[test]
    fn test_new_zeroed() {
        let oam = make_oam();
        assert!(oam.data.iter().all(|&b| b == 0));
        assert_eq!(oam.word_addr, 0);
    }

    // write_addr sets the byte address (masked to 9 bits) and resets the latch.
    #[test]
    fn test_write_addr() {
        let mut oam = make_oam();
        oam.write_phase_high = true;
        oam.write_addr(0xFFFF);
        assert_eq!(oam.word_addr, 0x01FF);
        assert!(!oam.write_phase_high);
    }

    // Table 1: first write latches, second commits lo+hi and advances by 2.
    #[test]
    fn test_write_data_table1_write_twice() {
        let mut oam = make_oam();
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
        let mut oam = make_oam();
        oam.write_addr(OAM_TABLE1_SIZE as u16);
        oam.write_data(0b10110001);
        assert_eq!(oam.data[OAM_TABLE1_SIZE], 0b10110001);
    }

    // read_data returns bytes in order and advances the address.
    #[test]
    fn test_read_data_advances_address() {
        let mut oam = make_oam();
        oam.data[0] = 0xAA;
        oam.data[1] = 0xBB;
        oam.write_addr(0x0000);
        assert_eq!(oam.read_data(), 0xAA);
        assert_eq!(oam.word_addr, 1);
        assert_eq!(oam.read_data(), 0xBB);
        assert_eq!(oam.word_addr, 2);
    }

    // ============================================================
    // Sprite decoding
    // ============================================================

    // get_sprite decodes all fields, including 9-bit signed X and large bit.
    #[test]
    fn test_get_sprite_decodes_fields() {
        let mut oam = make_oam();
        // x=100, y=50, tile=5, attr=vhoopppN=0b11001110
        oam.data[0] = 100;
        oam.data[1] = 50;
        oam.data[2] = 5;
        oam.data[3] = 0b11001110;
        // table 2 sprite 0: x_hi=1, large=0
        oam.data[OAM_TABLE1_SIZE] = 0b01;

        let s = oam.get_sprite(0, 0x00);
        assert_eq!(s.x, -156); // (1<<8 | 100) = 356 -> 356-512 = -156
        assert_eq!(s.y, 50);
        assert_eq!(s.tile, 5);
        assert!(s.flip_x);
        assert!(s.flip_y);
        assert_eq!(s.palette, 7);
        assert_eq!(s.priority, 0);
        assert_eq!(s.name_table, 0);
        assert!(!s.large);

        // large bit set
        oam.data[OAM_TABLE1_SIZE] = 0b10;
        let s = oam.get_sprite(0, 0x00);
        assert!(s.large);
        assert_eq!(s.x, 0);
    }

    // sprite_size returns correct dimensions for all 6 documented size modes.
    #[test]
    fn test_sprite_size() {
        let cases: &[(u8, (u8, u8), (u8, u8))] = &[
            (0 << 5, (8, 8), (16, 16)),
            (1 << 5, (8, 8), (32, 32)),
            (2 << 5, (8, 8), (64, 64)),
            (3 << 5, (16, 16), (32, 32)),
            (4 << 5, (16, 16), (64, 64)),
            (5 << 5, (32, 32), (64, 64)),
        ];
        for &(objsel, small, large) in cases {
            assert_eq!(OAM::sprite_size(objsel, false), small, "objsel={objsel:#04X} small");
            assert_eq!(OAM::sprite_size(objsel, true), large, "objsel={objsel:#04X} large");
        }
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
