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

    addr: u16,          // 0..511 (byte address)
    write_latch: u8,    // first byte latch
    write_phase: bool,  // false = expecting low byte
}

impl Oam {
    /// Creates a new OAM instance with cleared memory and address set to 0.
    pub fn new() -> Self {
        Self {
            low: [0; 512],
            high: [0; 32],
            addr: 0,
            write_latch: 0,
            write_phase: false,
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
        self.write_phase = false;
    }

    /// Writes a byte to OAM through the data port ($2104).
    ///
    /// OAM writes are buffered:
    /// - The first write stores the value in an internal latch
    /// - The second write commits the latched byte to OAM
    /// - Only after the commit does the OAM address increment
    ///
    /// The address increments modulo 512 and always refers
    /// to the low table. The high table is not directly
    /// addressable via this port.
    pub fn write_data(&mut self, value: u8) {
        if !self.write_phase {
            // first byte: latch only
            self.write_latch = value;
            self.write_phase = true;
        } else {
            let addr = self.addr as usize;

            if addr < 512 {
                self.low[addr] = self.write_latch;
            }

            self.addr = (self.addr + 1) & 0x1FF;
            self.write_phase = false;
        }
    }

    /// Reads a byte from OAM through the data port ($2138).
    ///
    /// Reads return sequential OAM data starting from the
    /// current address. After each read, the address increments
    /// modulo 512.
    ///
    /// Although the internal read sequence may access the high
    /// table, the CPU-visible address space always wraps at 512.
    pub fn read_data(&mut self) -> u8 {
        let addr = self.addr as usize;

        let value = if addr < 512 {
            self.low[addr]
        } else {
            self.high[addr - 512]
        };

        self.addr = (self.addr + 1) & 0x1FF;
        value
    }

    // ---------- helpers ----------

    /// Reads the 4-byte attribute data of a sprite by index.
    ///
    /// This bypasses PPU port behavior and is intended for debugging
    /// or testing purposes only.
    pub fn read_sprite(&self, index: usize) -> [u8; 6] {
        let base = index * 4;

        let y = self.low[base];
        let tile = self.low[base + 1];
        let attr = self.low[base + 2];
        let x_low = self.low[base + 3];

        let hi = self.high[index / 4];
        let shift = (index % 4) * 2;

        let x_msb = (hi >> shift) & 1;
        let size = (hi >> (shift + 1)) & 1;

        let x = ((x_msb as u16) << 8) | x_low as u16;

        [
            y,
            tile,
            attr,
            x as u8,
            (x >> 8) as u8, // MSB
            size,
        ]
    }

    /// Writes the 4-byte attribute data of a sprite by index.
    ///
    /// This directly modifies OAM memory and does not emulate
    /// real SNES write timing or port behavior.
    pub fn write_sprite(&mut self, index: usize, sprite: [u8; 4], x_msb: bool, size: bool) {
        let base = index * 4;
        self.low[base..base + 4].copy_from_slice(&sprite);

        let hi_index = index / 4;
        let shift = (index % 4) * 2;

        self.high[hi_index] &= !(0b11 << shift);
        self.high[hi_index] |= ((x_msb as u8) | ((size as u8) << 1)) << shift;
    }
}
