//! BG Mode 7 renderer: one 8bpp layer (BG1) that can rotate and scale.
//!
//! The map is 128x128 tiles (1024x1024 pixels), in the first 16K words of VRAM:
//! - low byte of word `ty * 128 + tx` = tile number of map tile (tx, ty)
//! - high byte of word `tile * 64 + y * 8 + x` = color index of pixel (x, y) of `tile`

use crate::ppu::PPU;
use crate::registers::M7Outside;
use crate::rendering::renderer::{Layer, LinePixel, Priority, Renderer};
use crate::vram::RawVRAM;
use std::ops::Range;

// OBJ3 > OBJ2 > BG2.1 > OBJ1 > BG1 > OBJ0 > BG2.0 (sprites are 3, 6, 9, 12)
const BG1_Z: Priority = Priority::new(5);
const BG2_LOW: Priority = Priority::new(2);
const BG2_HIGH: Priority = Priority::new(8);

const MAP_PX: i32 = 1024;

// Values shared by BG1 and BG2 on one line.
struct M7Line {
    origin_x: i32,
    origin_y: i32,
    a: i32,
    c: i32,
    flip_x: bool,
    outside: M7Outside,
}

impl M7Line {
    // Color index at screen x `sx`, or None if nothing is drawn there.
    fn color_index(&self, vram: &RawVRAM, sx: i32) -> Option<u8> {
        let map_x = (self.origin_x + self.a * sx) >> 8;
        let map_y = (self.origin_y + self.c * sx) >> 8;

        let in_map = (map_x | map_y) & !(MAP_PX - 1) == 0;
        let tile = match (in_map, self.outside) {
            (true, _) | (false, M7Outside::Wrap) => {
                let tile_x = ((map_x & (MAP_PX - 1)) >> 3) as usize;
                let tile_y = ((map_y & (MAP_PX - 1)) >> 3) as usize;
                (vram[tile_y * 128 + tile_x] & 0x00FF) as usize
            }
            (false, M7Outside::Tile0) => 0,
            (false, M7Outside::Transparent) => return None,
        };

        let fine_x = (map_x & 7) as usize;
        let fine_y = (map_y & 7) as usize;
        Some((vram[tile * 64 + fine_y * 8 + fine_x] >> 8) as u8)
    }
}

impl Renderer {
    /// Renders BG1 (and BG2 with EXTBG) in Mode 7 on pixels `x_range` of framebuffer row `y`.
    pub fn render_scanline_mode7(&mut self, ppu: &PPU, y: usize, x_range: Range<usize>) {
        let regs = &ppu.regs;
        let extbg = regs.extbg();
        let bg1 = (regs.tm & 0x01 != 0, regs.ts & 0x01 != 0);
        let bg2 = (extbg && regs.tm & 0x02 != 0, extbg && regs.ts & 0x02 != 0);
        if bg1 == (false, false) && bg2 == (false, false) {
            return;
        }

        let (a, b, c, d) = regs.m7_matrix();
        let (cx, cy) = regs.m7_center();
        let (h, v) = regs.m7_scroll();
        let (flip_x, flip_y) = regs.m7_flip();

        // The math uses the scanline number, not the framebuffer row.
        let mut line = y as i32 + 1;
        if regs.bg_mosaic(0) {
            line -= ppu.mosaic_voffset() as i32;
        }
        if flip_y {
            line = 255 - line;
        }

        // Map position of screen pixel 0 (8.8 fixed point), low 6 bits of each product removed.
        let hx = Self::m7_clip(h - cx);
        let vy = Self::m7_clip(v - cy);
        let m7 = M7Line {
            origin_x: ((a * hx) & !63) + ((b * vy) & !63) + ((b * line) & !63) + (cx << 8),
            origin_y: ((c * hx) & !63) + ((d * vy) & !63) + ((d * line) & !63) + (cy << 8),
            a,
            c,
            flip_x,
            outside: regs.m7_outside(),
        };

        if bg1 != (false, false) {
            self.render_m7_layer(ppu, &m7, 0, x_range.clone(), bg1);
        }
        if bg2 != (false, false) {
            self.render_m7_layer(ppu, &m7, 1, x_range, bg2);
        }
    }

