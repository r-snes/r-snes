use image::{Rgba};
use crate::ppu::PPU;
use crate::utils::{TILE_SIZE};

pub fn load_and_split_image(path: &str) -> Vec<Vec<Rgba<u8>>> {
    let img = image::open(path).expect("[ERR::ImageLoad] Failed to load image file.");
    let img = img.to_rgba8();
    let width = img.width() as usize;
    let height = img.height() as usize;

    let tile_size = TILE_SIZE;

    let tiles_x = (width + tile_size - 1) / tile_size;
    let tiles_y = (height + tile_size - 1) / tile_size;

    let mut tiles = Vec::new();

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let mut tile = Vec::new();

            let x_start = tx * tile_size;
            let y_start = ty * tile_size;
            let x_end = (x_start + tile_size).min(width);
            let y_end = (y_start + tile_size).min(height);

            for y in y_start..y_end {
                for x in x_start..x_end {
                    let px = img.get_pixel(x as u32, y as u32);
                    tile.push(*px);
                }
            }

            tiles.push(tile);
        }
    }

    tiles
}

pub fn load_tiles_into_vram(ppu: &mut PPU, tiles: &Vec<Vec<Rgba<u8>>>) {
    for (tile_index, tile) in tiles.iter().enumerate() {
        let base_addr = tile_index * TILE_SIZE * TILE_SIZE;
        if tile.len() < TILE_SIZE * TILE_SIZE {
            eprintln!(
                "[WARN] Skipping tile {}: only {} pixels, expected {}",
                tile_index,
                tile.len(),
                TILE_SIZE * TILE_SIZE
            );
            continue;
        }

        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                let pixel = &tile[y * TILE_SIZE + x];

                // Tmp: taking the red intensity as a "palette index"
                let value = pixel[0] >> 2; // 0–255 → 0–63 (enough to index a fake palette)

                let addr = base_addr + y * TILE_SIZE + x;
                ppu.write_vram(addr, value);
            }
        }
    }
}

#[allow(dead_code)] // Will be used when VRAM/memory is linked (may need changes later)
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
            let palette_index = ppu.read_vram(addr);
            indices.push(palette_index);
        }
    }

    indices
}
