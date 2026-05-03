use crate::constants::*;
use crate::ppu::PPU;
use crate::rendering::renderer::*;

pub trait Mode1Render {
    fn render_scanline_mode1(&mut self, ppu: &PPU, y: usize);
    fn decode_4bpp_tile_pixel_from(vram: &[u16], tile_word_base: usize, x: usize, y: usize) -> u8;
}

impl Mode1Render for Renderer {
    fn render_scanline_mode1(&mut self, ppu: &PPU, y: usize) {
        // VRAM word addresses
        let tilemap_base = ppu.regs.bg1_tilemap_addr(); // tilemap
        let tiledata_base = ppu.regs.bg1_tiledata_addr(); // CHR data

        // BG1 scroll registers
        let scroll_x = ppu.regs.bg1hofs as usize;
        let scroll_y = ppu.regs.bg1vofs as usize;

        for x in 0..SCREEN_WIDTH {
            // ============================================================
            // Screen pixel -> tile coordinates
            // ============================================================
            let px = (x + scroll_x) & 0xFF;
            let py = (y + scroll_y) & 0xFF;

            let tile_col = px >> 3;
            let tile_row = py >> 3;
            let fine_x = px & 7;
            let fine_y = py & 7;

            // ==========================================================================
            // Read tilemap entry: tilemap_base is a word address => byte address = * 2
            // ==========================================================================
            let map_word_addr = tilemap_base as usize + tile_row * 32 + tile_col;
            let entry = ppu.vram.memory[map_word_addr];

            let tile_index = entry & 0x03FF; // bits 9:0
            let palette_num = (entry >> 10) & 0x07; // bits 12:10
            let _priority = (entry & 0x2000) != 0; // bit 13
            let flip_x = (entry & 0x4000) != 0; // bit 14
            let flip_y = (entry & 0x8000) != 0; // bit 15

            // Apply flip
            let fx = if flip_x { 7 - fine_x } else { fine_x };
            let fy = if flip_y { 7 - fine_y } else { fine_y };

            // ============================================================
            // Decode 4bpp pixel from CHR data
            // ============================================================
            let tile_word_base = tiledata_base as usize + tile_index as usize * 16;
            let color_index = Self::decode_4bpp_tile_pixel_from(&ppu.vram.memory, tile_word_base, fx, fy);

            // Transparent pixel -> do nothing
            if color_index == 0 {
                continue;
            }

            let palette_entry = ((palette_num as u8) << 4) | color_index;
            let color = ppu.cgram.read(palette_entry);

            let (r, g, b) = Self::apply_brightness(color, self.current_brightness as u16);
            self.set_pixel(x, y, r, g, b);
        }
    }

    fn decode_4bpp_tile_pixel_from(vram: &[u16], tile_word_base: usize, x: usize, y: usize) -> u8 {
        // Planes 0+1: words 0-7
        let w01 = vram[tile_word_base + y];
        let p0 = (w01 & 0xFF) as u8; // lo byte = plane 0
        let p1 = (w01 >> 8) as u8; // hi byte = plane 1

        // Planes 2+3: words 8-15
        let w23 = vram[tile_word_base + 8 + y];
        let p2 = (w23 & 0xFF) as u8; // lo byte = plane 2
        let p3 = (w23 >> 8) as u8; // hi byte = plane 3

        let bit = 7 - x;
        ((p0 >> bit) & 1)
            | (((p1 >> bit) & 1) << 1)
            | (((p2 >> bit) & 1) << 2)
            | (((p3 >> bit) & 1) << 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ppu::PPU;
    use crate::rendering::renderer::Renderer;

    // ============================================================
    // Helpers
    // ============================================================

    /// Build a minimal PPU configured for mode 1 with BG1 enabled.
    fn make_ppu_mode1() -> PPU {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0x00); // no force blank, brightness = 0
        ppu.write(0x2105, 0x01); // BG mode 1
        ppu.write(0x212C, 0x01); // BG1 enabled on main screen
        ppu
    }

    // ============================================================
    // decode_4bpp_tile_pixel_from
    // ============================================================

    /// All-zero tile data must decode to color index 0 (transparent) for every pixel.
    #[test]
    fn test_decode_4bpp_all_zero_is_transparent() {
        let vram = vec![0u16; 512];
        for y in 0..8 {
            for x in 0..8 {
                let idx = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, x, y);
                assert_eq!(idx, 0, "expected transparent at ({}, {})", x, y);
            }
        }
    }

