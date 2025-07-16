use crate::utils::render_tile_from_vram;
use crate::utils::{WIDTH, HEIGHT, VRAM_SIZE};

pub struct PPU {
    pub framebuffer: Vec<u32>,
    vram: [u8; VRAM_SIZE],
}

impl PPU {
    pub fn new() -> Self {
        Self {
            framebuffer: vec![0; WIDTH * HEIGHT],
            vram: [0; VRAM_SIZE],
        }
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

    pub fn update_framebuffer(&mut self, tiles_per_row: usize, total_tiles: usize) {
        self.framebuffer.fill(0);
        for i in 0..total_tiles {
            let x = i % tiles_per_row;
            let y = i / tiles_per_row;
            render_tile_from_vram(self, i, x, y);
        }
    }
}
