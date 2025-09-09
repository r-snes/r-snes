use crate::utils::{render_scanline, CGRAM_SIZE, HEIGHT, VRAM_SIZE, WIDTH};

pub struct PPU {
    pub framebuffer: Vec<u32>,
    vram: [u8; VRAM_SIZE],
    cgram: [u32; CGRAM_SIZE],
}

impl PPU {
    pub fn new() -> Self {
        let mut ppu = Self {
            framebuffer: vec![0; WIDTH * HEIGHT],
            vram: [0; VRAM_SIZE],
            cgram: [0; 256],
        };

        // Hardcoded palette (SNES = 15 bits, here simplified to 32-bit ARGB)
        for i in 0..256 {
            // Example: colorful gradient
            let r = ((i & 0x1F) << 3) as u32;
            let g = (((i >> 2) & 0x1F) << 3) as u32;
            let b = (((i >> 4) & 0x1F) << 3) as u32;
            ppu.cgram[i] = (0xFF << 24) | (r << 16) | (g << 8) | b;
        }

        ppu
    }

    pub fn write_vram(&mut self, addr: usize, value: u8) {
        if addr >= VRAM_SIZE {
            eprintln!("[ERR::VRAM] Write attempt to invalid address 0x{:04X}", addr);
            return;
        }

        self.vram[addr] = value;
    }

    pub fn read_vram(&self, addr: usize) -> u8 {
        if addr >= VRAM_SIZE {
            eprintln!("[ERR::VRAM] Read attempt from invalid address 0x{:04X}", addr);
            return 0;
        }

        return self.vram[addr];
    }

    pub fn read_cgram(&self, index: u8) -> u32 {
        self.cgram[index as usize]
    }

    pub fn render(&mut self, tiles_per_row: usize) {
        for y in 0..HEIGHT {
            render_scanline(self, y, tiles_per_row);
        }
    }
}
