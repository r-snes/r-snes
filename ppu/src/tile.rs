use crate::ppu::PPU;
use crate::constants::*;

#[allow(dead_code)]
pub fn get_tile_from_vram(ppu: &PPU, tile_index: usize) -> Vec<u32> {
    let mut tile_pixels = Vec::new();
    let base_addr = tile_index * TILE_SIZE * TILE_SIZE;

    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            let addr = base_addr + y * TILE_SIZE + x;
            let palette_index = ppu.read_vram(addr);
            let argb = ppu.read_cgram(palette_index);
            tile_pixels.push(argb);
        }
    }

    tile_pixels
}

pub fn get_tile_indices_from_vram(ppu: &PPU, tile_index: usize) -> Vec<u8> {
    let mut indices = Vec::with_capacity(TILE_SIZE * TILE_SIZE);
    let base_addr = tile_index * TILE_SIZE * TILE_SIZE;

    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            let addr = base_addr + y * TILE_SIZE + x;
            indices.push(ppu.read_vram(addr));
        }
    }

    indices
}
