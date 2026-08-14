use crate::constants::*;
use crate::oam::OAM;
use crate::ppu::PPU;
use crate::rendering::renderer::{Renderer, Z_OBJ0, Z_OBJ1, Z_OBJ2, Z_OBJ3};

// VRAM is 32768 words, sprite CHR addresses wrap within it.
const VRAM_WORD_MASK: usize = (VRAM_SIZE / 2) - 1;

impl Renderer {
    /// Render all visible sprites on scanline `y`
    pub fn render_sprites(&mut self, ppu: &PPU, y: usize) {
        let objsel = ppu.regs.objsel;
        let oamadd = ppu.regs.oamadd;

        let (sprites, _time_over, _range_over) =
            ppu.oam.eval_sprites_for_scanline(y, objsel, oamadd);

        // Draw from the last evaluated sprite to the first
        for &(_idx, sprite) in sprites.iter().rev() {
            let (w, h) = OAM::sprite_size(objsel, sprite.large);
            let w = w as usize;
            let h = h as usize;

            // Row within the sprite for this scanline (0..h), with V flip.
            let mut sy = (y as u8).wrapping_sub(sprite.y) as usize;
            if sprite.flip_y {
                sy = h - 1 - sy;
            }

            let z = match sprite.priority {
                0 => Z_OBJ0,
                1 => Z_OBJ1,
                2 => Z_OBJ2,
                _ => Z_OBJ3,
            };

            for col in 0..w {
                let screen_x = sprite.x + col as i16;
                if screen_x < 0 || screen_x >= SCREEN_WIDTH as i16 {
                    continue;
                }

                // Column within the sprite, with H flip.
                let sx = if sprite.flip_x {
                    w - 1 - col
                } else {
                    col
                };

                // Locate the tile inside the sprite and the pixel inside the tile.
                let tile_col = sx / 8;
                let tile_row = sy / 8;
                let fine_x = sx % 8;
                let fine_y = sy % 8;

                // Multi-tile sprites wrap the low nibble (X) and high nibble (Y) of the tile number independently.
                let base = sprite.tile as usize;
                let tx = (base & 0x0F).wrapping_add(tile_col) & 0x0F;
                let ty = (base & 0xF0).wrapping_add(tile_row * 0x10) & 0xF0;
                let tile_num = ty | tx;

                let tile_word_base = (sprite.chr_base as usize + tile_num * 16) & VRAM_WORD_MASK;

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

                let (r, g, b) = Self::apply_brightness(color, self.current_brightness as u16);
                self.set_pixel_z(screen_x as usize, y, r, g, b, z);
            }
        }
    }
}
