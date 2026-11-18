//! BG Mode 4 renderer: one 8bpp layer (BG1) and one 2bpp layer (BG2).
//!
//! Each pixel reads a tilemap entry (tile number, palette, priority, flips),
//! then decodes the tile data from VRAM.
//! BG1 uses all 256 CGRAM colors: the palette bits of its tilemap entries are ignored.
//! BG2 uses 8 palettes of 4 colors (entries 0-31). Color 0 is transparent.
//! With direct color, BG1 skips CGRAM: the index and palette bits are the color.
//! BG3 is not drawn in this mode.

use crate::ppu::PPU;
use crate::rendering::renderer::{BgParams, BitDepth, Layer, Priority, Renderer};

impl Renderer {
    /// Renders BG1-BG2 in Mode 4 on framebuffer row `y`.
    pub fn render_scanline_mode4(&mut self, ppu: &PPU, y: usize) {
        // Mode 4 order (front to back), between the sprites (OBJ0-OBJ3 = 3, 6, 9, 12):
        //   OBJ3 > BG1.1 > OBJ2 > BG2.1 > OBJ1 > BG1.0 > OBJ0 > BG2.0 > backdrop

        // (bg_index, bpp, z_low, z_high)
        let layers = [
            (0usize, BitDepth::Eight, Priority::new(5), Priority::new(11)),
            (1, BitDepth::Two, Priority::new(2), Priority::new(8)),
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
    use crate::ppu::PPU;
    use crate::rendering::renderer::Renderer;
    use crate::test_utils::{fb_pixel, fill_tilemap, set_color, setup_sprite_at_origin};

    // ============================================================
    // Helpers
    // ============================================================

    // BG1: tilemap 0x0400, CHR 0x2000. BG2: tilemap 0x0800, CHR 0x3000.
    const BG1_MAP: usize = 0x0400;
    const BG1_CHR: usize = 0x2000;
    const BG2_MAP: usize = 0x0800;
    const BG2_CHR: usize = 0x3000;

    // Mode 4 PPU at full brightness, with the BG1/BG2 addresses above. No layer enabled.
    fn make_ppu_mode4() -> PPU {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0x0F);
        ppu.write(0x2105, 0x04);
        ppu.write(0x2107, 0x04); // BG1SC: tilemap 0x0400
        ppu.write(0x2108, 0x08); // BG2SC: tilemap 0x0800
        ppu.write(0x210B, 0x32); // BG12NBA: BG1 CHR 0x2000, BG2 CHR 0x3000
        ppu
    }

    // BG1 and BG2 with tile 0 full of color 1. BG1 color 1 = red,
    // BG2 palette 1 color 1 (CGRAM 5) = blue.
    fn setup_bg1_bg2(ppu: &mut PPU, bg1_entry: u16, bg2_entry: u16) {
        fill_tilemap(ppu, BG1_MAP, bg1_entry);
        fill_tilemap(ppu, BG2_MAP, bg2_entry);
        for row in 0..8 {
            ppu.vram.memory[BG1_CHR + row] = 0x00FF;
            ppu.vram.memory[BG2_CHR + row] = 0x00FF;
        }
        set_color(ppu, 1, 0x001F);
        set_color(ppu, 5, 0x7C00);
    }

    fn red() -> (u8, u8, u8) {
        Renderer::apply_brightness(0x001F, 15)
    }

    fn blue() -> (u8, u8, u8) {
        Renderer::apply_brightness(0x7C00, 15)
    }

    fn render(ppu: &PPU) -> Renderer {
        let mut r = Renderer::new();
        r.render_scanline(ppu, 0);
        r
    }

    // ============================================================
    // BG1: 8bpp
    // ============================================================

    // Index 129 (planes 0 and 7) is CGRAM 129, whatever the tilemap palette.
    #[test]
    fn test_mode4_bg1_uses_256_colors() {
        let mut ppu = make_ppu_mode4();
        ppu.write(0x212C, 0x01); // TM: BG1
        fill_tilemap(&mut ppu, BG1_MAP, 0x0C00); // tile 0, palette 3 (ignored)
        for row in 0..8 {
            ppu.vram.memory[BG1_CHR + row] = 0x00FF; // plane 0
            ppu.vram.memory[BG1_CHR + 24 + row] = 0xFF00; // plane 7
        }
        set_color(&mut ppu, 129, 0x001F);

        assert_eq!(fb_pixel(&render(&ppu), 0, 0), red());
    }

    // An 8bpp tile is 32 words: tile 1 starts at CHR + 32.
    #[test]
    fn test_mode4_bg1_tile_size() {
        let mut ppu = make_ppu_mode4();
        ppu.write(0x212C, 0x01); // TM: BG1
        fill_tilemap(&mut ppu, BG1_MAP, 0x0001); // tile 1
        for row in 0..8 {
            ppu.vram.memory[BG1_CHR + 32 + row] = 0x00FF;
        }
        set_color(&mut ppu, 1, 0x001F);

        assert_eq!(fb_pixel(&render(&ppu), 0, 0), red());
    }

    // Index 0x07 gives R = 28, and palette 1 adds the next bit of R (30).
    #[test]
    fn test_mode4_bg1_direct_color() {
        let mut ppu = make_ppu_mode4();
        ppu.write(0x212C, 0x01); // TM: BG1
        ppu.write(0x2130, 0x01); // CGWSEL: direct color
        fill_tilemap(&mut ppu, BG1_MAP, 0x0400); // tile 0, palette 1
        for row in 0..8 {
            ppu.vram.memory[BG1_CHR + row] = 0xFFFF; // planes 0 and 1
            ppu.vram.memory[BG1_CHR + 8 + row] = 0x00FF; // plane 2
        }

        assert_eq!(
            fb_pixel(&render(&ppu), 0, 0),
            Renderer::apply_brightness(30, 15)
        );
    }

    // ============================================================
    // BG2: 2bpp
    // ============================================================

    // Palette 1, color 1 reads CGRAM 1 * 4 + 1 = 5.
    #[test]
    fn test_mode4_bg2_is_2bpp() {
        let mut ppu = make_ppu_mode4();
        ppu.write(0x212C, 0x02); // TM: BG2
        setup_bg1_bg2(&mut ppu, 0x0000, 0x0400);

        assert_eq!(fb_pixel(&render(&ppu), 0, 0), blue());
    }

    // A 2bpp tile is 8 words: tile 1 starts at CHR + 8.
    #[test]
    fn test_mode4_bg2_tile_size() {
        let mut ppu = make_ppu_mode4();
        ppu.write(0x212C, 0x02); // TM: BG2
        fill_tilemap(&mut ppu, BG2_MAP, 0x0001); // tile 1
        for row in 0..8 {
            ppu.vram.memory[BG2_CHR + 8 + row] = 0x00FF;
        }
        set_color(&mut ppu, 1, 0x7C00);

        assert_eq!(fb_pixel(&render(&ppu), 0, 0), blue());
    }

    // Direct color only works on 8bpp layers: BG2 still reads CGRAM.
    #[test]
    fn test_mode4_bg2_no_direct_color() {
        let mut ppu = make_ppu_mode4();
        ppu.write(0x212C, 0x02); // TM: BG2
        ppu.write(0x2130, 0x01); // CGWSEL: direct color
        setup_bg1_bg2(&mut ppu, 0x0000, 0x0400);

        assert_eq!(fb_pixel(&render(&ppu), 0, 0), blue());
    }

    // BG3 is not drawn: enabling it in TM draws nothing.
    #[test]
    fn test_mode4_has_no_bg3() {
        let mut ppu = make_ppu_mode4();
        ppu.write(0x212C, 0x04); // TM: BG3
        ppu.write(0x2109, 0x0C); // BG3SC: tilemap 0x0C00 (all tile 0)
        for row in 0..8 {
            ppu.vram.memory[row] = 0x00FF; // BG3 CHR 0x0000: tile 0 opaque
        }
        set_color(&mut ppu, 1, 0x001F);

        assert_eq!(fb_pixel(&render(&ppu), 0, 0), (0, 0, 0));
    }

    // ============================================================
    // Layer order
    // ============================================================

    // OBJ3 > BG1.1 > OBJ2 > BG2.1 > OBJ1 > BG1.0 > OBJ0 > BG2.0.
    #[test]
    fn test_mode4_bg_vs_sprites() {
        let green = Renderer::apply_brightness(0x03E0, 15);

        // (TM, BG1 entry, BG2 entry, sprite priority, expected)
        let cases = [
            (0x11, 0x0000, 0x0000, 0, red()),  // BG1.0 > OBJ0
            (0x11, 0x0000, 0x0000, 1, green),  // OBJ1 > BG1.0
            (0x11, 0x2000, 0x0000, 2, red()),  // BG1.1 > OBJ2
            (0x11, 0x2000, 0x0000, 3, green),  // OBJ3 > BG1.1
            (0x12, 0x0000, 0x0400, 0, green),  // OBJ0 > BG2.0
            (0x12, 0x0000, 0x2400, 1, blue()), // BG2.1 > OBJ1
            (0x12, 0x0000, 0x2400, 2, green),  // OBJ2 > BG2.1
        ];
        for (tm, bg1_entry, bg2_entry, prio, expected) in cases {
            let mut ppu = make_ppu_mode4();
            ppu.write(0x212C, tm);
            setup_bg1_bg2(&mut ppu, bg1_entry, bg2_entry);
            // Sprite 0 at (0, 0), color 1 = green.
            setup_sprite_at_origin(&mut ppu, 0x00, prio, 0x03E0);

            assert_eq!(
                fb_pixel(&render(&ppu), 0, 0),
                expected,
                "TM {tm:#04X}, BG1 {bg1_entry:#06X}, BG2 {bg2_entry:#06X}, OBJ{prio}"
            );
        }
    }

    // A high priority BG2 tile is in front of a low priority BG1 tile (BG2.1 > BG1.0).
    #[test]
    fn test_mode4_bg2_high_over_bg1_low() {
        // (BG2 entry, expected)
        for (bg2_entry, expected) in [(0x0400, red()), (0x2400, blue())] {
            let mut ppu = make_ppu_mode4();
            ppu.write(0x212C, 0x03); // TM: BG1 + BG2
            setup_bg1_bg2(&mut ppu, 0x0000, bg2_entry);

            assert_eq!(
                fb_pixel(&render(&ppu), 0, 0),
                expected,
                "BG2 {bg2_entry:#06X}"
            );
        }
    }
}
