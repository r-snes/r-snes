//! BG Mode 3 renderer: one 8bpp layer (BG1) and one 4bpp layer (BG2).
//!
//! Each pixel reads a tilemap entry (tile number, palette, priority, flips),
//! then decodes the tile data from VRAM.
//! BG1 uses all 256 CGRAM colors: the palette bits of its tilemap entries are ignored.
//! BG2 uses 8 palettes of 16 colors (entries 0-127). Color 0 is transparent.
//! There is no BG3 or BG4 in this mode.

use crate::ppu::PPU;
use crate::rendering::renderer::{BgParams, BitDepth, Layer, Priority, Renderer};
use crate::vram::RawVRAM;

impl Renderer {
    /// Renders BG1-BG2 in Mode 3 on framebuffer row `y`.
    pub fn render_scanline_mode3(&mut self, ppu: &PPU, y: usize) {
        // Mode 3 order (front to back), between the sprites (OBJ0-OBJ3 = 3, 6, 9, 12):
        //   OBJ3 > BG1.1 > OBJ2 > BG2.1 > OBJ1 > BG1.0 > OBJ0 > BG2.0 > backdrop
        // BGMODE bit 3 only exists in mode 1: it is ignored here.

        // (bg_index, bpp, z_low, z_high)
        let layers = [
            (0usize, BitDepth::Eight, Priority::new(5), Priority::new(11)),
            (1, BitDepth::Four, Priority::new(2), Priority::new(8)),
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

    /// Returns the color index (0-255) of pixel (`x`, `y`) in the 8bpp tile at `tile_word_base`.
    pub fn decode_8bpp_tile_pixel_from(
        vram: &RawVRAM,
        tile_word_base: usize,
        x: usize,
        y: usize,
    ) -> u8 {
        // Planes 0+1 in words 0-7, 2+3 in words 8-15, 4+5 in words 16-23, 6+7 in words 24-31.
        let bit = 7 - x;
        let mut index = 0;
        for pair in 0..4 {
            let [lo, hi] = vram[(tile_word_base + y + pair * 8) & 0x7FFF].to_le_bytes();
            index |= ((lo >> bit) & 1) << (pair * 2);
            index |= ((hi >> bit) & 1) << (pair * 2 + 1);
        }
        index
    }
}

#[cfg(test)]
mod tests {
    use crate::ppu::PPU;
    use crate::rendering::renderer::Renderer;
    use crate::test_utils::{fb_pixel, set_color, write_sprite};

    // ============================================================
    // Helpers
    // ============================================================

    // BG1: tilemap 0x0400, CHR 0x2000. BG2: tilemap 0x0800, CHR 0x3000.
    const BG1_MAP: usize = 0x0400;
    const BG1_CHR: usize = 0x2000;
    const BG2_MAP: usize = 0x0800;
    const BG2_CHR: usize = 0x3000;

    // Mode 3 PPU at full brightness, with the BG1/BG2 addresses above. No layer enabled.
    fn make_ppu_mode3() -> PPU {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0x0F);
        ppu.write(0x2105, 0x03);
        ppu.write(0x2107, 0x04); // BG1SC: tilemap 0x0400
        ppu.write(0x2108, 0x08); // BG2SC: tilemap 0x0800
        ppu.write(0x210B, 0x32); // BG12NBA: BG1 CHR 0x2000, BG2 CHR 0x3000
        ppu
    }

    // Fills the 32x32 tilemap at `map` with `entry`.
    fn fill_tilemap(ppu: &mut PPU, map: usize, entry: u16) {
        for word in map..map + 0x400 {
            ppu.vram.memory[word] = entry;
        }
    }

    // ============================================================
    // decode_8bpp_tile_pixel_from
    // ============================================================

    // Each plane gives one bit of the index: planes 0-1 in words 0-7, 2-3 in 8-15, 4-5 in 16-23, 6-7 in 24-31.
    #[test]
    fn test_decode_8bpp_planes() {
        for plane in 0..8 {
            let mut vram = Box::new([0; _]);
            vram[(plane / 2) * 8] = if plane % 2 == 0 { 0x0080 } else { 0x8000 };
            assert_eq!(
                Renderer::decode_8bpp_tile_pixel_from(&vram, 0, 0, 0),
                1 << plane,
                "plane {plane}"
            );
            assert_eq!(Renderer::decode_8bpp_tile_pixel_from(&vram, 0, 1, 0), 0);
        }
    }

    // y selects the row in each pair of planes; all planes set gives index 255.
    #[test]
    fn test_decode_8bpp_row_and_full_index() {
        let mut vram = Box::new([0; _]);
        for pair in 0..4 {
            vram[3 + pair * 8] = 0xFFFF;
        }
        assert_eq!(Renderer::decode_8bpp_tile_pixel_from(&vram, 0, 5, 3), 255);
        assert_eq!(Renderer::decode_8bpp_tile_pixel_from(&vram, 0, 5, 2), 0);
    }

    // ============================================================
    // BG1: 8bpp
    // ============================================================

    // BG1 reads all 256 colors: index 129 (planes 0 and 7) is CGRAM 129, whatever the tilemap palette.
    #[test]
    fn test_mode3_bg1_uses_256_colors() {
        let red = Renderer::apply_brightness(0x001F, 15);
        let mut ppu = make_ppu_mode3();
        ppu.write(0x212C, 0x01); // TM: BG1
        fill_tilemap(&mut ppu, BG1_MAP, 0x0C00); // tile 0, palette 3 (ignored)
        for row in 0..8 {
            ppu.vram.memory[BG1_CHR + row] = 0x00FF; // plane 0
            ppu.vram.memory[BG1_CHR + 24 + row] = 0xFF00; // plane 7
        }
        set_color(&mut ppu, 129, 0x001F);

        let mut r = Renderer::new();
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), red);
    }

    // An 8bpp tile is 32 words: tile 1 starts at CHR + 32.
    #[test]
    fn test_mode3_bg1_tile_size() {
        let red = Renderer::apply_brightness(0x001F, 15);
        let mut ppu = make_ppu_mode3();
        ppu.write(0x212C, 0x01); // TM: BG1
        fill_tilemap(&mut ppu, BG1_MAP, 0x0001); // tile 1
        for row in 0..8 {
            ppu.vram.memory[BG1_CHR + 32 + row] = 0x00FF;
        }
        set_color(&mut ppu, 1, 0x001F);

        let mut r = Renderer::new();
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), red);
    }

    // ============================================================
    // BG2: 4bpp
    // ============================================================

    // BG2 is 4bpp: palette 1, color 1 reads CGRAM 17.
    #[test]
    fn test_mode3_bg2_is_4bpp() {
        let blue = Renderer::apply_brightness(0x7C00, 15);
        let mut ppu = make_ppu_mode3();
        ppu.write(0x212C, 0x02); // TM: BG2
        fill_tilemap(&mut ppu, BG2_MAP, 0x0400); // tile 0, palette 1
        for row in 0..8 {
            ppu.vram.memory[BG2_CHR + row] = 0x00FF;
        }
        set_color(&mut ppu, 17, 0x7C00);

        let mut r = Renderer::new();
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), blue);
    }

    // BG3 does not exist in mode 3: enabling it in TM draws nothing.
    #[test]
    fn test_mode3_has_no_bg3() {
        let mut ppu = make_ppu_mode3();
        ppu.write(0x212C, 0x04); // TM: BG3
        ppu.write(0x2109, 0x0C); // BG3SC: tilemap 0x0C00 (all tile 0)
        for row in 0..8 {
            ppu.vram.memory[row] = 0x00FF; // BG3 CHR 0x0000: tile 0 opaque
        }
        set_color(&mut ppu, 1, 0x001F);

        let mut r = Renderer::new();
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), (0, 0, 0));
    }

    // ============================================================
    // Layer order
    // ============================================================

    // Mode 3 order (front to back): OBJ3 > BG1.1 > OBJ2 > BG2.1 > OBJ1 > BG1.0 > OBJ0 > BG2.0.
    // Each BG level is checked against the sprite levels just below and just above it.
    #[test]
    fn test_mode3_bg_vs_sprites() {
        let red = Renderer::apply_brightness(0x001F, 15);
        let blue = Renderer::apply_brightness(0x7C00, 15);
        let green = Renderer::apply_brightness(0x03E0, 15);

        // (TM, BG1 entry, BG2 entry, sprite priority, expected)
        let cases = [
            (0x11, 0x0000, 0x0000, 0, red),   // BG1.0 > OBJ0
            (0x11, 0x0000, 0x0000, 1, green), // OBJ1 > BG1.0
            (0x11, 0x2000, 0x0000, 2, red),   // BG1.1 > OBJ2
            (0x11, 0x2000, 0x0000, 3, green), // OBJ3 > BG1.1
            (0x12, 0x0000, 0x0400, 0, green), // OBJ0 > BG2.0
            (0x12, 0x0000, 0x2400, 1, blue),  // BG2.1 > OBJ1
            (0x12, 0x0000, 0x2400, 2, green), // OBJ2 > BG2.1
        ];
        for (tm, bg1_entry, bg2_entry, prio, expected) in cases {
            let mut ppu = make_ppu_mode3();
            ppu.write(0x212C, tm);

            // BG1 color 1 = red, BG2 palette 1 color 1 = blue.
            fill_tilemap(&mut ppu, BG1_MAP, bg1_entry);
            fill_tilemap(&mut ppu, BG2_MAP, bg2_entry);
            for row in 0..8 {
                ppu.vram.memory[BG1_CHR + row] = 0x00FF;
                ppu.vram.memory[BG2_CHR + row] = 0x00FF;
            }
            set_color(&mut ppu, 1, 0x001F);
            set_color(&mut ppu, 17, 0x7C00);

            // Sprite 0 at (0, 0): CHR base 0, tile 0, color 1 = green. Other sprites hidden.
            ppu.write(0x2101, 0x00);
            for i in 0..128 {
                write_sprite(&mut ppu, i, 0, 0xF0, 0, 0);
            }
            write_sprite(&mut ppu, 0, 0, 0, 0, prio << 4);
            for row in 0..8 {
                ppu.vram.memory[row] = 0x00FF;
            }
            set_color(&mut ppu, 129, 0x03E0);

            let mut r = Renderer::new();
            r.render_scanline(&ppu, 0);
            assert_eq!(
                fb_pixel(&r, 0, 0),
                expected,
                "TM {tm:#04X}, BG1 {bg1_entry:#06X}, BG2 {bg2_entry:#06X}, OBJ{prio}"
            );
        }
    }

    // A high priority BG2 tile is in front of a low priority BG1 tile (BG2.1 > BG1.0).
    #[test]
    fn test_mode3_bg2_high_over_bg1_low() {
        let red = Renderer::apply_brightness(0x001F, 15);
        let blue = Renderer::apply_brightness(0x7C00, 15);

        // (BG2 entry, expected)
        for (bg2_entry, expected) in [(0x0400, red), (0x2400, blue)] {
            let mut ppu = make_ppu_mode3();
            ppu.write(0x212C, 0x03); // TM: BG1 + BG2
            fill_tilemap(&mut ppu, BG1_MAP, 0x0000);
            fill_tilemap(&mut ppu, BG2_MAP, bg2_entry);
            for row in 0..8 {
                ppu.vram.memory[BG1_CHR + row] = 0x00FF;
                ppu.vram.memory[BG2_CHR + row] = 0x00FF;
            }
            set_color(&mut ppu, 1, 0x001F);
            set_color(&mut ppu, 17, 0x7C00);

            let mut r = Renderer::new();
            r.render_scanline(&ppu, 0);
            assert_eq!(fb_pixel(&r, 0, 0), expected, "BG2 {bg2_entry:#06X}");
        }
    }
}
