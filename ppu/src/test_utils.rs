//! Helpers shared by the PPU tests.

use crate::constants::SCREEN_WIDTH;
use crate::ppu::PPU;
use crate::rendering::renderer::Renderer;

// Writes sprite `i`'s table-1 entry (X low, Y, tile, attributes) through $2102-$2104.
pub fn write_sprite(ppu: &mut PPU, i: u16, x: u8, y: u8, tile: u8, attr: u8) {
    let word = i * 2;
    ppu.write(0x2102, (word & 0xFF) as u8);
    ppu.write(0x2103, ((word >> 8) & 0x01) as u8);
    ppu.write(0x2104, x);
    ppu.write(0x2104, y);
    ppu.write(0x2104, tile);
    ppu.write(0x2104, attr);
}

// Sprite 0 at (0, 0) with priority `prio`, tile 0 full of color 1 (CGRAM 129 = `color`).
// The other sprites are hidden. `objsel` sets the sprite CHR base (bits 0-2, 0x2000-word steps).
pub fn setup_sprite_at_origin(ppu: &mut PPU, objsel: u8, prio: u8, color: u16) {
    ppu.write(0x2101, objsel);
    for i in 0..128 {
        write_sprite(ppu, i, 0, 0xF0, 0, 0);
    }
    write_sprite(ppu, 0, 0, 0, 0, prio << 4);

    let chr = ((objsel & 0x07) as usize) << 13;
    for row in 0..8 {
        ppu.vram.memory[chr + row] = 0x00FF;
    }
    set_color(ppu, 129, color);
}

// Writes a BGR555 color into CGRAM `entry` through $2121/$2122.
pub fn set_color(ppu: &mut PPU, entry: u8, color: u16) {
    ppu.write(0x2121, entry);
    ppu.write(0x2122, (color & 0xFF) as u8);
    ppu.write(0x2122, (color >> 8) as u8);
}

// RGB of the framebuffer pixel at (x, y).
pub fn fb_pixel(r: &Renderer, x: usize, y: usize) -> (u8, u8, u8) {
    let i = (y * SCREEN_WIDTH + x) * 3;
    (r.framebuffer[i], r.framebuffer[i + 1], r.framebuffer[i + 2])
}
