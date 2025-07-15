mod ppu;
mod tile;
mod utils;

use minifb::Key;
use std::time::{Duration, Instant};

use crate::ppu::PPU;
use crate::tile::load_and_split_image;
use crate::utils::{create_window, update_window};

fn main() {
    let tiles = load_and_split_image("tileset.png");

    let mut ppu = PPU::new();
    ppu.load_tiles_into_vram(&tiles);

    let mut window = create_window();

    let mut last_time = Instant::now();
    let mut current_tile_index = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if last_time.elapsed() >= Duration::new(1, 0) {
            current_tile_index = (current_tile_index + 1) % tiles.len();
            last_time = Instant::now();

            // clean
            ppu.framebuffer.fill(0);

            ppu.render_tile_from_vram(current_tile_index, 0, 0);
        }

        update_window(&mut window, &ppu.framebuffer);
    }
}
