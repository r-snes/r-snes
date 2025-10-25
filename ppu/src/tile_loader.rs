use image::Rgba;
use crate::ppu::PPU;
use crate::constants::*;

pub fn load_and_split_image(path: &str) -> Vec<Vec<Rgba<u8>>> {
    let img = image::open(path).expect("[ERR::ImageLoad] Failed to load image file.");
    let img = img.to_rgba8();
    let width = img.width() as usize;
    let height = img.height() as usize;

    let tiles_x = (width + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (height + TILE_SIZE - 1) / TILE_SIZE;

    let mut tiles = Vec::new();

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let mut tile = Vec::new();

            for y in (ty * TILE_SIZE)..((ty + 1) * TILE_SIZE).min(height) {
                for x in (tx * TILE_SIZE)..((tx + 1) * TILE_SIZE).min(width) {
                    tile.push(*img.get_pixel(x as u32, y as u32));
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
                let value = pixel[0] >> 2;
                let addr = base_addr + y * TILE_SIZE + x;
                ppu.write_vram(addr, value);
            }
        }
    }
}
