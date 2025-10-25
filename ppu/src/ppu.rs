use crate::utils::render_scanline;
use crate::constants::*;
use crate::sprite::Sprite;
use crate::color::bgr555_to_argb;

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

    pub fn write_vram(&mut self, addr: usize, value: u8) {
        if addr >= VRAM_SIZE {
            eprintln!("[ERR::VRAM] Write to invalid address 0x{:04X}", addr);
            return;
        }
        self.vram[addr] = value;
    }

    pub fn read_vram(&self, addr: usize) -> u8 {
        if addr >= VRAM_SIZE {
            eprintln!("[ERR::VRAM] Read from invalid address 0x{:04X}", addr);
            return 0;
        }
        self.vram[addr]
    }

    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    pub fn set_cgram_addr(&mut self, addr: u8) {
        self.cgaddr = addr;
        self.latch_filled = false;
    }

    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    pub fn write_cgram_data(&mut self, value: u8) {
        if self.latch_filled {
            let color = u16::from_le_bytes([self.latch, value]);
            self.cgram[self.cgaddr as usize] = color & 0x7FFF;
            self.cgaddr = self.cgaddr.wrapping_add(1);
            self.latch_filled = false;
        } else {
            self.latch = value;
            self.latch_filled = true;
        }
    }

    pub fn read_cgram(&self, index: u8) -> u32 {
        bgr555_to_argb(self.cgram[index as usize])
    }

    pub fn set_oam_sprite(&mut self, index: usize, sprite: Sprite) {
        if index >= OAM_MAX_SPRITES {
            eprintln!("[ERR::OAM] index out of range: {}", index);
            return;
        }
        self.oam[index] = sprite;
    }

    pub fn get_oam_sprite(&self, index: usize) -> Option<Sprite> {
        if index >= OAM_MAX_SPRITES {
            None
        } else {
            Some(self.oam[index])
        }
    }

    pub fn render(&mut self) {
        for y in 0..HEIGHT {
            render_scanline(self, y);
        }
    }
}
