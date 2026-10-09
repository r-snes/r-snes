//! BG Mode 1 renderer: two 4bpp layers (BG1, BG2) and one 2bpp layer (BG3).
//!
//! Each pixel reads a tilemap entry (tile number, palette, priority, flips),
//! then decodes the tile data from VRAM.
//! BG1 and BG2 share 8 palettes of 16 colors (CGRAM entries 0-127),
//! BG3 uses 8 palettes of 4 colors (entries 0-31). Color 0 is transparent.
//! BGMODE bit 3 moves high priority BG3 tiles in front of every other layer.

use crate::ppu::PPU;
use crate::rendering::renderer::{BgParams, BitDepth, Layer, Priority, Renderer};
use crate::vram::RawVRAM;

impl Renderer {
    /// Renders BG1-BG3 in Mode 1 on framebuffer row `y`.
    pub fn render_scanline_mode1(&mut self, ppu: &PPU, y: usize) {
        // Mode 1: BG1/BG2 4bpp, BG3 2bpp. No per-layer palette offset.
        // BGMODE bit3 lifts BG3 high-priority tiles above every other layer.
        let bg3_high = if ppu.regs.bgmode & 0x08 != 0 {
            Priority::Bg3Prio
        } else {
            Priority::Bg3High
        };

        // (bg_index, bpp, z_low, z_high)
        let layers = [
            (0usize, BitDepth::Four, Priority::Bg1Low, Priority::Bg1High),
            (1, BitDepth::Four, Priority::Bg2Low, Priority::Bg2High),
            (2, BitDepth::Two, Priority::Bg3Low, bg3_high),
        ];

        for (bg, bpp, z_low, z_high) in layers {
            let to_main = ppu.regs.tm & (1 << bg) != 0;
            let to_sub = ppu.regs.ts & (1 << bg) != 0;
            if !to_main && !to_sub {
                continue;
            }

            let (w64, h64) = ppu.regs.bg_tilemap_size(bg);
            let (scroll_x, scroll_y) = ppu.regs.bg_scroll(bg);

            self.render_bg_scanline(
                ppu,
                y,
                &BgParams {
                    tilemap_base: ppu.regs.bg_tilemap_addr(bg),
                    tiledata_base: ppu.regs.bg_tiledata_addr(bg),
                    scroll_x,
                    scroll_y,
                    bpp,
                    palette_base: 0,
                    w64,
                    h64,
                    tile16: ppu.regs.bg_tile16(bg),
                    z_low,
                    z_high,
                    layer: Layer::from_bg(bg),
                    to_main,
                    to_sub,
                },
            );
        }
    }

    /// Returns the color index (0-15) of pixel (`x`, `y`) in the 4bpp tile at `tile_word_base`.
    pub fn decode_4bpp_tile_pixel_from(
        vram: &RawVRAM,
        tile_word_base: usize,
        x: usize,
        y: usize,
    ) -> u8 {
        // Planes 0+1: p0 = low byte, p1 = high byte
        let [p0, p1] = vram[(tile_word_base + y) & 0x7FFF].to_le_bytes();

        // Planes 2+3: words 8-15
        let [p2, p3] = vram[(tile_word_base + y + 8) & 0x7FFF].to_le_bytes();

        let bit = 7 - x;
        ((p0 >> bit) & 1)
            | (((p1 >> bit) & 1) << 1)
            | (((p2 >> bit) & 1) << 2)
            | (((p3 >> bit) & 1) << 3)
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

    /// Build a minimal PPU configured for mode 1 with BG1 enabled.
    fn make_ppu_mode1() -> PPU {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0x00); // no force blank, brightness = 0
        ppu.write(0x2105, 0x01); // BG mode 1
        ppu.write(0x212C, 0x01); // BG1 enabled on main screen
        ppu
    }

