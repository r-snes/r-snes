use minifb::{Window, WindowOptions};
use crate::ppu::PPU;
use crate::tile::get_tile_from_vram;

pub const VRAM_SIZE: usize = 64 * 1024; // 64 KB
pub const TILE_SIZE: usize = 8;
pub const TILES_X: usize = 32;
pub const TILES_Y: usize = 32;
pub const WIDTH: usize = TILES_X * TILE_SIZE;
pub const HEIGHT: usize = TILES_Y * TILE_SIZE;
pub const SCALE: usize = 2;
pub const SCREEN_WIDTH: usize = WIDTH * SCALE;
pub const SCREEN_HEIGHT: usize = HEIGHT * SCALE;

pub fn create_window() -> Window {
    Window::new(
        "rsnes ppu",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions {
            resize: false,
            ..WindowOptions::default()
        },
    )
    .expect("[ERR::WindowInit] Unable to create display context.")
}

pub fn update_window(window: &mut Window, framebuffer: &Vec<u32>) {
    window
        .update_with_buffer(framebuffer, WIDTH, HEIGHT)
        .expect("[ERR::Render] Framebuffer refused to cooperate.");
}

pub fn render_scanline(ppu: &mut PPU, y: usize, tiles_per_row: usize) {
    if y >= HEIGHT {
        return;
    }

    for x in 0..tiles_per_row {
        let tile_index = (y / TILE_SIZE) * tiles_per_row + x;

        let tile_pixels = get_tile_from_vram(ppu, tile_index);

        let tile_line = y % TILE_SIZE;

        for px in 0..TILE_SIZE {
            let screen_x = x * TILE_SIZE + px;
            if screen_x < WIDTH {
                let color = tile_pixels[tile_line * TILE_SIZE + px];
                ppu.framebuffer[y * WIDTH + screen_x] = color;
            }
        }
    }
}