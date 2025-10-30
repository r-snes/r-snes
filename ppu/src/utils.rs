use crate::ppu::PPU;
use crate::tile::get_tile_indices_from_vram;
use crate::constants::*;


/// Renders a single scanline (row of pixels) on the screen framebuffer
///
/// # Parameters
/// - `ppu`: The PPU instance containing the VRAM, CGRAM, OAM, and framebuffer
/// - `y`: The vertical index of the scanline to render (0..HEIGHT)
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

/// Clears a single scanline in the framebuffer to the background color
///
/// # Parameters
/// - `ppu`: The PPU instance containing the framebuffer
/// - `y`: The vertical index of the scanline to clear
fn clear_scanline(ppu: &mut PPU, y: usize) {
    for x in 0..WIDTH {
        ppu.framebuffer[y * WIDTH + x] = 0xFF000000; // Solid black
    }
}

/// Renders a single sprite on a specific scanline, if it intersects the line
///
/// This function handles sprite visibility checks, vertical flipping,
/// and retrieves the tile data from VRAM
///
/// # Parameters
/// - `ppu`: The PPU instance
/// - `sprite`: The sprite to render
/// - `y`: The vertical index of the scanline
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

/// Draws a horizontal line of pixels for a sprite onto the framebuffer
///
/// This function handles horizontal flipping, palette lookup,
/// and ensures pixels are drawn within screen boundaries
///
/// # Parameters
/// - `ppu`: The PPU instance containing the framebuffer and CGRAM
/// - `sprite`: The sprite being drawn
/// - `indices`: Tile pixel indices retrieved from VRAM
/// - `sx`: Horizontal position of the sprite on the screen
/// - `y`: Vertical scanline being drawn
/// - `line_in_tile`: The line of the tile to render (0..TILE_SIZE)
/// - `hflip`: Whether the sprite is horizontally flipped
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
