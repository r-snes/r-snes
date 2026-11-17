//! BG Mode 1 renderer: two 4bpp layers (BG1, BG2) and one 2bpp layer (BG3).
//!
//! Each pixel reads a tilemap entry (tile number, palette, priority, flips),
//! then decodes the tile data from VRAM.
//! BG1 and BG2 share 8 palettes of 16 colors (CGRAM entries 0-127),
//! BG3 uses 8 palettes of 4 colors (entries 0-31). Color 0 is transparent.
//! BGMODE bit 3 moves high priority BG3 tiles in front of every other layer.

use crate::ppu::PPU;
use crate::rendering::renderer::{BgParams, BitDepth, Layer, Priority, Renderer};

impl Renderer {
    /// Renders BG1-BG3 in Mode 1 on framebuffer row `y`.
    pub fn render_scanline_mode1(&mut self, ppu: &PPU, y: usize) {
        // Mode 1: BG1/BG2 4bpp, BG3 2bpp. No per-layer palette offset.
        // Mode 1 order (front to back), between the sprites (OBJ0-OBJ3 = 3, 6, 9, 12):
        //   OBJ3 > BG1.1 > BG2.1 > OBJ2 > BG1.0 > BG2.0 > OBJ1 >
        //   BG3.1 > OBJ0 > BG3.0 > backdrop
        // BGMODE bit3 lifts BG3 high-priority tiles above every other layer.
        let bg3_high = if ppu.regs.bgmode & 0x08 != 0 {
            Priority::new(13)
        } else {
            Priority::new(5)
        };

        // (bg_index, bpp, z_low, z_high)
        let layers = [
            (0usize, BitDepth::Four, Priority::new(8), Priority::new(11)),
            (1, BitDepth::Four, Priority::new(7), Priority::new(10)),
            (2, BitDepth::Two, Priority::new(2), bg3_high),
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
                    mosaic: ppu.regs.bg_mosaic(bg),
                    z_low,
                    z_high,
                    layer: Layer::from_bg(bg),
                    to_main,
                    to_sub,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::constants::SCREEN_WIDTH;
    use crate::ppu::PPU;
    use crate::rendering::renderer::Renderer;
    use crate::test_utils::fb_pixel;

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
