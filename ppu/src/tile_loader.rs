use image::Rgba;
use crate::ppu::PPU;
use crate::constants::*;

/// Loads an image from a file and splits it into tiles of size TILE_SIZE x TILE_SIZE
///
/// This function reads the image at the given path, converts it to RGBA format,
/// and splits it into a vector of tiles. Each tile is a `Vec<Rgba<u8>>` containing
/// the pixel colors in row-major order
///
/// # Parameters
/// - `path`: Path to the image file (supports any format supported by the `image` crate)
///
/// # Returns
/// A `Vec<Vec<Rgba<u8>>>` containing all tiles extracted from the image
/// Each inner vector contains TILE_SIZE * TILE_SIZE pixels
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

/// Loads a list of tiles into the PPU's VRAM
///
/// This function takes each tile (as returned by `load_and_split_image`) and writes
/// its pixel data into VRAM. The pixel's red channel is used as a temporary palette index
///
/// # Parameters
/// - `ppu`: Mutable reference to the PPU instance, where the tiles will be loaded
/// - `tiles`: A vector of tiles, each containing TILE_SIZE * TILE_SIZE pixels in RGBA format
///
/// # Notes
/// - Currently only the red channel of each pixel is used as a fake palette index
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
                let value = pixel[0] >> 2; // Red channel as palette index
                let addr = base_addr + y * TILE_SIZE + x;
                ppu.write_vram(addr, value);
            }
        }
    }
}
