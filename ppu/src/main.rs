mod ppu;
mod tile;
mod tile_loader;
mod utils;
mod sprite;
mod color;
mod window;
mod constants;

use minifb::Key;
use crate::ppu::PPU;
use crate::tile_loader::{load_and_split_image, load_tiles_into_vram};
use crate::constants::*;
use crate::window::*;
use crate::sprite::Sprite;

fn main() {
    let tiles = load_and_split_image("./tileset.png");
    println!("Loaded {} tiles", tiles.len());

    let mut ppu = PPU::new();
    load_tiles_into_vram(&mut ppu, &tiles);

    let mut window = create_window();

    let test_sprite = Sprite { x: 64, y: 64, tile: 1, attr: 0x01, filed: true };
    ppu.set_oam_sprite(0, test_sprite);

    let test_sprite2 = Sprite { x: 32, y: 32, tile: 27, attr: 0x01, filed: true };
    ppu.set_oam_sprite(1, test_sprite2);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        ppu.render();
        update_window(&mut window, &ppu.framebuffer);
    }
}
