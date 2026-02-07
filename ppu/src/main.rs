mod ppu;
mod tile;
mod utils;

use crate::ppu::PPU;
use crate::tile::{load_and_split_image, load_tiles_into_vram};
use crate::utils::{HEIGHT, SCREEN_HEIGHT, SCREEN_WIDTH, TILE_SIZE, WIDTH};
use minifb::{Key, Window, WindowOptions};

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

fn main() {
    let (tiles, image_width) = load_and_split_image("./tileset.png");
    println!("Loaded {} tiles, image width: {}", tiles.len(), image_width);
    // hard-coded filepath => to be removed (but ok for pr #13)

    let mut ppu = PPU::new();
    load_tiles_into_vram(&mut ppu, &tiles);

    let tiles_per_row = image_width / TILE_SIZE as usize;

    let mut window = create_window();

    // hard-coded display => to be removed (but ok for pr #13)
    while window.is_open() && !window.is_key_down(Key::Escape) {
        ppu.render(tiles_per_row);
        update_window(&mut window, &ppu.framebuffer);
    }
}