    // Draws BG `bg` (0 = BG1, 1 = BG2) on the screens in `screens` (main, sub).
    fn render_m7_layer(
        &mut self,
        ppu: &PPU,
        m7: &M7Line,
        bg: usize,
        x_range: Range<usize>,
        screens: (bool, bool),
    ) {
        // Horizontal mosaic uses the bit of the layer.
        let block = if ppu.regs.bg_mosaic(bg) {
            ppu.regs.mosaic_size() as usize
        } else {
            1
        };

        for x in x_range {
            // Mosaic works on screen pixels, before the flip.
            let src_x = (x - x % block) as i32;
            let sx = if m7.flip_x { 255 - src_x } else { src_x };

            let Some(index) = m7.color_index(&ppu.vram.memory, sx) else {
                continue;
            };

            let (color_index, z) = if bg == 0 {
                (index, BG1_Z)
            } else {
                let z = if index & 0x80 != 0 { BG2_HIGH } else { BG2_LOW };
                (index & 0x7F, z)
            };
            if color_index == 0 {
                continue;
            }

            // Direct color is for BG1 only, with no palette bits.
            let color = if bg == 0 && ppu.regs.direct_color() {
                Self::direct_color(color_index, 0)
            } else {
                ppu.cgram.read(color_index)
            };

            let pixel = LinePixel {
                color,
                z,
                layer: Layer::from_bg(bg),
                obj_math: false,
            };
            self.deposit_bg(ppu, bg, x, pixel, screens);
        }
    }

