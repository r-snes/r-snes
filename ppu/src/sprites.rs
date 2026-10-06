//! Sprite (OBJ) rendering for one scanline.
//!
//! Uses the OAM evaluation to get the sprites on the line, then fetches their
//! tiles from the last evaluated sprite to the first, at most 34 tiles per line.
//! The tiles are first combined into a single OBJ line, where the first-evaluated
//! sprite wins over the others regardless of priority, then that line is compared
//! with the backgrounds. Sprites are 4bpp, use the CGRAM entries 128-255 and have
//! a priority (0-3). Multi-tile sprites wrap the tile number per nibble
//! (X in the low nibble, Y in the high nibble).

use crate::constants::*;
use crate::oam::OAM;
use crate::ppu::PPU;
use crate::rendering::renderer::{Layer, Priority, Renderer};

// VRAM is 32768 words, sprite CHR addresses wrap within it.
const VRAM_WORD_MASK: usize = (VRAM_SIZE / 2) - 1;

// Maximum number of sprite tiles (8-pixel slices) fetched per scanline.
const MAX_TILES_PER_LINE: usize = 34;

// One pixel of the OBJ line: color, priority, color math enabled.
type ObjPixel = (u16, Priority, bool);

impl Renderer {
    /// Render all visible sprites on scanline `y`
    pub fn render_sprites(&mut self, ppu: &PPU, y: usize) {
        // OBJ enable on main / sub screen (TM / TS bit 4).
        let to_main = ppu.regs.tm & 0x10 != 0;
        let to_sub = ppu.regs.ts & 0x10 != 0;
        if !to_main && !to_sub {
            return;
        }

        let objsel = ppu.regs.objsel;
        let oamadd = ppu.regs.oamadd;

        let (sprites, _time_over, _range_over) =
            ppu.oam.eval_sprites_for_scanline(y, objsel, oamadd);

        // OBJ line: one pixel per column, before comparison with the backgrounds.
        let mut obj_line: [Option<ObjPixel>; SCREEN_WIDTH] = [None; SCREEN_WIDTH];

        // Tile fetch: from the last evaluated sprite to the first, only tiles with
        // X in -7..255 (all tiles at X = -256, quirk), at most 34 per line. Past the
        // limit, the remaining tiles (those of the first-evaluated sprites) are dropped.
        // Each opaque pixel overwrites the OBJ line, so the first-evaluated sprite wins.
        let mut fetched = 0;
        'fetch: for &(_idx, sprite) in sprites.iter().rev() {
            let (w, h) = OAM::sprite_size(objsel, sprite.large);
            let w = w as usize;
            let h = h as usize;

            // Row within the sprite for this scanline (0..h), with V flip.
            let mut sy = (y as u8).wrapping_sub(sprite.y) as usize;
            if sprite.flip_y {
                sy = h - 1 - sy;
            }
            let tile_row = sy / 8;
            let fine_y = sy % 8;

            let prio = match sprite.priority {
                0 => Priority::Obj0,
                1 => Priority::Obj1,
                2 => Priority::Obj2,
                _ => Priority::Obj3,
            };

            // Sprites do color math only when using palettes 4-7.
            let obj_math = sprite.palette >= 4;

            let x9 = (sprite.x as u16 & 0x01FF) as usize;
            let tiles_wide = w / 8;

            for tx in 0..tiles_wide {
                // Screen X of this tile (9-bit, wraps around).
                let tile_x = (x9 + tx * 8) & 0x01FF;
                // Off-screen tiles are skipped, except at X = -256 where they are still
                // fetched (and use up the 34 slots) but nothing is drawn.
                if x9 != 256 && tile_x >= 256 && tile_x + 7 < 512 {
                    continue;
                }
                if fetched == MAX_TILES_PER_LINE {
                    break 'fetch;
                }
                fetched += 1;

                // Tile column within the sprite, with H flip (the whole sprite is mirrored).
                let tile_col = if sprite.flip_x {
                    tiles_wide - 1 - tx
                } else {
                    tx
                };

                // Multi-tile sprites wrap the low nibble (X) and high nibble (Y) of the tile number independently.
                let base = sprite.tile as usize;
                let tile_x_num = ((base & 0x0F) + tile_col) & 0x0F;
                let tile_y_num = ((base & 0xF0) + tile_row * 0x10) & 0xF0;
                let tile_num = tile_y_num | tile_x_num;

                let tile_word_base = (sprite.chr_base as usize + tile_num * 16) & VRAM_WORD_MASK;

                // Left edge of the tile on screen: 505-511 are the partially visible -7..-1,
                // 256-504 (only reachable at X = -256) stay fully off-screen.
                let screen_base = if tile_x >= 256 {
                    tile_x as isize - 512
                } else {
                    tile_x as isize
                };

                for px in 0..8 {
                    let screen_x = screen_base + px as isize;
                    if !(0..SCREEN_WIDTH as isize).contains(&screen_x) {
                        continue;
                    }

                    let fine_x = if sprite.flip_x { 7 - px } else { px };
                    let color_index = Self::decode_4bpp_tile_pixel_from(
                        &ppu.vram.memory,
                        tile_word_base,
                        fine_x,
                        fine_y,
                    );

                    if color_index == 0 {
                        continue;
                    }

                    let palette_entry = 128 + sprite.palette * 16 + color_index;
                    let color = ppu.cgram.read(palette_entry);
                    obj_line[screen_x as usize] = Some((color, prio, obj_math));
                }
            }
        }

