//! BG Mode 2 renderer: two 4bpp layers (BG1, BG2).
//!
//! Each pixel reads a tilemap entry (tile number, palette, priority, flips),
//! then decodes the tile data from VRAM.
//! BG1 and BG2 share 8 palettes of 16 colors (CGRAM entries 0-127). Color 0 is transparent.
//! BG3 is not drawn: its tilemap changes the scroll of each column of BG1 and BG2

use crate::ppu::PPU;
use crate::rendering::renderer::{BgParams, BitDepth, Layer, Priority, Renderer};
use std::ops::Range;

impl Renderer {
    /// Renders BG1-BG2 in Mode 2 on pixels `x_range` of framebuffer row `y`.
    pub fn render_scanline_mode2(&mut self, ppu: &PPU, y: usize, x_range: Range<usize>) {
        // Mode 2 order (front to back), between the sprites (OBJ0-OBJ3 = 3, 6, 9, 12):
        //   OBJ3 > BG1.1 > OBJ2 > BG2.1 > OBJ1 > BG1.0 > OBJ0 > BG2.0 > backdrop

        // (bg_index, z_low, z_high)
        let layers = [
            (0usize, Priority::new(5), Priority::new(11)),
            (1, Priority::new(2), Priority::new(8)),
        ];

        for (bg, z_low, z_high) in layers {
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
                x_range.clone(),
                &BgParams {
                    tilemap_base: ppu.regs.bg_tilemap_addr(bg),
                    tiledata_base: ppu.regs.bg_tiledata_addr(bg),
                    scroll_x,
                    scroll_y,
                    bpp: BitDepth::Four,
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

    // Mode 2 PPU at full brightness, with the BG1/BG2 addresses above. No layer enabled.
    fn make_ppu_mode2() -> PPU {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0x0F);
        ppu.write(0x2105, 0x02);
        ppu.write(0x2107, 0x04); // BG1SC: tilemap 0x0400
        ppu.write(0x2108, 0x08); // BG2SC: tilemap 0x0800
        ppu.write(0x210B, 0x32); // BG12NBA: BG1 CHR 0x2000, BG2 CHR 0x3000
        ppu
    }

    // BG1 and BG2 with tile 0 full of color 1. BG1 palette 0 color 1 = red,
    // BG2 palette 1 color 1 = blue.
    fn setup_bg1_bg2(ppu: &mut PPU, bg1_entry: u16, bg2_entry: u16) {
        fill_tilemap(ppu, BG1_MAP, bg1_entry);
        fill_tilemap(ppu, BG2_MAP, bg2_entry);
        for row in 0..8 {
            ppu.vram.memory[BG1_CHR + row] = 0x00FF;
            ppu.vram.memory[BG2_CHR + row] = 0x00FF;
        }
        set_color(ppu, 1, 0x001F);
        set_color(ppu, 17, 0x7C00);
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
    // BG1 and BG2: 4bpp
    // ============================================================

    // Palette 2, color 3 reads CGRAM 2 * 16 + 3 = 35.
    #[test]
    fn test_mode2_bg1_is_4bpp() {
        let mut ppu = make_ppu_mode2();
        ppu.write(0x212C, 0x01); // TM: BG1
        fill_tilemap(&mut ppu, BG1_MAP, 0x0800); // tile 0, palette 2
        for row in 0..8 {
            ppu.vram.memory[BG1_CHR + row] = 0xFFFF; // planes 0 and 1: color 3
        }
        set_color(&mut ppu, 35, 0x001F);

        assert_eq!(fb_pixel(&render(&ppu), 0, 0), red());
    }

    // A 4bpp tile is 16 words: tile 1 starts at CHR + 16.
    #[test]
    fn test_mode2_tile_size() {
        let mut ppu = make_ppu_mode2();
        ppu.write(0x212C, 0x01); // TM: BG1
        fill_tilemap(&mut ppu, BG1_MAP, 0x0001); // tile 1
        for row in 0..8 {
            ppu.vram.memory[BG1_CHR + 16 + row] = 0x00FF;
        }
        set_color(&mut ppu, 1, 0x001F);

        assert_eq!(fb_pixel(&render(&ppu), 0, 0), red());
    }

    // BG2 is 4bpp too: palette 1, color 1 reads CGRAM 17.
    #[test]
    fn test_mode2_bg2_is_4bpp() {
        let mut ppu = make_ppu_mode2();
        ppu.write(0x212C, 0x02); // TM: BG2
        setup_bg1_bg2(&mut ppu, 0x0000, 0x0400);

        assert_eq!(fb_pixel(&render(&ppu), 0, 0), blue());
    }

    // BG3 is not drawn: enabling it in TM draws nothing.
    #[test]
    fn test_mode2_has_no_bg3() {
        let mut ppu = make_ppu_mode2();
        ppu.write(0x212C, 0x04); // TM: BG3
        ppu.write(0x2109, 0x0C); // BG3SC: tilemap 0x0C00 (all tile 0)
        for row in 0..8 {
            ppu.vram.memory[row] = 0x00FF; // BG3 CHR 0x0000: tile 0 opaque
        }
        set_color(&mut ppu, 1, 0x001F);

        assert_eq!(fb_pixel(&render(&ppu), 0, 0), (0, 0, 0));
    }

    // Direct color only works on 8bpp layers: BG1 still reads CGRAM.
    #[test]
    fn test_mode2_no_direct_color() {
        let mut ppu = make_ppu_mode2();
        ppu.write(0x212C, 0x01); // TM: BG1
        ppu.write(0x2130, 0x01); // CGWSEL: direct color
        setup_bg1_bg2(&mut ppu, 0x0000, 0x0000);

        assert_eq!(fb_pixel(&render(&ppu), 0, 0), red());
    }

    // ============================================================
    // Layer order
    // ============================================================

    // OBJ3 > BG1.1 > OBJ2 > BG2.1 > OBJ1 > BG1.0 > OBJ0 > BG2.0.
    // Each BG level is checked against the sprite levels just below and just above it.
    #[test]
    fn test_mode2_bg_vs_sprites() {
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
            let mut ppu = make_ppu_mode2();
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
    fn test_mode2_bg2_high_over_bg1_low() {
        // (BG2 entry, expected)
        for (bg2_entry, expected) in [(0x0400, red()), (0x2400, blue())] {
            let mut ppu = make_ppu_mode2();
            ppu.write(0x212C, 0x03); // TM: BG1 + BG2
            setup_bg1_bg2(&mut ppu, 0x0000, bg2_entry);

            assert_eq!(
                fb_pixel(&render(&ppu), 0, 0),
                expected,
                "BG2 {bg2_entry:#06X}"
            );
        }
    }

    // BGMODE bit 3 (BG3 priority) only exists in mode 1: it changes nothing here.
    #[test]
    fn test_mode2_ignores_bgmode_bit3() {
        let mut ppu = make_ppu_mode2();
        ppu.write(0x2105, 0x0A); // mode 2 + bit 3
        ppu.write(0x212C, 0x03); // TM: BG1 + BG2
        setup_bg1_bg2(&mut ppu, 0x0000, 0x0400);

        assert_eq!(fb_pixel(&render(&ppu), 0, 0), red());
    }

    // ============================================================
    // Offset-per-tile
    // ============================================================

    const BG3_MAP: usize = 0x0C00;

    // BG1 on main: tile 0 (red) everywhere, tile 1 (blue) on the whole column 4 (BG x 32-39).
    // BG3 tilemap at 0x0C00, empty.
    fn make_ppu_opt() -> PPU {
        let mut ppu = make_ppu_mode2();
        ppu.write(0x212C, 0x01); // TM: BG1
        ppu.write(0x2109, 0x0C); // BG3SC: tilemap 0x0C00
        fill_tilemap(&mut ppu, BG1_MAP, 0x0000);
        for row in 0..32 {
            ppu.vram.memory[BG1_MAP + row * 32 + 4] = 0x0001;
        }
        for row in 0..8 {
            ppu.vram.memory[BG1_CHR + row] = 0x00FF; // tile 0: color 1
            ppu.vram.memory[BG1_CHR + 16 + row] = 0xFF00; // tile 1: color 2
        }
        set_color(&mut ppu, 1, 0x001F);
        set_color(&mut ppu, 2, 0x7C00);
        ppu
    }

    fn set_bg3_entry(ppu: &mut PPU, col: usize, row: usize, entry: u16) {
        ppu.vram.memory[BG3_MAP + row * 32 + col] = entry;
    }

    // Writes a BG scroll register twice: low byte, then high byte.
    fn write_scroll(ppu: &mut PPU, addr: u16, value: u16) {
        ppu.write(addr, (value & 0xFF) as u8);
        ppu.write(addr, (value >> 8) as u8);
    }

    // Screen column n reads BG3 column n - 1 on row 0 (horizontal values).
    #[test]
    fn test_opt_horizontal() {
        let mut ppu = make_ppu_opt();
        set_bg3_entry(&mut ppu, 0, 0, 0x2000 | 24);

        let r = render(&ppu);
        assert_eq!(fb_pixel(&r, 0, 0), red());
        assert_eq!(fb_pixel(&r, 8, 0), blue());
        assert_eq!(fb_pixel(&r, 16, 0), red());
    }

    // An entry on every BG3 column still leaves column 0 of the screen alone.
    #[test]
    fn test_opt_first_column_never_changes() {
        let mut ppu = make_ppu_opt();
        for col in 0..32 {
            set_bg3_entry(&mut ppu, col, 0, 0x2000 | 24);
        }

        let r = render(&ppu);
        assert_eq!(fb_pixel(&r, 0, 0), red());
        assert_eq!(fb_pixel(&r, 7, 0), red());
        assert_eq!(fb_pixel(&r, 8, 0), blue());
    }

    // Bit 13 is for BG1, bit 14 for BG2: an entry for BG2 does not move BG1.
    #[test]
    fn test_opt_enable_bits() {
        let mut ppu = make_ppu_opt();
        set_bg3_entry(&mut ppu, 0, 0, 0x4000 | 24);
        assert_eq!(fb_pixel(&render(&ppu), 8, 0), red());

        // Same layout on BG2: the entry now moves it.
        let mut ppu = make_ppu_opt();
        ppu.write(0x212C, 0x02); // TM: BG2
        ppu.write(0x2108, 0x04); // BG2SC: same tilemap as BG1
        ppu.write(0x210B, 0x22); // BG2 CHR 0x2000, same as BG1
        set_bg3_entry(&mut ppu, 0, 0, 0x4000 | 24);
        assert_eq!(fb_pixel(&render(&ppu), 8, 0), blue());
    }

    // HOFS 3 is kept as fine scroll: screen x 5 is already in column 1.
    #[test]
    fn test_opt_keeps_fine_scroll() {
        let mut ppu = make_ppu_opt();
        write_scroll(&mut ppu, 0x210D, 3);
        set_bg3_entry(&mut ppu, 0, 0, 0x2000 | 31); // 31 & !7 = 24

        let r = render(&ppu);
        assert_eq!(fb_pixel(&r, 4, 0), red()); // column 0: BG x 7
        assert_eq!(fb_pixel(&r, 5, 0), blue()); // column 1: 5 + (24 | 3) = BG x 32
    }

    // Row 1 holds the vertical values, they replace BG1VOFS.
    #[test]
    fn test_opt_vertical() {
        let mut ppu = make_ppu_opt();
        ppu.vram.memory[BG1_MAP + 32 + 1] = 0x0001; // BG1 tile (1, 1) is blue
        write_scroll(&mut ppu, 0x210E, 100); // replaced, not added
        set_bg3_entry(&mut ppu, 0, 1, 0x2000 | 8);

        let r = render(&ppu);
        assert_eq!(fb_pixel(&r, 0, 0), red()); // column 0: normal VOFS
        assert_eq!(fb_pixel(&r, 8, 0), blue());
    }

    // BG3HOFS 8 starts at column 1, BG3VOFS 21 reads row 2 (low 3 bits ignored).
    #[test]
    fn test_opt_bg3_scroll_selects_entries() {
        let mut ppu = make_ppu_opt();
        write_scroll(&mut ppu, 0x2111, 8);
        write_scroll(&mut ppu, 0x2112, 21);
        set_bg3_entry(&mut ppu, 1, 2, 0x2000 | 24);

        assert_eq!(fb_pixel(&render(&ppu), 8, 0), blue());
    }

    // With 16x16 BG3 tiles, the same entry is used for both directions and 2 columns.
    #[test]
    fn test_opt_bg3_16x16() {
        let mut ppu = make_ppu_opt();
        for col in 0..32 {
            ppu.vram.memory[BG1_MAP + 32 + col] = 0x0001; // BG1 tile row 1 is blue
        }
        set_bg3_entry(&mut ppu, 0, 0, 0x2000 | 8);

        // 8x8: only the horizontal value, row 0 stays on BG row 1 (tile row 0).
        assert_eq!(fb_pixel(&render(&ppu), 8, 0), red());

        // 16x16: VOFS 8 too, so BG row 9 (tile row 1). Column 2 reads the same tile.
        ppu.write(0x2105, 0x42); // mode 2, BG3 16x16
        let r = render(&ppu);
        assert_eq!(fb_pixel(&r, 8, 0), blue());
        assert_eq!(fb_pixel(&r, 16, 0), blue());
    }

    // BG3 two maps tall: the vertical row after row 31 is row 0 of the map below.
    #[test]
    fn test_opt_bg3_tall_map() {
        let mut ppu = make_ppu_opt();
        for col in 0..32 {
            ppu.vram.memory[BG1_MAP + 32 + col] = 0x0001; // BG1 tile row 1 is blue
        }
        write_scroll(&mut ppu, 0x2112, 248); // horizontal row 31, vertical row 32
        ppu.vram.memory[0x1000] = 0x2000 | 8; // map below (0x0C00 + 0x400), row 0

        // 32x32: row 32 goes back to row 0 of the same map, which is empty.
        assert_eq!(fb_pixel(&render(&ppu), 8, 0), red());

        // 32x64: row 32 is in the map below, VOFS 8.
        ppu.write(0x2109, 0x0E); // BG3SC: tilemap 0x0C00, 32x64
        assert_eq!(fb_pixel(&render(&ppu), 8, 0), blue());
    }

    // Offset-per-tile only exists in modes 2, 4 and 6: mode 1 ignores BG3 here.
    #[test]
    fn test_opt_not_in_mode1() {
        let mut ppu = make_ppu_opt();
        set_bg3_entry(&mut ppu, 0, 0, 0x2000 | 24);
        ppu.write(0x2105, 0x01);
        ppu.write(0x212C, 0x01); // TM: BG1 only, BG3 is not drawn

        assert_eq!(fb_pixel(&render(&ppu), 8, 0), red());
    }
}