    /// Mode 1 with BG1 at full brightness: tilemap at 0x0400 filled with `entry`,
    /// CHR at 0x0000 (all zero), palette 0 color 1 = red.
    fn make_ppu_bg1_filled(entry: u16) -> PPU {
        let mut ppu = make_ppu_mode1();
        ppu.write(0x2100, 0x0F);
        ppu.write(0x2107, 0x04);
        for word in 0x0400..0x0800 {
            ppu.vram.memory[word] = entry;
        }
        ppu.cgram.memory[0x01] = 0x001F;
        ppu
    }

    /// RGB of the framebuffer pixel at (x, y).
    fn fb_pixel(r: &Renderer, x: usize, y: usize) -> (u8, u8, u8) {
        let i = (y * SCREEN_WIDTH + x) * 3;
        (r.framebuffer[i], r.framebuffer[i + 1], r.framebuffer[i + 2])
    }

    // ============================================================
    // decode_4bpp_tile_pixel_from
    // ============================================================

    /// All-zero tile data must decode to color index 0 (transparent) for every pixel.
    #[test]
    fn test_decode_4bpp_all_zero_is_transparent() {
        let vram = Box::new([0; _]);
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
        let mut vram = Box::new([0; _]);
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
        let mut vram = Box::new([0; _]);
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
        let mut vram = Box::new([0; _]);
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
        let mut vram = Box::new([0; _]);
        vram[8] = 0x00FF; // planes 2+3 row 0: plane 2 lo = 0xFF, plane 3 hi = 0x00
        for x in 0..8 {
            let idx = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, x, 0);
            assert_eq!(idx, 4, "plane2 only -> color index 4 at x={}", x);
        }
    }

    /// Plane 3 only must contribute bit 3 of the color index.
    #[test]
    fn test_decode_4bpp_plane3_only() {
        let mut vram = Box::new([0; _]);
        vram[8] = 0xFF00; // planes 2+3 row 0: plane 2 lo = 0x00, plane 3 hi = 0xFF
        for x in 0..8 {
            let idx = Renderer::decode_4bpp_tile_pixel_from(&vram, 0, x, 0);
            assert_eq!(idx, 8, "plane3 only -> color index 8 at x={}", x);
        }
    }

    /// Pixels are addressed right-to-left within a byte (bit 7 = x=0, bit 0 = x=7).
    #[test]
    fn test_decode_4bpp_bit_order_right_to_left() {
        let mut vram = Box::new([0; _]);
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
        let mut vram = Box::new([0; _]);
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
        let mut vram = Box::new([0; _]);
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
    // render_scanline_mode1 - transparent pixels
    // ============================================================

    /// A fully transparent tile shows the backdrop (CGRAM 0), not color 0 of its own palette.
    #[test]
    fn test_render_mode1_transparent_tile_shows_backdrop() {
        let green = Renderer::apply_brightness(0x03E0, 15);

        // Every tile: tile 0 (all-zero CHR, so transparent), palette 1.
        let mut ppu = make_ppu_bg1_filled(0x0400);
        ppu.cgram.memory[0x00] = 0x03E0; // backdrop = green
        ppu.cgram.memory[0x10] = 0x001F; // palette 1 color 0 = red, must not show

        let mut r = Renderer::new();
        r.render_scanline(&ppu, 0);
        for x in 0..SCREEN_WIDTH {
            assert_eq!(fb_pixel(&r, x, 0), green, "x={x}");
        }
    }

    // ============================================================
    // render_scanline_mode1 - opaque pixels
    // ============================================================

    /// An opaque tile pixel must write the CGRAM colour (with brightness) to the framebuffer.
    #[test]
    fn test_render_mode1_opaque_pixel_written() {
        let mut renderer = Renderer::new();

        let mut ppu = make_ppu_mode1();
        ppu.write(0x2100, 0x0F); // full brightness so the colour survives compositing

        // Tilemap at 0x0400 (bg1sc=0x04), CHR data at 0x0000
        ppu.write(0x2107, 0x04);
        ppu.vram.memory[0x0400] = 0x0000; // tile 0, palette 0, no flip

        // Tile 0: plane 0 all rows set -> every pixel = color index 1
        for row in 0..8 {
            ppu.vram.memory[row] = 0x00FF;
        }
        // CGRAM palette 0 entry 1 = pure red (BGR555)
        ppu.cgram.memory[0x01] = 0x001F;

        renderer.render_scanline(&ppu, 0);

        let (r, _g, _b) = Renderer::apply_brightness(0x001F, 15);
        assert_eq!(renderer.framebuffer[0], r);
    }

    // ============================================================
    // render_scanline_mode1 - flip_x / flip_y
    // ============================================================

    /// H flip (tilemap bit 14) mirrors the tile horizontally.
    #[test]
    fn test_render_mode1_flip_x_mirrors_pixel() {
        let red = Renderer::apply_brightness(0x001F, 15);
        let black = (0, 0, 0);

        // (tilemap entry, expected at x=0, expected at x=7)
        for (entry, x0, x7) in [(0x0000, black, red), (0x4000, red, black)] {
            let mut ppu = make_ppu_bg1_filled(entry);
            // Tile 0: only the rightmost pixel (x=7) is opaque, on every row.
            for row in 0..8 {
                ppu.vram.memory[row] = 0x0001;
            }
            let mut r = Renderer::new();
            r.render_scanline(&ppu, 0);
            assert_eq!(fb_pixel(&r, 0, 0), x0, "entry {entry:#06X}, x=0");
            assert_eq!(fb_pixel(&r, 7, 0), x7, "entry {entry:#06X}, x=7");
        }
    }

    /// V flip (tilemap bit 15) mirrors the tile vertically.
    /// Framebuffer row y shows BG row y + 1, so rows 6 and 7 show tile rows 7 and 0.
    #[test]
    fn test_render_mode1_flip_y_mirrors_pixel() {
        let red = Renderer::apply_brightness(0x001F, 15);
        let black = (0, 0, 0);

        // (tilemap entry, expected on row 6, expected on row 7)
        for (entry, row6, row7) in [(0x0000, red, black), (0x8000, black, red)] {
            let mut ppu = make_ppu_bg1_filled(entry);
            // Tile 0: only tile row 7 is opaque.
            ppu.vram.memory[7] = 0x00FF;
            let mut r = Renderer::new();
            r.render_scanline(&ppu, 6);
            r.render_scanline(&ppu, 7);
            assert_eq!(fb_pixel(&r, 0, 6), row6, "entry {entry:#06X}, row 6");
            assert_eq!(fb_pixel(&r, 0, 7), row7, "entry {entry:#06X}, row 7");
        }
    }

    // ============================================================
    // render_scanline_mode1 - BG3 palette
    // ============================================================

    /// BG3 is 2bpp: palette p, color c reads CGRAM p * 4 + c (entries 0-31).
    #[test]
    fn test_render_mode1_bg3_palette() {
        let red = Renderer::apply_brightness(0x001F, 15);

        // (palette, tile row data, CGRAM entry)
        // 0x00FF -> color 1, 0xFFFF -> color 3
        for (palette, row_data, entry) in [(1u16, 0x00FFu16, 5usize), (7, 0xFFFF, 31)] {
            let mut ppu = make_ppu_mode1();
            ppu.write(0x2100, 0x0F);
            ppu.write(0x212C, 0x04); // BG3 only on main
            ppu.write(0x2109, 0x04); // BG3 tilemap at 0x0400, CHR at 0x0000
            for word in 0x0400..0x0800 {
                ppu.vram.memory[word] = palette << 10; // tile 0, palette `palette`
            }
            for row in 0..8 {
                ppu.vram.memory[row] = row_data;
            }
            ppu.cgram.memory[entry] = 0x001F;

            let mut r = Renderer::new();
            r.render_scanline(&ppu, 0);
            assert_eq!(fb_pixel(&r, 0, 0), red, "palette {palette}");
        }
    }
}