        // Compare the OBJ line with the backgrounds.
        for (x, pixel) in obj_line.iter().enumerate() {
            if let Some((color, prio, obj_math)) = *pixel {
                if to_main {
                    self.deposit_main(x, color, prio, Layer::Obj, obj_math);
                }
                if to_sub {
                    self.deposit_sub(x, color, prio, Layer::Obj, obj_math);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::constants::SCREEN_WIDTH;
    use crate::ppu::PPU;
    use crate::rendering::renderer::Renderer;

    // ============================================================
    // Helpers
    // ============================================================

    // Writes sprite `i`'s table-1 entry (X low, Y, tile, attributes) through $2102-$2104.
    fn write_sprite(ppu: &mut PPU, i: u16, x: u8, y: u8, tile: u8, attr: u8) {
        let word = i * 2;
        ppu.write(0x2102, (word & 0xFF) as u8);
        ppu.write(0x2103, ((word >> 8) & 0x01) as u8);
        ppu.write(0x2104, x);
        ppu.write(0x2104, y);
        ppu.write(0x2104, tile);
        ppu.write(0x2104, attr);
    }

    // Writes the first bytes of OAM table 2 (2 bits per sprite: X bit 8, large).
    fn write_oam_high_table(ppu: &mut PPU, bytes: &[u8]) {
        ppu.write(0x2102, 0x00);
        ppu.write(0x2103, 0x01); // word 256 -> byte 512
        for &b in bytes {
            ppu.write(0x2104, b);
        }
    }

    fn set_color(ppu: &mut PPU, entry: u8, color: u16) {
        ppu.write(0x2121, entry);
        ppu.write(0x2122, (color & 0xFF) as u8);
        ppu.write(0x2122, (color >> 8) as u8);
    }

    // Mode 1, OBJ only on main, full brightness, all sprites below the screen,
    // 8x8 / 64x64 sizes, CHR at 0x0000 with tiles 0-7 fully opaque (color 1),
    // sprite palette 0 color 1 = red.
    fn make_ppu_sprites() -> PPU {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0x0F);
        ppu.write(0x2105, 0x01);
        ppu.write(0x212C, 0x10); // OBJ on main
        ppu.write(0x2101, 2 << 5); // OBJSEL: small 8x8, large 64x64, CHR base 0
        for i in 0..128 {
            write_sprite(&mut ppu, i, 0, 0xE0, 0, 0);
        }
        for tile in 0..8 {
            for row in 0..8 {
                ppu.vram.memory[tile * 16 + row] = 0x00FF; // plane 0 -> color index 1
            }
        }
        set_color(&mut ppu, 129, 0x001F); // CGRAM 128 + palette 0 * 16 + 1
        ppu
    }

    // RGB of the framebuffer pixel at (x, y).
    fn fb_pixel(r: &Renderer, x: usize, y: usize) -> (u8, u8, u8) {
        let i = (y * SCREEN_WIDTH + x) * 3;
        (r.framebuffer[i], r.framebuffer[i + 1], r.framebuffer[i + 2])
    }

    // ============================================================
    // 34-tile limit
    // ============================================================

    /// Tiles are fetched from the last evaluated sprite to the first, at most 34
    /// per line: the first-evaluated sprite loses the tiles past the limit.
    #[test]
    fn test_tiles_beyond_34_are_dropped() {
        let red = Renderer::apply_brightness(0x001F, 15);
        let black = (0, 0, 0);

        // Sprite 0 alone (64px at x=0): all 8 tiles drawn.
        let mut ppu = make_ppu_sprites();
        write_sprite(&mut ppu, 0, 0, 0, 0, 0);
        write_oam_high_table(&mut ppu, &[0b0000_0010]); // sprite 0 large
        let mut r = Renderer::new();
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 8, 0), red);
        assert_eq!(fb_pixel(&r, 16, 0), red);
        assert_eq!(fb_pixel(&r, 63, 0), red);

        // Sprites 1-4 (64px at x=128) are fetched first: 32 tiles. Sprite 0 only
        // gets 2 tiles (x 0-15), the other 6 are dropped.
        for i in 1..5 {
            write_sprite(&mut ppu, i, 128, 0, 0, 0);
        }
        write_oam_high_table(&mut ppu, &[0b1010_1010, 0b0000_0010]); // sprites 0-4 large
        let mut r = Renderer::new();
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 8, 0), red);
        assert_eq!(fb_pixel(&r, 16, 0), black);
        assert_eq!(fb_pixel(&r, 63, 0), black);
        assert_eq!(fb_pixel(&r, 128, 0), red);
    }

    /// A sprite partially off the left edge only draws its visible pixels.
    #[test]
    fn test_sprite_partially_off_left_edge() {
        let red = Renderer::apply_brightness(0x001F, 15);
        let mut ppu = make_ppu_sprites();
        write_sprite(&mut ppu, 0, 252, 0, 0, 0); // x = -4 (X bit 8 set below), 8x8
        write_oam_high_table(&mut ppu, &[0b0000_0001]);
        let mut r = Renderer::new();
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), red);
        assert_eq!(fb_pixel(&r, 3, 0), red);
        assert_eq!(fb_pixel(&r, 4, 0), (0, 0, 0));
    }

    // ============================================================
    // Sprite vs sprite
    // ============================================================

    /// Where sprites overlap, the first-evaluated one wins even with a lower priority.
    #[test]
    fn test_first_sprite_wins_over_higher_priority() {
        let red = Renderer::apply_brightness(0x001F, 15);
        let mut ppu = make_ppu_sprites();
        set_color(&mut ppu, 145, 0x03E0); // palette 1 color 1 = green
        write_sprite(&mut ppu, 0, 0, 0, 0, 0x00); // priority 0, palette 0 (red)
        write_sprite(&mut ppu, 1, 0, 0, 0, 0x32); // priority 3, palette 1 (green)
        let mut r = Renderer::new();
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), red);
    }

    /// The winning sprite keeps its own priority against the backgrounds: a
    /// priority-0 sprite in front hides a priority-3 sprite behind BG1.
    #[test]
    fn test_first_sprite_priority_used_against_bg() {
        let blue = Renderer::apply_brightness(0x7C00, 15);
        let mut ppu = make_ppu_sprites();
        // BG1: tilemap at 0x0400 (all tile 0), CHR at 0x0000 (tile 0 opaque), color 1 = blue.
        ppu.write(0x2107, 0x04);
        ppu.write(0x212C, 0x11); // BG1 + OBJ on main
        set_color(&mut ppu, 1, 0x7C00);
        set_color(&mut ppu, 145, 0x03E0); // palette 1 color 1 = green
        write_sprite(&mut ppu, 0, 0, 0, 0, 0x00); // priority 0: below BG1 low
        write_sprite(&mut ppu, 1, 0, 0, 0, 0x32); // priority 3: above everything
        let mut r = Renderer::new();
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), blue);
    }

    /// Transparent pixels of the first sprite let the sprites behind show through.
    #[test]
    fn test_transparent_pixels_dont_hide_sprites() {
        let green = Renderer::apply_brightness(0x03E0, 15);
        let mut ppu = make_ppu_sprites();
        set_color(&mut ppu, 145, 0x03E0); // palette 1 color 1 = green
        write_sprite(&mut ppu, 0, 0, 0, 8, 0x00); // tile 8: fully transparent
        write_sprite(&mut ppu, 1, 0, 0, 0, 0x02); // palette 1 (green)
        let mut r = Renderer::new();
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), green);
    }
}