    // (scroll - center) keeps only 10 bits and the sign (bit 13).
    fn m7_clip(value: i32) -> i32 {
        if value & 0x2000 != 0 {
            value | !0x03FF
        } else {
            value & 0x03FF
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ppu::PPU;
    use crate::rendering::renderer::Renderer;
    use crate::test_utils::{fb_pixel, set_color, setup_sprite_at_origin};

    // ============================================================
    // Helpers
    // ============================================================

    fn write_m7(ppu: &mut PPU, addr: u16, value: u16) {
        ppu.write(addr, (value & 0xFF) as u8);
        ppu.write(addr, (value >> 8) as u8);
    }

    // BG1 on main screen, A = D = 1.0 (no rotation, no scale).
    fn make_ppu_mode7() -> PPU {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0x0F);
        ppu.write(0x2105, 0x07);
        ppu.write(0x212C, 0x01); // TM: BG1
        write_m7(&mut ppu, 0x211B, 0x0100); // M7A = 1.0
        write_m7(&mut ppu, 0x211E, 0x0100); // M7D = 1.0
        ppu
    }

    fn set_map_tile(ppu: &mut PPU, tx: usize, ty: usize, tile: u8) {
        let word = &mut ppu.vram.memory[ty * 128 + tx];
        *word = (*word & 0xFF00) | tile as u16;
    }

    fn set_tile_pixel(ppu: &mut PPU, tile: usize, x: usize, y: usize, color_index: u8) {
        let word = &mut ppu.vram.memory[tile * 64 + y * 8 + x];
        *word = (*word & 0x00FF) | ((color_index as u16) << 8);
    }

    fn fill_tile(ppu: &mut PPU, tile: usize, color_index: u8) {
        for y in 0..8 {
            for x in 0..8 {
                set_tile_pixel(ppu, tile, x, y, color_index);
            }
        }
    }

    // All the map is tile 0 (blue), except map tile (1, 0) = tile 1 (red).
    fn setup_two_tiles(ppu: &mut PPU) {
        fill_tile(ppu, 0, 2);
        fill_tile(ppu, 1, 1);
        set_map_tile(ppu, 1, 0, 1);
        set_color(ppu, 1, 0x001F);
        set_color(ppu, 2, 0x7C00);
    }

    fn red() -> (u8, u8, u8) {
        Renderer::apply_brightness(0x001F, 15)
    }

    fn blue() -> (u8, u8, u8) {
        Renderer::apply_brightness(0x7C00, 15)
    }

    fn render(ppu: &PPU, y: usize) -> Renderer {
        let mut r = Renderer::new();
        r.render_scanline(ppu, y);
        r
    }

    // ============================================================
    // Registers
    // ============================================================

    // Bits 13-15 are not used.
    #[test]
    fn test_m7_13bit_sign_extension() {
        let mut ppu = PPU::new();
        write_m7(&mut ppu, 0x210D, 0xFFF8); // M7HOFS = -8
        write_m7(&mut ppu, 0x210E, 0x1FF8); // M7VOFS = -8
        assert_eq!(ppu.regs.m7_scroll(), (-8, -8));

        write_m7(&mut ppu, 0x211F, 0x0FFF); // M7X = 4095
        write_m7(&mut ppu, 0x2120, 0xF000); // M7Y = -4096
        assert_eq!(ppu.regs.m7_center(), (4095, -4096));
    }

    #[test]
    fn test_m7_matrix_is_signed() {
        let mut ppu = PPU::new();
        write_m7(&mut ppu, 0x211B, 0xFF00);
        write_m7(&mut ppu, 0x211C, 0x0080);
        write_m7(&mut ppu, 0x211D, 0x8000);
        write_m7(&mut ppu, 0x211E, 0x7FFF);
        assert_eq!(ppu.regs.m7_matrix(), (-256, 128, -32768, 32767));
    }

    // ============================================================
    // VRAM layout
    // ============================================================

    #[test]
    fn test_mode7_identity_reads_tilemap_and_pixels() {
        let mut ppu = make_ppu_mode7();
        setup_two_tiles(&mut ppu);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), blue());
        assert_eq!(fb_pixel(&r, 7, 0), blue());
        assert_eq!(fb_pixel(&r, 8, 0), red());
        assert_eq!(fb_pixel(&r, 15, 0), red());
        assert_eq!(fb_pixel(&r, 16, 0), blue());
    }

    // Framebuffer row 0 is scanline 1, so we see map row 1.
    #[test]
    fn test_mode7_pixel_addressing() {
        let mut ppu = make_ppu_mode7();
        set_tile_pixel(&mut ppu, 0, 3, 1, 1);
        set_color(&mut ppu, 1, 0x001F);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 3, 0), red());
        assert_eq!(fb_pixel(&r, 2, 0), (0, 0, 0));
        assert_eq!(fb_pixel(&r, 4, 0), (0, 0, 0));

        let r = render(&ppu, 1);
        assert_eq!(fb_pixel(&r, 3, 1), (0, 0, 0));
    }

    #[test]
    fn test_mode7_color_0_transparent() {
        let mut ppu = make_ppu_mode7();
        set_color(&mut ppu, 0, 0x03E0);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), Renderer::apply_brightness(0x03E0, 15));
    }

    // ============================================================
    // Transformation
    // ============================================================

    #[test]
    fn test_mode7_scroll() {
        let mut ppu = make_ppu_mode7();
        setup_two_tiles(&mut ppu);
        write_m7(&mut ppu, 0x210D, 0x0008);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), red());
        assert_eq!(fb_pixel(&r, 8, 0), blue());
    }

    // M7A = 2.0: screen x 4 shows map x 8.
    #[test]
    fn test_mode7_scale() {
        let mut ppu = make_ppu_mode7();
        setup_two_tiles(&mut ppu);
        write_m7(&mut ppu, 0x211B, 0x0200);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 3, 0), blue());
        assert_eq!(fb_pixel(&r, 4, 0), red());
        assert_eq!(fb_pixel(&r, 8, 0), blue());
    }

    // M7B = 1.0: row 6 (scanline 7) starts at map x 7, row 7 at map x 8.
    #[test]
    fn test_mode7_b_shifts_x_per_line() {
        let mut ppu = make_ppu_mode7();
        setup_two_tiles(&mut ppu);
        write_m7(&mut ppu, 0x211C, 0x0100);
        write_m7(&mut ppu, 0x211E, 0x0000);

        assert_eq!(fb_pixel(&render(&ppu, 6), 0, 6), blue());
        assert_eq!(fb_pixel(&render(&ppu, 7), 0, 7), red());
    }

    // M7C = 1.0: screen x 7 (scanline 1) is map row 8.
    #[test]
    fn test_mode7_c_shifts_y_per_pixel() {
        let mut ppu = make_ppu_mode7();
        fill_tile(&mut ppu, 1, 1);
        set_map_tile(&mut ppu, 0, 1, 1);
        set_color(&mut ppu, 1, 0x001F);
        write_m7(&mut ppu, 0x211B, 0x0000);
        write_m7(&mut ppu, 0x211D, 0x0100);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 6, 0), (0, 0, 0));
        assert_eq!(fb_pixel(&r, 7, 0), red());
    }

    // M7A = -1.0: screen x 1 is map x -1, the last tile column.
    #[test]
    fn test_mode7_negative_a_wraps_left() {
        let mut ppu = make_ppu_mode7();
        setup_two_tiles(&mut ppu);
        set_map_tile(&mut ppu, 127, 0, 1);
        write_m7(&mut ppu, 0x211B, 0xFF00);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), blue());
        assert_eq!(fb_pixel(&r, 1, 0), red());
    }

    #[test]
    fn test_mode7_map_wraps() {
        let mut ppu = make_ppu_mode7();
        setup_two_tiles(&mut ppu);
        set_map_tile(&mut ppu, 127, 0, 1);

        write_m7(&mut ppu, 0x210D, 0x0408); // 1032
        assert_eq!(fb_pixel(&render(&ppu, 0), 0, 0), red());

        write_m7(&mut ppu, 0x210D, 0x1FF8); // -8
        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), red()); // map x 1016 = tile 127
        assert_eq!(fb_pixel(&r, 8, 0), blue());
    }

    // M7A = -1.0 around M7X = 8: screen x 0 shows map x 16.
    #[test]
    fn test_mode7_center() {
        let mut ppu = make_ppu_mode7();
        fill_tile(&mut ppu, 2, 1);
        set_map_tile(&mut ppu, 2, 0, 2);
        set_color(&mut ppu, 1, 0x001F);
        write_m7(&mut ppu, 0x211B, 0xFF00);
        write_m7(&mut ppu, 0x211F, 0x0008);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), red());
        assert_eq!(fb_pixel(&r, 1, 0), (0, 0, 0));
    }

    // With M7A = 0.5, scroll 1024 works like scroll 0, not like a 512 pixel move.
    #[test]
    fn test_mode7_scroll_minus_center_is_10_bits() {
        let mut ppu = make_ppu_mode7();
        setup_two_tiles(&mut ppu);
        set_map_tile(&mut ppu, 64, 0, 1); // map x 512
        write_m7(&mut ppu, 0x211B, 0x0080);
        write_m7(&mut ppu, 0x210D, 0x0400);

        assert_eq!(fb_pixel(&render(&ppu, 0), 0, 0), blue());
    }

    // ============================================================
    // Layer order and screens
    // ============================================================

    // OBJ3 > OBJ2 > OBJ1 > BG1 > OBJ0
    #[test]
    fn test_mode7_bg1_vs_sprites() {
        let green = Renderer::apply_brightness(0x03E0, 15);

        for (prio, expected) in [(0u8, red()), (1, green), (3, green)] {
            let mut ppu = make_ppu_mode7();
            ppu.write(0x212C, 0x11); // TM: BG1 + OBJ
            fill_tile(&mut ppu, 0, 1);
            set_color(&mut ppu, 1, 0x001F);
            // Sprite CHR at word 0x4000, after the Mode 7 data.
            setup_sprite_at_origin(&mut ppu, 0x02, prio, 0x03E0);

            assert_eq!(fb_pixel(&render(&ppu, 0), 0, 0), expected, "OBJ{prio}");
        }
    }

    // Without EXTBG, BG2-BG4 in TM draw nothing.
    #[test]
    fn test_mode7_has_no_other_bg() {
        let mut ppu = make_ppu_mode7();
        fill_tile(&mut ppu, 0, 1);
        set_color(&mut ppu, 1, 0x001F);
        ppu.write(0x212C, 0x0E); // TM: BG2-BG4

        assert_eq!(fb_pixel(&render(&ppu, 0), 0, 0), (0, 0, 0));
    }

    // Window 1 (x 10 to 20) hides BG1 on the main screen.
    #[test]
    fn test_mode7_window() {
        let mut ppu = make_ppu_mode7();
        fill_tile(&mut ppu, 0, 1);
        set_color(&mut ppu, 1, 0x001F);
        ppu.write(0x2126, 10); // WH0
        ppu.write(0x2127, 20); // WH1
        ppu.write(0x2123, 0x02); // W12SEL: BG1 window 1
        ppu.write(0x212E, 0x01); // TMW: BG1

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 9, 0), red());
        assert_eq!(fb_pixel(&r, 15, 0), (0, 0, 0));
        assert_eq!(fb_pixel(&r, 21, 0), red());
    }

    // ============================================================
    // Direct color
    // ============================================================

    // Index 0x07 gives R = 28, with no palette bits.
    #[test]
    fn test_mode7_direct_color() {
        let mut ppu = make_ppu_mode7();
        fill_tile(&mut ppu, 0, 0x07);
        set_color(&mut ppu, 7, 0x7C00);
        ppu.write(0x2130, 0x01); // CGWSEL: direct color

        assert_eq!(
            fb_pixel(&render(&ppu, 0), 0, 0),
            Renderer::apply_brightness(28, 15)
        );
    }

    // ============================================================
    // M7SEL: screen flip
    // ============================================================

    // Screen x 247 shows map x 8, screen x 248 shows map x 7.
    #[test]
    fn test_mode7_flip_x() {
        let mut ppu = make_ppu_mode7();
        setup_two_tiles(&mut ppu);
        ppu.write(0x211A, 0x01);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 247, 0), red());
        assert_eq!(fb_pixel(&r, 248, 0), blue());
        assert_eq!(fb_pixel(&r, 0, 0), blue());
    }

    // Row 0 (scanline 1) is computed like line 254.
    #[test]
    fn test_mode7_flip_y() {
        let mut ppu = make_ppu_mode7();
        setup_two_tiles(&mut ppu);
        set_map_tile(&mut ppu, 0, 31, 1);

        assert_eq!(fb_pixel(&render(&ppu, 0), 0, 0), blue());

        ppu.write(0x211A, 0x02);
        assert_eq!(fb_pixel(&render(&ppu, 0), 0, 0), red());
    }

    // ============================================================
    // M7SEL: outside the map
    // ============================================================

    // Bit 6 alone does nothing.
    #[test]
    fn test_mode7_outside_wraps() {
        for m7sel in [0x00, 0x40] {
            let mut ppu = make_ppu_mode7();
            setup_two_tiles(&mut ppu);
            set_map_tile(&mut ppu, 127, 0, 1);
            write_m7(&mut ppu, 0x210D, 0x1FF8); // M7HOFS = -8
            ppu.write(0x211A, m7sel);

            assert_eq!(
                fb_pixel(&render(&ppu, 0), 0, 0),
                red(),
                "M7SEL {m7sel:#04X}"
            );
        }
    }

    #[test]
    fn test_mode7_outside_transparent() {
        let mut ppu = make_ppu_mode7();
        setup_two_tiles(&mut ppu);
        set_map_tile(&mut ppu, 127, 0, 1);
        write_m7(&mut ppu, 0x210D, 0x1FF8); // M7HOFS = -8
        ppu.write(0x211A, 0x80);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), (0, 0, 0));
        assert_eq!(fb_pixel(&r, 7, 0), (0, 0, 0));
        assert_eq!(fb_pixel(&r, 8, 0), blue());
    }

    #[test]
    fn test_mode7_outside_transparent_vertical() {
        let mut ppu = make_ppu_mode7();
        setup_two_tiles(&mut ppu);
        write_m7(&mut ppu, 0x210E, 0x1FF0); // M7VOFS = -16
        ppu.write(0x211A, 0x80);

        assert_eq!(fb_pixel(&render(&ppu, 0), 0, 0), (0, 0, 0));
    }

    // The pixel inside tile 0 comes from the map position: screen x 0 (map x -8) is pixel (0, 1).
    #[test]
    fn test_mode7_outside_tile0() {
        let mut ppu = make_ppu_mode7();
        setup_two_tiles(&mut ppu);
        set_map_tile(&mut ppu, 127, 0, 1); // red if the map repeats
        set_tile_pixel(&mut ppu, 0, 0, 1, 1);
        write_m7(&mut ppu, 0x210D, 0x1FF8); // M7HOFS = -8
        ppu.write(0x211A, 0xC0);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), red());
        assert_eq!(fb_pixel(&r, 1, 0), blue());
        assert_eq!(fb_pixel(&r, 8, 0), red());
        assert_eq!(fb_pixel(&r, 16, 0), red());
    }

    // ============================================================
    // Mosaic
    // ============================================================

    // Only column `x` of tile 0 is red, on every row.
    fn setup_mosaic_column(ppu: &mut PPU, x: usize) {
        for y in 0..8 {
            set_tile_pixel(ppu, 0, x, y, 1);
        }
        set_color(ppu, 1, 0x001F);
    }

    // Size 4: screen x 0-3 copy screen x 0, x 4-7 copy x 4.
    #[test]
    fn test_mode7_mosaic_horizontal() {
        let mut ppu = make_ppu_mode7();
        setup_mosaic_column(&mut ppu, 0);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), red());
        assert_eq!(fb_pixel(&r, 1, 0), (0, 0, 0));

        ppu.write(0x2106, 0x31); // size 4, BG1
        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 3, 0), red());
        assert_eq!(fb_pixel(&r, 4, 0), (0, 0, 0));
        assert_eq!(fb_pixel(&r, 9, 0), red());
    }

    // Mosaic on BG2 only does not change BG1.
    #[test]
    fn test_mode7_mosaic_needs_bg1_bit() {
        let mut ppu = make_ppu_mode7();
        setup_mosaic_column(&mut ppu, 0);
        ppu.write(0x2106, 0x32); // size 4, BG2

        assert_eq!(fb_pixel(&render(&ppu, 0), 1, 0), (0, 0, 0));
    }

    // Only map row 1 is red. Counter 3 with size 4: scanline 2 is 1 line into the block,
    // so the math uses line 1.
    #[test]
    fn test_mode7_mosaic_vertical() {
        let mut ppu = make_ppu_mode7();
        for x in 0..8 {
            set_tile_pixel(&mut ppu, 0, x, 1, 1);
        }
        set_color(&mut ppu, 1, 0x001F);

        assert_eq!(fb_pixel(&render(&ppu, 1), 0, 1), (0, 0, 0));

        ppu.write(0x2106, 0x31);
        ppu.mosaic_vcounter = 3;
        assert_eq!(fb_pixel(&render(&ppu, 1), 0, 1), red());
    }

    // With the flip, blocks still start at screen x 0: x 0-3 copy x 0 (map x 255).
    #[test]
    fn test_mode7_mosaic_with_flip() {
        let mut ppu = make_ppu_mode7();
        setup_mosaic_column(&mut ppu, 7);
        ppu.write(0x211A, 0x01);

        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0, 0), red());
        assert_eq!(fb_pixel(&r, 1, 0), (0, 0, 0));

        ppu.write(0x2106, 0x31);
        let r = render(&ppu, 0);
        assert_eq!(fb_pixel(&r, 3, 0), red());
        assert_eq!(fb_pixel(&r, 4, 0), (0, 0, 0));
    }

    // ============================================================
    // EXTBG
    // ============================================================

    // Mode 7 with EXTBG (SETINI bit 6) and BG2 on the main screen.
    fn make_ppu_extbg() -> PPU {
        let mut ppu = make_ppu_mode7();
        ppu.write(0x2133, 0x40);
        ppu.write(0x212C, 0x02); // TM: BG2
        ppu
    }

    // BG2 uses bits 0-6 for the color: index 0x81 reads CGRAM 1.
    #[test]
    fn test_extbg_bg2_color() {
        let mut ppu = make_ppu_extbg();
        fill_tile(&mut ppu, 0, 0x81);
        set_color(&mut ppu, 1, 0x001F);
        set_color(&mut ppu, 129, 0x7C00);

        assert_eq!(fb_pixel(&render(&ppu, 0), 0, 0), red());
    }

    // Index 0x80 is transparent for BG2 (color 0), but CGRAM 128 for BG1.
    #[test]
    fn test_extbg_index_0x80() {
        let mut ppu = make_ppu_extbg();
        fill_tile(&mut ppu, 0, 0x80);
        set_color(&mut ppu, 128, 0x001F);

        assert_eq!(fb_pixel(&render(&ppu, 0), 0, 0), (0, 0, 0));

        ppu.write(0x212C, 0x01); // TM: BG1
        assert_eq!(fb_pixel(&render(&ppu, 0), 0, 0), red());
    }

    // OBJ3 > OBJ2 > BG2.1 > OBJ1 > BG1 > OBJ0 > BG2.0
    #[test]
    fn test_extbg_bg2_vs_sprites() {
        let green = Renderer::apply_brightness(0x03E0, 15);

        // (BG2 index, sprite priority, expected)
        let cases = [
            (0x01, 0u8, green), // OBJ0 > BG2.0
            (0x81, 1, red()),   // BG2.1 > OBJ1
            (0x81, 2, green),   // OBJ2 > BG2.1
        ];
        for (index, prio, expected) in cases {
            let mut ppu = make_ppu_extbg();
            ppu.write(0x212C, 0x12); // TM: BG2 + OBJ
            fill_tile(&mut ppu, 0, index);
            set_color(&mut ppu, 1, 0x001F);
            // Sprite CHR at word 0x4000, after the Mode 7 data.
            setup_sprite_at_origin(&mut ppu, 0x02, prio, 0x03E0);

            assert_eq!(
                fb_pixel(&render(&ppu, 0), 0, 0),
                expected,
                "index {index:#04X}, OBJ{prio}"
            );
        }
    }

    // BG2.1 > BG1 > BG2.0. Direct color is on, and only BG1 uses it,
    // so BG1 (R = 4) and BG2 (CGRAM 1) have different colors.
    #[test]
    fn test_extbg_bg2_vs_bg1() {
        let bg1_direct = Renderer::apply_brightness(Renderer::direct_color(0x01, 0), 15);

        // (index, expected)
        for (index, expected) in [(0x01, bg1_direct), (0x81, red())] {
            let mut ppu = make_ppu_extbg();
            ppu.write(0x212C, 0x03); // TM: BG1 + BG2
            ppu.write(0x2130, 0x01); // CGWSEL: direct color
            fill_tile(&mut ppu, 0, index);
            set_color(&mut ppu, 1, 0x001F);

            assert_eq!(
                fb_pixel(&render(&ppu, 0), 0, 0),
                expected,
                "index {index:#04X}"
            );
        }
    }

    // BG2 horizontal mosaic uses the BG2 bit.
    #[test]
    fn test_extbg_mosaic_horizontal_uses_bg2_bit() {
        let mut ppu = make_ppu_extbg();
        setup_mosaic_column(&mut ppu, 0);

        ppu.write(0x2106, 0x32); // size 4, BG2
        assert_eq!(fb_pixel(&render(&ppu, 0), 1, 0), red());

        ppu.write(0x2106, 0x31); // size 4, BG1
        assert_eq!(fb_pixel(&render(&ppu, 0), 1, 0), (0, 0, 0));
    }

    // BG2 vertical mosaic uses the BG1 bit. Only map row 1 is red.
    #[test]
    fn test_extbg_mosaic_vertical_uses_bg1_bit() {
        let mut ppu = make_ppu_extbg();
        for x in 0..8 {
            set_tile_pixel(&mut ppu, 0, x, 1, 1);
        }
        set_color(&mut ppu, 1, 0x001F);
        ppu.mosaic_vcounter = 3;

        ppu.write(0x2106, 0x31); // size 4, BG1
        assert_eq!(fb_pixel(&render(&ppu, 1), 0, 1), red());

        ppu.write(0x2106, 0x32); // size 4, BG2
        assert_eq!(fb_pixel(&render(&ppu, 1), 0, 1), (0, 0, 0));
    }
}
