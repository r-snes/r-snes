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

#[cfg(test)]
mod tests_utils {
    use super::*;
    use crate::sprite::Sprite;

    fn create_test_ppu() -> PPU {
        let mut ppu = PPU::new();

        // Fill framebuffer with black
        for pixel in ppu.framebuffer.iter_mut() {
            *pixel = 0xFF000000;
        }

        // Fill VRAM with 0 for predictable tile indices
        for byte in ppu.vram.iter_mut() {
            *byte = 0;
        }

        // Fill CGRAM with a simple gradient
        for i in 0..CGRAM_SIZE {
            ppu.cgram[i] = (i as u16) | ((i as u16) << 5) | ((i as u16) << 10);
        }

        ppu
    }

    #[test] // Rendering a scanline outside screen height should do nothing
    fn test_render_scanline_out_of_bounds() {
        let mut ppu = create_test_ppu();
        render_scanline(&mut ppu, HEIGHT + 10);
        // Framebuffer should remain all black (0xFF000000)
        assert!(ppu.framebuffer.iter().all(|&v| v == 0xFF000000));
    }

    #[test] // Clearing a scanline should fill the correct row with black
    fn test_clear_scanline() {
        let mut ppu = create_test_ppu();
        for i in 0..ppu.framebuffer.len() {
            ppu.framebuffer[i] = 0xFFFFFFFF; // White before clear
        }
        clear_scanline(&mut ppu, 0);
        for x in 0..WIDTH {
            assert_eq!(ppu.framebuffer[x], 0xFF000000);
        }
    }

    #[test] // A sprite not intersecting the scanline should not draw anything
    fn test_sprite_not_on_scanline() {
        let mut ppu = create_test_ppu();

        let sprite = Sprite {
            x: 5,
            y: 20,
            tile: 0,
            attr: 0x00,
            filed: true,
        };
        ppu.set_oam_sprite(0, sprite);

        render_scanline(&mut ppu, 0); // Scanline 0 is far above y=20

        // All pixels should still be black
        assert!(ppu.framebuffer.iter().all(|&v| v == 0xFF000000));
    }

    #[test] // A sprite overlapping the scanline should write colors to framebuffer
    fn test_sprite_draws_on_scanline() {
        let mut ppu = create_test_ppu();

        // Fill VRAM tile 0 with non-zero indices
        for i in 0..(TILE_SIZE * TILE_SIZE) {
            ppu.vram[i] = (i % 8) as u8;
        }

        let sprite = Sprite {
            x: 0,
            y: 0,
            tile: 0,
            attr: 0x00,
            filed: true,
        };
        ppu.set_oam_sprite(0, sprite);

        render_scanline(&mut ppu, 0);

        // Some pixels in first row should have changed color (non-black)
        assert!(ppu.framebuffer[0..TILE_SIZE]
            .iter()
            .any(|&v| v != 0xFF000000));
    }

    #[test] // Horizontally flipped sprites should draw reversed pixels
    fn test_sprite_horizontal_flip() {
        let mut ppu = create_test_ppu();

        // Left to right increasing values
        for i in 0..(TILE_SIZE * TILE_SIZE) {
            ppu.vram[i] = (i % TILE_SIZE) as u8;
        }

        let sprite = Sprite {
            x: 0,
            y: 0,
            tile: 0,
            attr: 0x40, // HFLIP flag
            filed: true,
        };
        ppu.set_oam_sprite(0, sprite);

        render_scanline(&mut ppu, 0);

        let normal_first_pixel = ppu.read_cgram(7); // last pixel in reversed order
        assert_eq!(ppu.framebuffer[0], normal_first_pixel);
    }

    #[test] // Vertically flipped sprites should draw mirrored vertically
    fn test_sprite_vertical_flip() {
        let mut ppu = create_test_ppu();

        for i in 0..(TILE_SIZE * TILE_SIZE) {
            ppu.vram[i] = (i / TILE_SIZE) as u8; // Each row has a different color index
        }

        let sprite = Sprite {
            x: 0,
            y: 0,
            tile: 0,
            attr: 0x80, // VFLIP flag
            filed: true,
        };
        ppu.set_oam_sprite(0, sprite);

        // Normally, first line (y=0) would use first row of indices (0)
        // With vertical flip, it should use the last row instead (index=7)
        render_scanline(&mut ppu, 0);
        let expected_color = ppu.read_cgram(7);
        assert_eq!(ppu.framebuffer[0], expected_color);
    }

    #[test] // Drawing with palette index 0 should skip transparent pixels
    fn test_skip_transparent_pixels() {
        let mut ppu = create_test_ppu();

        // Fill tile with all zeros (transparent)
        for i in 0..(TILE_SIZE * TILE_SIZE) {
            ppu.vram[i] = 0;
        }

        let sprite = Sprite {
            x: 0,
            y: 0,
            tile: 0,
            attr: 0,
            filed: true,
        };
        ppu.set_oam_sprite(0, sprite);

        render_scanline(&mut ppu, 0);

        // All framebuffer should stay black
        assert!(ppu.framebuffer[0..TILE_SIZE]
            .iter()
            .all(|&v| v == 0xFF000000));
    }

    #[test] // Sprites partially off-screen should not cause out-of-bounds writes
    fn test_sprite_partially_offscreen() {
        let mut ppu = create_test_ppu();

        for i in 0..(TILE_SIZE * TILE_SIZE) {
            ppu.vram[i] = 1;
        }

        // Sprite starting partially off left side
        let sprite = Sprite {
            x: -3,
            y: 0,
            tile: 0,
            attr: 0,
            filed: true,
        };
        ppu.set_oam_sprite(0, sprite);

        render_scanline(&mut ppu, 0);

        // Ensure at least one visible pixel drawn
        assert!(ppu.framebuffer.iter().any(|&v| v != 0xFF000000));
    }
}
