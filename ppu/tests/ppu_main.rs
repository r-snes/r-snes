use ppu::ppu::PPU;
use ppu::tile::{load_and_split_image, load_tiles_into_vram};
use ppu::utils::TILE_SIZE;

// This test follows the logic of `main.rs` without depending on minifb
#[test]
fn test_main_logic() {
    let path = "tests/assets/test_10x10.png";

    let (tiles, image_width) = load_and_split_image(path);
    assert!(!tiles.is_empty(), "Tiles should not be empty");

    let mut ppu = PPU::new();
    load_tiles_into_vram(&mut ppu, &tiles);

    let tiles_per_row = image_width / TILE_SIZE as usize;
    assert!(tiles_per_row > 0, "tiles_per_row should be greater than 0");

    ppu.render(tiles_per_row);

    let non_zero_pixels = ppu.framebuffer.iter().filter(|&&px| px != 0).count();
    assert!(non_zero_pixels > 0, "Framebuffer should not be completely empty");
}
