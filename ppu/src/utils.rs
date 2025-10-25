use crate::ppu::PPU;
use crate::tile::get_tile_indices_from_vram;
use crate::constants::*;

pub fn render_scanline(ppu: &mut PPU, y: usize) {
    if y >= HEIGHT {
        return;
    }

    clear_scanline(ppu, y);

    for i in 0..OAM_MAX_SPRITES {
        if let Some(sprite) = ppu.get_oam_sprite(i) {
            if sprite.filed {
                render_sprite_line(ppu, &sprite, y);
            }
        }
    }
}

fn clear_scanline(ppu: &mut PPU, y: usize) {
    for x in 0..WIDTH {
        ppu.framebuffer[y * WIDTH + x] = 0xFF000000;
    }
}

fn render_sprite_line(ppu: &mut PPU, sprite: &crate::sprite::Sprite, y: usize) {
    let sy = sprite.y as isize;
    let sx = sprite.x as isize;

    if (y as isize) < sy || (y as isize) >= sy + TILE_SIZE as isize {
        return;
    }

    let tile_idx = sprite.tile as usize;
    let indices = get_tile_indices_from_vram(ppu, tile_idx);

    let hflip = (sprite.attr & 0x40) != 0;
    let vflip = (sprite.attr & 0x80) != 0;

    let mut line_in_tile = (y as isize - sy) as usize;
    if vflip {
        line_in_tile = TILE_SIZE - 1 - line_in_tile;
    }

    draw_sprite_line(ppu, &indices, sx, y, line_in_tile, hflip);
}

fn draw_sprite_line(
    ppu: &mut PPU,
    indices: &Vec<u8>,
    sx: isize,
    y: usize,
    line_in_tile: usize,
    hflip: bool,
) {
    for px in 0..TILE_SIZE {
        let tile_x = if hflip { TILE_SIZE - 1 - px } else { px };
        let palette_index = indices[line_in_tile * TILE_SIZE + tile_x];

        if palette_index == 0 {
            continue;
        }

        let color = ppu.read_cgram(palette_index);
        let screen_x = sx + px as isize;

        if screen_x >= 0 && (screen_x as usize) < WIDTH {
            ppu.framebuffer[y * WIDTH + (screen_x as usize)] = color;
        }
    }
}
