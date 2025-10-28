mod ppu;
mod tile;
mod utils;

use minifb::{Key, Window, WindowOptions};
use crate::ppu::PPU;
use crate::tile::{load_and_split_image, load_tiles_into_vram};
use crate::utils::{SCREEN_WIDTH, SCREEN_HEIGHT, WIDTH, HEIGHT};

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
    let tiles = load_and_split_image("./tileset.png");
    println!("Loaded {} tiles", tiles.len());

    let mut ppu = PPU::new();
    load_tiles_into_vram(&mut ppu, &tiles);

    let mut window = create_window();

    let test_sprite = crate::ppu::Sprite {
        x: 64,
        y: 64,
        tile: 1, // tile index into VRAM
        attr: 0x01, // palette = 1 (avoids full transparency)
        filed: true,
    };
    ppu.set_oam_sprite(0, test_sprite);

    let test_sprite2 = crate::ppu::Sprite {
        x: 32,
        y: 32,
        tile: 27, // tile index into VRAM
        attr: 0x01, // palette = 1 (avoids full transparency)
        filed: true,
    };
    ppu.set_oam_sprite(1, test_sprite2);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        ppu.render();
        update_window(&mut window, &ppu.framebuffer);
    }
}
