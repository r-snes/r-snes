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

pub fn render_tile_from_vram(ppu: &mut PPU, tile_index: usize, tile_x: usize, tile_y: usize) {
    let tile_pixels = get_tile_from_vram(ppu, tile_index);

    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            let px = tile_x * TILE_SIZE + x;
            let py = tile_y * TILE_SIZE + y;
            if px < WIDTH && py < HEIGHT {
                let color = tile_pixels[y * TILE_SIZE + x];
                ppu.framebuffer[py * WIDTH + px] = color;
            }
        }
    }
}
