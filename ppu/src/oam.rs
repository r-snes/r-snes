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

    /// STAT77 bit 7: set when more than 32 sprites were found on a scanline.
    /// Latched via set_flags; read through $213E.
    pub time_over: bool,

    /// STAT77 bit 6: set when more than 34 sprite tiles were found on a scanline.
    /// Latched via set_flags; read through $213E.
    pub range_over: bool,
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
            time_over: false,
            range_over: false,
        }
    }

    // ============================================================
    // $2102/$2103 - OAMADDL/OAMADDH
    // ============================================================

    /// Set the OAM byte address from the OAMADDL/H register pair, covering
    /// both tables (0-543). Resets the write-twice latch.
    pub fn write_addr(&mut self, oamadd: u16) {
        self.word_addr = (oamadd as usize % OAM_SIZE) as u16;
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

    // ============================================================
    // Per-scanline evaluation
    // ============================================================

    /// Evaluate which sprites are visible on scanline `y`.
    ///
    /// - Sprites are evaluated in order, starting from the priority-rotation
    ///   index (OAMADDH bit 7 enables it, OAMADDL >> 1 gives the start sprite),
    ///   wrapping around all 128 sprites.
    /// - At most 32 sprites per scanline are kept; a 33rd sets `time_over`.
    /// - At most 34 sprite tiles (8-pixel slices) fit on a scanline; beyond
    ///   that, `range_over` is set.
    /// - Returns (visible sprites, time_over, range_over). The flags are not
    ///   stored here (this borrows &self); call `set_flags` to latch them.
    pub fn eval_sprites_for_scanline(&self, y: usize, objsel: u8, oamadd: u16) -> (Vec<(u8, Sprite)>, bool, bool) {
        let priority_rotation = (oamadd >> 8) & 0x01 != 0;
        let start = if priority_rotation {
            ((oamadd & 0xFF) >> 1) as usize & 0x7F
        } else {
            0
        };

        let mut visible: Vec<(u8, Sprite)> = Vec::with_capacity(32);
        let mut time_over = false;
        let mut range_over = false;
        let mut tile_count: u16 = 0;

        for i in 0..128usize {
            let idx = (start + i) & 0x7F;
            let sprite = self.get_sprite(idx as u8, objsel);
            let (width, height) = Self::sprite_size(objsel, sprite.large);

            // Y range check in wrapping u8 arithmetic (matches hardware).
            let dy = (y as u8).wrapping_sub(sprite.y);
            if dy >= height {
                continue;
            }

            if visible.len() >= 32 {
                time_over = true;
                break;
            }

            // Each visible sprite contributes width/8 tiles on this scanline.
            tile_count += (width as u16) / 8;
            if tile_count > 34 {
                range_over = true;
            }

            visible.push((idx as u8, sprite));
        }

        (visible, time_over, range_over)
    }

    /// Latch the time_over / range_over flags for STAT77 ($213E).
    pub fn set_flags(&mut self, time_over: bool, range_over: bool) {
        self.time_over = time_over;
        self.range_over = range_over;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_oam() -> OAM {
        let mut oam = OAM::new();
        // Park every sprite off-screen (y=240) by default, so evaluation
        // tests only see the sprites they explicitly place.
        for i in 0..128 {
            oam.data[i * 4 + 1] = 240;
        }
        oam
    }

    fn make_sprite_entry(oam: &mut OAM, index: u8, x: u8, y: u8, tile: u8, attr: u8, t2_bits: u8) {
        let i = index as usize;
        oam.data[i * 4] = x;
        oam.data[i * 4 + 1] = y;
        oam.data[i * 4 + 2] = tile;
        oam.data[i * 4 + 3] = attr;
        let byte_idx = OAM_TABLE1_SIZE + i / 4;
        let shift = (i % 4) * 2;
        oam.data[byte_idx] &= !(0x03 << shift);
        oam.data[byte_idx] |= (t2_bits & 0x03) << shift;
    }

    // objsel = 0x00 -> small=8x8, large=16x16
    const OBJSEL_8_16: u8 = 0x00;

    // ============================================================
    // Register access
    // ============================================================

    // A freshly created OAM is fully zeroed.
    #[test]
    fn test_new_zeroed() {
        let oam = OAM::new();
        assert!(oam.data.iter().all(|&b| b == 0));
        assert_eq!(oam.word_addr, 0);
    }

    // write_addr sets the byte address (covering both tables) and resets the latch.
    #[test]
    fn test_write_addr() {
        let mut oam = make_oam();
        oam.write_phase_high = true;

        // Table 1 address
        oam.write_addr(0x0010);
        assert_eq!(oam.word_addr, 0x0010);
        assert!(!oam.write_phase_high);

        // Table 2 start address
        oam.write_addr(OAM_TABLE1_SIZE as u16);
        assert_eq!(oam.word_addr, OAM_TABLE1_SIZE as u16);

        // Out-of-range addresses wrap within the 544-byte OAM
        oam.write_addr(OAM_SIZE as u16);
        assert_eq!(oam.word_addr, 0);
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

        // large bit set (x_hi=0 here, so x = x_lo = 100)
        oam.data[OAM_TABLE1_SIZE] = 0b10;
        let s = oam.get_sprite(0, 0x00);
        assert!(s.large);
        assert_eq!(s.x, 100);
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

    // ============================================================
    // Per-scanline evaluation
    // ============================================================

    // A sprite covering the scanline appears; outside its Y range it does not.
    #[test]
    fn test_eval_sprite_range() {
        let mut oam = make_oam();
        make_sprite_entry(&mut oam, 0, 0, 10, 0, 0, 0); // y=10, 8x8 -> rows 10-17

        let (visible, over, _) = oam.eval_sprites_for_scanline(10, OBJSEL_8_16, 0);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].0, 0);
        assert!(!over);

        assert_eq!(oam.eval_sprites_for_scanline(17, OBJSEL_8_16, 0).0.len(), 1);
        assert_eq!(oam.eval_sprites_for_scanline(18, OBJSEL_8_16, 0).0.len(), 0);
        assert_eq!(oam.eval_sprites_for_scanline(9, OBJSEL_8_16, 0).0.len(), 0);
    }

    // 33 sprites on one scanline: only 32 kept, time_over set; exactly 32 is fine.
    #[test]
    fn test_eval_time_over() {
        let mut oam = make_oam();
        for i in 0..33u8 {
            make_sprite_entry(&mut oam, i, 0, 0, 0, 0, 0);
        }
        let (visible, over, _) = oam.eval_sprites_for_scanline(0, OBJSEL_8_16, 0);
        assert_eq!(visible.len(), 32);
        assert!(over);

        let mut oam = make_oam();
        for i in 0..32u8 {
            make_sprite_entry(&mut oam, i, 0, 0, 0, 0, 0);
        }
        let (visible, over, _) = oam.eval_sprites_for_scanline(0, OBJSEL_8_16, 0);
        assert_eq!(visible.len(), 32);
        assert!(!over);
    }

    // Priority rotation starts evaluation at the sprite given by OAMADDL >> 1.
    #[test]
    fn test_eval_priority_rotation() {
        let mut oam = make_oam();
        make_sprite_entry(&mut oam, 10, 0, 0, 0, 0, 0);
        let oamadd: u16 = (1 << 8) | 20; // enable + start sprite 10 (20 >> 1)
        let (visible, _, _) = oam.eval_sprites_for_scanline(0, OBJSEL_8_16, oamadd);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].0, 10);
    }

    // Y wraps in u8: a sprite at y=250 covers scanline 255 but not 249.
    #[test]
    fn test_eval_y_wrap() {
        let mut oam = make_oam();
        make_sprite_entry(&mut oam, 0, 0, 250, 0, 0, 0); // y=250, 8x8
        assert_eq!(oam.eval_sprites_for_scanline(255, OBJSEL_8_16, 0).0.len(), 1);
        assert_eq!(oam.eval_sprites_for_scanline(249, OBJSEL_8_16, 0).0.len(), 0);
    }

    // Without rotation, sprites come back in ascending index order.
    #[test]
    fn test_eval_order() {
        let mut oam = make_oam();
        make_sprite_entry(&mut oam, 0, 0, 0, 0, 0, 0);
        make_sprite_entry(&mut oam, 5, 0, 0, 0, 0, 0);
        make_sprite_entry(&mut oam, 2, 0, 0, 0, 0, 0);
        let (visible, _, _) = oam.eval_sprites_for_scanline(0, OBJSEL_8_16, 0);
        assert_eq!(visible[0].0, 0);
        assert_eq!(visible[1].0, 2);
        assert_eq!(visible[2].0, 5);
    }

    // range_over is set when more than 34 tiles (8px slices) fall on a line.
    // Small 8x8 sprites can't trigger it (the 32-sprite time_over limit hits
    // first), so use large 64px-wide sprites: 5 * 8 = 40 tiles > 34, with only
    // 5 sprites. OBJSEL mode 2 gives large = 64x64.
    #[test]
    fn test_eval_range_over() {
        const OBJSEL_8_64: u8 = 2 << 5; // small 8x8, large 64x64

        let mut oam = make_oam();
        for i in 0..5u8 {
            make_sprite_entry(&mut oam, i, 0, 0, 0, 0, 0b10); // large bit set
        }
        let (_, time_over, range_over) = oam.eval_sprites_for_scanline(0, OBJSEL_8_64, 0);
        assert!(!time_over);
        assert!(range_over);

        // 4 large sprites = 32 tiles, not over.
        let mut oam = make_oam();
        for i in 0..4u8 {
            make_sprite_entry(&mut oam, i, 0, 0, 0, 0, 0b10);
        }
        let (_, _, range_over) = oam.eval_sprites_for_scanline(0, OBJSEL_8_64, 0);
        assert!(!range_over);
    }

    // set_flags latches the STAT77 flags.
    #[test]
    fn test_set_flags() {
        let mut oam = make_oam();
        assert!(!oam.time_over);
        assert!(!oam.range_over);
        oam.set_flags(true, true);
        assert!(oam.time_over);
        assert!(oam.range_over);
        oam.set_flags(false, false);
        assert!(!oam.time_over);
        assert!(!oam.range_over);
    }
}
