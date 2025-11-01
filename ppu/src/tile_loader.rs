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

#[cfg(test)]
mod tests_tile_loader {
    use super::*;
    use image::{ImageBuffer, RgbaImage};
    use std::fs;

    // Helper: creates a temporary image file for testing
    fn create_test_image(path: &str, width: u32, height: u32, color: [u8; 4]) {
        let img: RgbaImage = ImageBuffer::from_fn(width, height, |_x, _y| {
            image::Rgba(color)
        });
        img.save(path).unwrap();
    }

    #[test] // Splitting an image smaller than one tile should return a single tile
    fn test_load_and_split_small_image() {
        let path = "test_small.png";
        create_test_image(path, 4, 4, [255, 0, 0, 255]);

        let tiles = load_and_split_image(path);
        assert_eq!(tiles.len(), 1);
        assert_eq!(tiles[0].len(), 16); // 4x4 pixels

        fs::remove_file(path).unwrap();
    }

    #[test] // Splitting an image that spans multiple tiles should return multiple tiles
    fn test_load_and_split_multiple_tiles() {
        let path = "test_large.png";
        create_test_image(path, (TILE_SIZE * 2) as u32, (TILE_SIZE * 2) as u32, [0, 255, 0, 255]);

        let tiles = load_and_split_image(path);
        assert_eq!(tiles.len(), 4); // 2x2 tiles
        assert_eq!(tiles[0].len(), TILE_SIZE * TILE_SIZE);

        fs::remove_file(path).unwrap();
    }

    #[test] // Loading an image with uneven dimensions should still fill all tiles
    fn test_load_and_split_uneven_size() {
        let path = "test_uneven.png";
        create_test_image(path, 10, 10, [0, 0, 255, 255]);

        let tiles = load_and_split_image(path);
        // (10 + 7) / 8 => 2 tiles in both directions = 4 tiles total
        assert_eq!(tiles.len(), 4);
        // Last tiles may be partially filled
        assert!(tiles[3].len() <= TILE_SIZE * TILE_SIZE);

        fs::remove_file(path).unwrap();
    }

    #[test] // Loading tiles into VRAM should correctly write red-channel values
    fn test_load_tiles_into_vram_basic() {
        let mut ppu = PPU::new();

        // Create one tile filled with red=128 (>>2 => 32)
        let pixel = image::Rgba([128, 0, 0, 255]);
        let tile = vec![pixel; TILE_SIZE * TILE_SIZE];
        let tiles = vec![tile];

        load_tiles_into_vram(&mut ppu, &tiles);

        // Verify VRAM contents
        assert_eq!(ppu.read_vram(0), 32);
    }

    #[test] // Tiles smaller than expected should be skipped safely
    fn test_load_tiles_into_vram_skips_small_tile() {
        let mut ppu = PPU::new();

        // Create invalid tile with too few pixels
        let bad_tile = vec![image::Rgba([255, 255, 255, 255]); 10];
        let tiles = vec![bad_tile];

        load_tiles_into_vram(&mut ppu, &tiles);

        // VRAM should remain all zeros
        assert!(ppu.vram.iter().all(|&v| v == 0));
    }

    #[test] // Multiple tiles should fill VRAM sequentially without overlap
    fn test_load_tiles_into_vram_multiple_tiles() {
        let mut ppu = PPU::new();

        // Tile 0 = red (value 63), Tile 1 = green (value 0)
        let red_tile = vec![image::Rgba([255, 0, 0, 255]); TILE_SIZE * TILE_SIZE];
        let green_tile = vec![image::Rgba([0, 255, 0, 255]); TILE_SIZE * TILE_SIZE];
        let tiles = vec![red_tile, green_tile];

        load_tiles_into_vram(&mut ppu, &tiles);

        // First tile starts at address 0
        assert_eq!(ppu.read_vram(0), 63);
        // Second tile starts right after first
        let second_tile_start = TILE_SIZE * TILE_SIZE;
        assert_eq!(ppu.read_vram(second_tile_start), 0);
    }
}
