use image::{Rgba, ImageBuffer};
use std::path::Path;
use crate::ppu::PPU;
use crate::tile::{load_and_split_image, load_tiles_into_vram, get_tile_from_vram, get_tile_indices_from_vram};
use crate::utils::TILE_SIZE;

// Helper: Creates a 16x16 image with red, green, blue, and yellow (4 different tiles)
fn create_test_image(path: &str) {
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(16, 16);

    for y in 0..16 {
        for x in 0..16 {
            let pixel = if x < 8 && y < 8 {
                Rgba([255u8, 0u8, 0u8, 255u8]) // red
            } else if x >= 8 && y < 8 {
                Rgba([0u8, 255u8, 0u8, 255u8]) // green
            } else if x < 8 && y >= 8 {
                Rgba([0u8, 0u8, 255u8, 255u8]) // blue
            } else {
                Rgba([255u8, 255u8, 0u8, 255u8]) // yellow
            };
            img.put_pixel(x, y, pixel);
        }
    }

    img.save(path).unwrap();
}

#[test] // Should write complete tiles into VRAM
fn test_load_tiles_into_vram_writes_data() {
    let mut ppu = PPU::new();
    let tile = vec![Rgba([64u8, 0u8, 0u8, 255u8]); TILE_SIZE * TILE_SIZE];
    let tiles = vec![tile];

    load_tiles_into_vram(&mut ppu, &tiles);

    // Check VRAM first few pixels
    for i in 0..8 {
        let v = ppu.read_vram(i);
        assert_eq!(v, 16); // 64 >> 2 = 16
    }
}

#[test] // Should skip incomplete tiles and print a warning
fn test_load_tiles_into_vram_skips_incomplete_tiles() {
    let mut ppu = PPU::new();
    let incomplete_tile = vec![Rgba([255u8, 0u8, 0u8, 255u8]); 4]; // smaller than 8x8
    let tiles = vec![incomplete_tile];

    load_tiles_into_vram(&mut ppu, &tiles);

    // VRAM should remain untouched (default 0)
    for i in 0..16 {
        assert_eq!(ppu.read_vram(i), 0);
    }
}

#[test] // Should load 4 tiles of correct size (8x8) from a 16x16 image
fn test_tile_count_and_size() {
    let path = "./src/tests/assets/test_4_colors_in_4_tiles.png";

    if !Path::new(path).exists() {
        create_test_image(path);
    }

    let tiles = load_and_split_image(path);

    assert_eq!(tiles.len(), 4);

    for tile in tiles.iter() {
        assert_eq!(tile.len(), (TILE_SIZE * TILE_SIZE) as usize);
    }
}

#[test] // Should load pixel color values for each tile (red, green, blue, yellow)
fn test_tile_pixel_values() {
    let path = "./src/tests/assets/test_4_colors_in_4_tiles.png";

    if !Path::new(path).exists() {
        create_test_image(path);
    }

    let tiles = load_and_split_image(path);

    // tile 0 : red
    for px in &tiles[0] {
        assert_eq!(px[0], 255);
        assert_eq!(px[1], 0);
        assert_eq!(px[2], 0);
    }

    // tile 1 : green
    for px in &tiles[1] {
        assert_eq!(px[0], 0);
        assert_eq!(px[1], 255);
        assert_eq!(px[2], 0);
    }

    // tile 2 : blue
    for px in &tiles[2] {
        assert_eq!(px[0], 0);
        assert_eq!(px[1], 0);
        assert_eq!(px[2], 255);
    }

    // tile 3 : yellow
    for px in &tiles[3] {
        assert_eq!(px[0], 255);
        assert_eq!(px[1], 255);
        assert_eq!(px[2], 0);
    }
}

