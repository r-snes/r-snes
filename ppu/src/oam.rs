// /// Represents the Object Attribute Memory (OAM) for sprites.
// ///
// /// Each sprite takes 4 bytes:
// /// - Byte 0: Y position
// /// - Byte 1: Tile index
// /// - Byte 2: Attributes (palette, priority, flipping)
// /// - Byte 3: X position
// ///
// /// There are 128 sprites, total 512 bytes.
// /// The SNES uses 544 bytes internally to manage extra latches.
// #[derive(Debug)]
// pub struct Oam {
//     mem: [u8; 544], // 512 bytes + 32 extra internal bytes
//     addr: u16,      // Current OAM address (0..543)
// }
/// Represents the Object Attribute Memory (OAM) for sprites.
///
/// Low table: 512 bytes (128 sprites × 4 bytes)
/// High table: 32 bytes (X MSB + size bits)
#[derive(Debug)]
pub struct Oam {
    low: [u8; 512],
    high: [u8; 32],

    addr: u16,
}

impl Oam {
    /// Creates a new OAM instance with cleared memory and address set to 0.
    pub fn new() -> Self {
        Self {
            low: [0; 512],
            high: [0; 32],
            addr: 0,
        }
    }

    /// Writes the low 8 bits of the OAM address register ($2102).
    ///
    /// Only affects bits 0–7 of the internal OAM address.
    pub fn write_addr_low(&mut self, value: u8) {
        self.addr = (self.addr & 0x100) | value as u16;
    }

    /// Writes the high bit of the OAM address register ($2103).
    ///
    /// Only bit 0 is used, forming bit 8 of the internal address.
    pub fn write_addr_high(&mut self, value: u8) {
        self.addr = ((value as u16 & 0x01) << 8) | (self.addr & 0xFF);
    }

    /// Writes a byte to OAM through the data port ($2104).
    ///
    /// The value is written at the current address, then the address
    /// auto-increments and wraps around at 544 bytes.
    pub fn write_data(&mut self, value: u8) {
        let a = self.addr as usize;
        if a < 544 {
            self.mem[a] = value;
        }
        self.addr = (self.addr + 1) % 544;
    }

    /// Reads a byte from OAM through the data port ($2138).
    ///
    /// The value is read from the current address, then the address
    /// auto-increments and wraps around at 544 bytes.
    pub fn read_data(&mut self) -> u8 {
        let a = self.addr as usize;
        let v = if a < 544 { self.mem[a] } else { 0 };
        self.addr = (self.addr + 1) % 544;
        v
    }

    // ---------- helpers ----------

    /// Reads the 4-byte attribute data of a sprite by index.
    ///
    /// This bypasses PPU port behavior and is intended for debugging
    /// or testing purposes only.
    pub fn read_sprite(&self, index: usize) -> [u8; 4] {
        let base = index * 4;
        [
            self.mem[base],
            self.mem[base + 1],
            self.mem[base + 2],
            self.mem[base + 3],
        ]
    }

    /// Writes the 4-byte attribute data of a sprite by index.
    ///
    /// This directly modifies OAM memory and does not emulate
    /// real SNES write timing or port behavior.
    pub fn write_sprite(&mut self, index: usize, sprite: [u8; 4]) {
        let base = index * 4;
        self.mem[base..base + 4].copy_from_slice(&sprite);
    }
}