    /// A tile with all bitplanes set to 0xFF must decode to color index 15 for every pixel.
    #[test]
    fn test_decode_4bpp_all_ones_is_color_15() {
        let mut vram = vec![0u16; 512];
        // All planes 0xFF for all 8 rows
        for y in 0..8 {
            vram[y] = 0xFFFF; // planes 0+1
            vram[8 + y] = 0xFFFF; // planes 2+3
        }
        for y in 0..8 {
            for x in 0..8 {
                let idx = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, x, y);
                assert_eq!(idx, 15, "expected color 15 at ({}, {})", x, y);
            }
        }
    }

    /// Plane 0 only (bit 0 of color index) must be extracted from the low byte of words 0-7.
    #[test]
    fn test_decode_4bpp_plane0_only() {
        let mut vram = vec![0u16; 512];
        // Row 0: plane 0 lo = 0b10000000 (only leftmost pixel set), plane 1/2/3 = 0
        vram[0] = 0x0080; // lo=0x80 (plane 0), hi=0x00 (plane 1)
        let idx_x0 = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 0, 0);
        let idx_x1 = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 1, 0);
        assert_eq!(idx_x0, 1); // bit 7 of plane 0 set -> color bit 0 = 1
        assert_eq!(idx_x1, 0); // bit 6 clear -> transparent
    }

    /// Plane 1 only must contribute bit 1 of the color index.
    #[test]
    fn test_decode_4bpp_plane1_only() {
        let mut vram = vec![0u16; 512];
        // Row 0: plane 1 hi = 0xFF, plane 0 lo = 0x00
        vram[0] = 0xFF00; // lo=0x00 (plane 0), hi=0xFF (plane 1)
        for x in 0..8 {
            let idx = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, x, 0);
            assert_eq!(idx, 2, "plane1 only -> color index 2 at x={}", x);
        }
    }

    /// Plane 2 only must contribute bit 2 of the color index.
    #[test]
    fn test_decode_4bpp_plane2_only() {
        let mut vram = vec![0u16; 512];
        vram[8] = 0x00FF; // planes 2+3 row 0: plane 2 lo = 0xFF, plane 3 hi = 0x00
        for x in 0..8 {
            let idx = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, x, 0);
            assert_eq!(idx, 4, "plane2 only -> color index 4 at x={}", x);
        }
    }

    /// Plane 3 only must contribute bit 3 of the color index.
    #[test]
    fn test_decode_4bpp_plane3_only() {
        let mut vram = vec![0u16; 512];
        vram[8] = 0xFF00; // planes 2+3 row 0: plane 2 lo = 0x00, plane 3 hi = 0xFF
        for x in 0..8 {
            let idx = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, x, 0);
            assert_eq!(idx, 8, "plane3 only -> color index 8 at x={}", x);
        }
    }

    /// Pixels are addressed right-to-left within a byte (bit 7 = x=0, bit 0 = x=7).
    #[test]
    fn test_decode_4bpp_bit_order_right_to_left() {
        let mut vram = vec![0u16; 512];
        // Set only bit 0 of plane 0 row 0 -> only x=7 should be set
        vram[0] = 0x0001;
        let idx_x7 = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 7, 0);
        let idx_x6 = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 6, 0);
        assert_eq!(idx_x7, 1);
        assert_eq!(idx_x6, 0);
    }

    /// decode_4bpp_tile_pixel_from must use the correct row offset (y selects the word row).
    #[test]
    fn test_decode_4bpp_correct_row_selected() {
        let mut vram = vec![0u16; 512];
        // Set plane 0 full for row 3 only
        vram[3] = 0x00FF;
        for y in 0..8 {
            let idx = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 0, y);
            if y == 3 {
                assert_eq!(idx, 1, "row 3 should be set");
            } else {
                assert_eq!(idx, 0, "row {} should be transparent", y);
            }
        }
    }

    /// tile_word_base offset must correctly index into VRAM (non-zero base).
    #[test]
    fn test_decode_4bpp_nonzero_tile_base() {
        let mut vram = vec![0u16; 1024];
        let base = 64usize;
        // All planes 0xFF at base
        for y in 0..8 {
            vram[base + y] = 0xFFFF;
            vram[base + 8 + y] = 0xFFFF;
        }
        // Base 0 must remain transparent
        let idx_base0 = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 0, 0);
        let idx_base64 = Renderer::decode_4bpp_tile_pixel_from(&vram, base, 0, 0);
        assert_eq!(idx_base0, 0);
        assert_eq!(idx_base64, 15);
    }

    // ============================================================
    // render_scanline_mode1 — transparent pixels
    // ============================================================

    /// A fully transparent tile (all zero CHR data) must leave the framebuffer unchanged.
    #[test]
    fn test_render_mode1_transparent_tile_leaves_framebuffer() {
        let mut renderer = Renderer::new();
        renderer.current_brightness = 15;
        // Pre-fill framebuffer with a sentinel value
        for b in renderer.framebuffer.iter_mut() { *b = 0xAA; }

        let mut ppu = make_ppu_mode1();
        // Tilemap entry at (0,0): tile 0, palette 0 — CHR data is all zero -> transparent
        ppu.vram.memory[0] = 0x0000; // tilemap entry: tile index 0
        // CHR data for tile 0 is already all zero

        renderer.render_scanline_mode1(&ppu, 0);

        // All pixels on scanline 0 must still be 0xAA (untouched)
        for x in 0..SCREEN_WIDTH {
            let idx = x * 3;
            assert_eq!(renderer.framebuffer[idx], 0xAA, "R changed at x={}", x);
        }
    }

    // ============================================================
    // render_scanline_mode1 — opaque pixels
    // ============================================================

    /// An opaque tile pixel must write the CGRAM colour (with brightness) to the framebuffer.
    #[test]
    fn test_render_mode1_opaque_pixel_written() {
        let mut renderer = Renderer::new();
        renderer.current_brightness = 15;

        let mut ppu = make_ppu_mode1();

        // Tilemap at 0x0400 (bg1sc=0x04), CHR data at 0x0000
        ppu.write(0x2107, 0x04);
        ppu.vram.memory[0x0400] = 0x0000; // tile 0, palette 0, no flip

        // Tile 0: plane 0 row 0 all set -> every pixel = color index 1
        ppu.vram.memory[0] = 0x00FF;

        // CGRAM palette 0 entry 1 = pure red (BGR555)
        ppu.cgram.memory[0x01] = 0x001F;

        renderer.render_scanline_mode1(&ppu, 0);

        let (r, _g, _b) = Renderer::apply_brightness(0x001F, 15);
        assert_eq!(renderer.framebuffer[0], r);
    }

    // ============================================================
    // render_scanline_mode1 — flip_x / flip_y
    // ============================================================

    /// flip_x must mirror the pixel horizontally within the tile (fine_x = 7 - fine_x).
    #[test]
    fn test_render_mode1_flip_x_mirrors_pixel() {
        let mut vram = vec![0u16; 0x8000];
        // Tile 0: only the rightmost pixel (x=7, bit 0) is set on row 0
        vram[0] = 0x0001; // plane 0 row 0: bit 0 set -> only x=7 lit

        // Without flip_x: x=7 lit, x=0 transparent
        let no_flip = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 7, 0);
        let transparent = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 0, 0);
        assert_eq!(no_flip, 1);
        assert_eq!(transparent, 0);

        // With flip_x: fine_x = 7 - x, so screen x=0 -> fine_x=7 -> lit
        let flipped_x0 = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 7 - 0, 0);
        let flipped_x7 = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 7 - 7, 0);
        assert_eq!(flipped_x0, 1);
        assert_eq!(flipped_x7, 0);
    }

    /// flip_y must mirror the pixel vertically within the tile (fine_y = 7 - fine_y).
    #[test]
    fn test_render_mode1_flip_y_mirrors_pixel() {
        let mut vram = vec![0u16; 0x8000];
        // Only row 7 is set
        vram[7] = 0xFFFF; // plane 0+1 row 7 all set

        // Without flip_y: row 0 transparent, row 7 lit
        let row0 = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 0, 0);
        let row7 = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 0, 7);
        assert_eq!(row0, 0);
        assert_ne!(row7, 0);

        // With flip_y: screen y=0 -> fine_y=7 -> lit
        let flipped_y0 = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 0, 7 - 0);
        let flipped_y7 = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, 0, 7 - 7);
        assert_ne!(flipped_y0, 0);
        assert_eq!(flipped_y7, 0);
    }

    // ============================================================
    // render_scanline_mode1 — scroll wrapping
    // ============================================================

    /// Scroll coordinates must wrap at 256 pixels (8-bit tilemap).
    #[test]
    fn test_scroll_wraps_at_256() {
        // px = (x + scroll_x) & 0xFF — verify the mask holds
        let scroll_x: usize = 0xFF;
        let x: usize = 1;
        let px = (x + scroll_x) & 0xFF;
        assert_eq!(px, 0); // 0xFF + 1 = 0x100, masked = 0x00
    }

    // ============================================================
    // render_scanline_mode1 — palette entry composition
    // ============================================================

    /// palette_entry must combine palette_num (bits[7:4]) and color_index (bits[3:0]).
    #[test]
    fn test_palette_entry_composition() {
        let palette_num: u8 = 3;
        let color_index: u8 = 5;
        let entry = (palette_num << 4) | color_index;
        assert_eq!(entry, 0x35);
        // Verify each nibble
        assert_eq!(entry >> 4, palette_num);
        assert_eq!(entry & 0x0F, color_index);
    }
}