#[test] // Should handle small images smaller than one tile (e.g., 4x4)
fn test_image_smaller_than_tile() {
    let path = "./src/tests/assets/test_small.png";
    let mut img = ImageBuffer::new(4, 4);
    for y in 0..4 {
        for x in 0..4 {
            img.put_pixel(x, y, Rgba([10u8, 20u8, 30u8, 255u8]));
        }
    }
    img.save(path).unwrap();

    let tiles = load_and_split_image(path);

    assert_eq!(tiles.len(), 1); // Should still create one tile
    assert_eq!(tiles[0].len(), 16); // 4x4 = 16 pixels, rest is missing

}

#[test] // Should correctly handle images whose dimensions are not divisible by TILE_SIZE
fn test_non_divisible_dimensions() {
    let path = "./src/tests/assets/test_10x10.png";
    let mut img = ImageBuffer::new(10, 10);
    for y in 0..10 {
        for x in 0..10 {
            img.put_pixel(x, y, Rgba([100u8, 100u8, 100u8, 255u8]));
        }
    }
    img.save(path).unwrap();

    let tiles = load_and_split_image(path);
    assert_eq!(tiles.len(), 4);
    assert!(tiles[0].len() <= (TILE_SIZE * TILE_SIZE) as usize);
}

#[test] // Should correctly handle tiles with transparent pixels (alpha = 0)
fn test_transparent_pixels() {
    let path = "./src/tests/assets/test_transparent.png";
    let mut img = ImageBuffer::new(8, 8);

    for y in 0..8 {
        for x in 0..8 {
            img.put_pixel(x, y, Rgba([100u8, 150u8, 200u8, 0u8])); // Transparent pixel
        }
    }

    img.save(path).unwrap();

    let tiles = load_and_split_image(path);
    assert_eq!(tiles.len(), 1);

    for px in &tiles[0] {
        assert_eq!(px[3], 0);
    }
}

#[test] // Should split a 16x8 image into two horizontal 8x8 tiles
fn test_2_horizontal_tiles() {
    let path = "./src/tests/assets/test_2_horizontal.png";
    let mut img = ImageBuffer::new(16, 8);

    for y in 0..8 {
        for x in 0..8 {
            img.put_pixel(x, y, Rgba([255u8, 0u8, 0u8, 255u8])); // red
        }
        for x in 8..16 {
            img.put_pixel(x, y, Rgba([0u8, 255u8, 0u8, 255u8])); // green
        }
    }

    img.save(path).unwrap();

    let tiles = load_and_split_image(path);
    assert_eq!(tiles.len(), 2);

    // Check tile 0 (red)
    for px in &tiles[0] {
        assert_eq!(px[0], 255);
        assert_eq!(px[1], 0);
        assert_eq!(px[2], 0);
    }

    // Check tile 1 (green)
    for px in &tiles[1] {
        assert_eq!(px[0], 0);
        assert_eq!(px[1], 255);
        assert_eq!(px[2], 0);
    }
}

#[test] // Should return the correct tile colors from VRAM and CGRAM
fn test_get_tile_from_vram_returns_colors() {
    let mut ppu = PPU::new();

    // Fill VRAM with palette index 2
    for i in 0..(TILE_SIZE * TILE_SIZE) {
        ppu.write_vram(i, 2);
    }

    let tile = get_tile_from_vram(&ppu, 0);

    assert_eq!(tile.len(), TILE_SIZE * TILE_SIZE);
    assert!(tile.iter().any(|&c| c != 0xFF000000)); // not all pixels black
}

#[test] // Should return palette indices from VRAM
fn test_get_tile_indices_from_vram_returns_indices() {
    let mut ppu = PPU::new();

    for i in 0..(TILE_SIZE * TILE_SIZE) {
        ppu.write_vram(i, (i % 64) as u8);
    }

    let indices = get_tile_indices_from_vram(&ppu, 0);
    assert_eq!(indices.len(), TILE_SIZE * TILE_SIZE);
    assert_eq!(indices[0], 0);
    assert_eq!(indices[1], 1);
    assert_eq!(indices[7], 7);
}
