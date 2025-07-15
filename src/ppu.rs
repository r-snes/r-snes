use image::Rgba;
use crate::utils::{WIDTH, HEIGHT, VRAM_SIZE, TILE_SIZE};

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
            eprintln!("PPU: can't write to 0x{:04X} (invalid address)", addr);
            return;
        }

        self.vram[addr] = value;
    }

    pub fn read_vram(&self, addr: usize) -> u8 {
        if addr >= VRAM_SIZE {
            eprintln!("PPU: can't read from 0x{:04X} (invalid address)", addr);
            return 0;
        }

        return self.vram[addr];
    }

    pub fn load_tiles_into_vram(&mut self, tiles: &Vec<Vec<Rgba<u8>>>) {
        for (tile_index, tile) in tiles.iter().enumerate() {
            let base_addr = tile_index * (TILE_SIZE as usize).pow(2);

            for y in 0..(TILE_SIZE as usize) {
                for x in 0..(TILE_SIZE as usize) {
                    let pixel = &tile[y * TILE_SIZE as usize + x];

                    // tmp 4 bits red color saving
                    let value = pixel[0] / 16;

                    let addr = base_addr + y * TILE_SIZE as usize + x;
                    self.write_vram(addr, value);
                }
            }
        }
    }

    pub fn get_tile_from_vram(&self, tile_index: usize) -> Vec<u32> {
        let mut tile_pixels = Vec::new();
        let base_addr = tile_index * TILE_SIZE * TILE_SIZE;

        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                let addr = base_addr + y * TILE_SIZE + x;
                let value = self.read_vram(addr);

                // Fake color (grey scale)
                let color = (value as u32) * 16; // Set back on 8 bits
                let argb = (0xFF << 24) | (color << 16) | (color << 8) | color;
                tile_pixels.push(argb);
            }
        }

        tile_pixels
    }

    pub fn render_tile_from_vram(&mut self, tile_index: usize, tile_x: usize, tile_y: usize) {
        let tile_pixels = self.get_tile_from_vram(tile_index);
        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                let px = tile_x * TILE_SIZE + x;
                let py = tile_y * TILE_SIZE + y;
                if px < WIDTH && py < HEIGHT {
                    let color = tile_pixels[y * TILE_SIZE + x];
                    self.framebuffer[py * WIDTH + px] = color;
                }
            }
        }
    }
}
