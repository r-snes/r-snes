mod ppu;
mod tile;
mod utils;

use minifb::Key;
use crate::ppu::PPU;
use crate::tile::{load_and_split_image, load_tiles_into_vram};
use crate::utils::{create_window, update_window, TILE_SIZE};

fn main() {
    let (tiles, image_width) = load_and_split_image("tileset.png");
    // hard-coded filepath => to be removed (but ok for pr #13)

    let mut ppu = PPU::new();
    load_tiles_into_vram(&mut ppu, &tiles);

    let tiles_per_row = image_width / TILE_SIZE as usize;

    let mut window = create_window();

    ppu.update_framebuffer(tiles_per_row, tiles.len());

    // hard-coded display => to be removed (but ok for pr #13)
    while window.is_open() && !window.is_key_down(Key::Escape) {
        update_window(&mut window, &ppu.framebuffer);
    }
}
