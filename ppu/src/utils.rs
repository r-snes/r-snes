use crate::ppu::PPU;
use crate::tile::get_tile_indices_from_vram;

pub const VRAM_SIZE: usize = 64 * 1024; // 64 KB
pub const CGRAM_SIZE: usize = 256; // 256 color palette
pub const OAM_MAX_SPRITES: usize = 128;
pub const TILE_SIZE: usize = 8;
pub const TILES_X: usize = 32;
pub const TILES_Y: usize = 32;
pub const WIDTH: usize = TILES_X * TILE_SIZE;
pub const HEIGHT: usize = TILES_Y * TILE_SIZE;
pub const SCALE: usize = 2;
pub const SCREEN_WIDTH: usize = WIDTH * SCALE;
pub const SCREEN_HEIGHT: usize = HEIGHT * SCALE;

pub fn render_scanline(ppu: &mut PPU, y: usize) {
    if y >= HEIGHT {
        return;
    }

    // Clear
    for x in 0..WIDTH {
        ppu.framebuffer[y * WIDTH + x] = 0xFF000000;
    }

    for i in 0..OAM_MAX_SPRITES {
        let spr = ppu.get_oam_sprite(i).unwrap();

        if !spr.filed {
            continue;
        }

        let sy = spr.y as isize;
        let sx = spr.x as isize;

        if (y as isize) < sy || (y as isize) >= sy + TILE_SIZE as isize {
            continue;
        }

        let tile_idx = spr.tile as usize;
        let indices = get_tile_indices_from_vram(ppu, tile_idx);

        let hflip = (spr.attr & 0x40) != 0;
        let vflip = (spr.attr & 0x80) != 0;

        let line_in_tile = (y as isize - sy) as usize;
        let line_in_tile = if vflip { TILE_SIZE - 1 - line_in_tile } else { line_in_tile };

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
}
