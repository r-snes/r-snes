use crate::utils::render_scanline;
use crate::constants::*;
use crate::sprite::Sprite;
use crate::color::bgr555_to_argb;

/// Represents the Picture Processing Unit (PPU) of the SNES
/// 
/// Contains VRAM, CGRAM, OAM (sprites), and the framebuffer
/// Handles tile and sprite rendering
pub struct PPU {
    pub framebuffer: Vec<u32>,
    pub vram: [u8; VRAM_SIZE],
    pub cgram: [u16; CGRAM_SIZE],
    pub oam: [Sprite; OAM_MAX_SPRITES],

    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    pub cgaddr: u8,

    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    pub latch: u8,
    pub latch_filled: bool,
}

impl PPU {
    /// Creates a new PPU instance with initialized VRAM, CGRAM, OAM, and framebuffer
    /// 
    /// # Returns
    /// A `PPU` with default values. The CGRAM is prefilled with a simple hardcoded palette
    pub fn new() -> Self {
        let mut ppu = Self {
            framebuffer: vec![0; WIDTH * HEIGHT],
            vram: [0; VRAM_SIZE],
            cgram: [0; CGRAM_SIZE],
            oam: [Sprite::default(); OAM_MAX_SPRITES],
            cgaddr: 0,
            latch: 0,
            latch_filled: false,
        };

        // Hardcoded palette
        for i in 0..CGRAM_SIZE {
            let r = (i & 0x1F) as u16;
            let g = ((i >> 2) & 0x1F) as u16;
            let b = ((i >> 4) & 0x1F) as u16;
            ppu.cgram[i] = (b << 10) | (g << 5) | r;
        }

        ppu
    }

    /// Writes a byte to VRAM at the given address
    /// 
    /// # Parameters
    /// - `addr`: VRAM address to write to (0..VRAM_SIZE)
    /// - `value`: Value to write.
    pub fn write_vram(&mut self, addr: usize, value: u8) {
        if addr >= VRAM_SIZE {
            eprintln!("[ERR::VRAM] Write to invalid address 0x{:04X}", addr);
            return;
        }
        self.vram[addr] = value;
    }

    /// Reads a byte from VRAM at the given address
    /// 
    /// # Parameters
    /// - `addr`: VRAM address to read from (0..VRAM_SIZE)
    /// 
    /// # Returns
    /// The value stored at the address, or 0 if the address is invalid
    pub fn read_vram(&self, addr: usize) -> u8 {
        if addr >= VRAM_SIZE {
            eprintln!("[ERR::VRAM] Read from invalid address 0x{:04X}", addr);
            return 0;
        }
        self.vram[addr]
    }

    /// Sets the current CGRAM address for future writes
    /// 
    /// # Parameters
    /// - `addr`: The new CGRAM address (0..CGRAM_SIZE)
    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    pub fn set_cgram_addr(&mut self, addr: u8) {
        self.cgaddr = addr;
        self.latch_filled = false;
    }

    /// Writes a byte to CGRAM at the current address
    /// 
    /// # Parameters
    /// - `value`: Byte to write
    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    pub fn write_cgram_data(&mut self, value: u8) {
        if self.latch_filled {
            // Second step: combine the previous latched byte with the new one and write to CGRAM
            let color = u16::from_le_bytes([self.latch, value]);
            self.cgram[self.cgaddr as usize] = color & 0x7FFF;
            self.cgaddr = self.cgaddr.wrapping_add(1);
            self.latch_filled = false;
        } else {
            // First step: store the first byte into the latch
            self.latch = value;
            self.latch_filled = true;
        }
    }

    /// Reads a 32-bit ARGB color from CGRAM
    /// 
    /// # Parameters
    /// - `index`: CGRAM index (0..CGRAM_SIZE)
    /// 
    /// # Returns
    /// A `u32` containing the ARGB color
    pub fn read_cgram(&self, index: u8) -> u32 {
        bgr555_to_argb(self.cgram[index as usize])
    }

    /// Sets a sprite in OAM at the given index
    /// 
    /// # Parameters
    /// - `index`: Sprite index (0..OAM_MAX_SPRITES)
    /// - `sprite`: The sprite data to store
    pub fn set_oam_sprite(&mut self, index: usize, sprite: Sprite) {
        if index >= OAM_MAX_SPRITES {
            eprintln!("[ERR::OAM] index out of range: {}", index);
            return;
        }
        self.oam[index] = sprite;
    }

    /// Retrieves a sprite from OAM by index
    /// 
    /// # Parameters
    /// - `index`: Sprite index (0..OAM_MAX_SPRITES)
    /// 
    /// # Returns
    /// `Some(sprite)` if the index is valid, or `None` if out of range
    pub fn get_oam_sprite(&self, index: usize) -> Option<Sprite> {
        if index >= OAM_MAX_SPRITES {
            None
        } else {
            Some(self.oam[index])
        }
    }

    /// Renders the entire screen by calling `render_scanline` for each row
    pub fn render(&mut self) {
        for y in 0..HEIGHT {
            render_scanline(self, y);
        }
    }
}
