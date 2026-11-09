//! BG Mode 7 renderer: one 8bpp layer (BG1) drawn through an affine transformation.
//!
//! The map is 128x128 tiles (1024x1024 pixels), interleaved in the first 16K words of VRAM:
//! the low byte of word `ty * 128 + tx` is the tile number of map tile (tx, ty),
//! the high byte of word `tile * 64 + y * 8 + x` is the color index of pixel (x, y) of `tile`.
//! There are no palette bits, no flips per tile and no priority bit. Color 0 is transparent.
//!
//! Each screen pixel is mapped to a map pixel with the M7A-M7D matrix (8.8 fixed point),
//! around the M7X/M7Y center, after the M7HOFS/M7VOFS scroll.

use crate::constants::SCREEN_WIDTH;
use crate::ppu::PPU;
use crate::rendering::renderer::{Layer, LinePixel, Priority, Renderer};

// OBJ3 > OBJ2 > OBJ1 > BG1 > OBJ0 (sprites are 3, 6, 9, 12)
const BG1_Z: Priority = Priority::new(5);

const MAP_PX: i32 = 1024;

impl Renderer {
    /// Renders BG1 in Mode 7 on framebuffer row `y`.
    pub fn render_scanline_mode7(&mut self, ppu: &PPU, y: usize) {
        let to_main = ppu.regs.tm & 0x01 != 0;
        let to_sub = ppu.regs.ts & 0x01 != 0;
        if !to_main && !to_sub {
            return;
        }

        let (a, b, c, d) = ppu.regs.m7_matrix();
        let (cx, cy) = ppu.regs.m7_center();
        let (h, v) = ppu.regs.m7_scroll();

        // The math uses the scanline number, not the framebuffer row.
        let line = y as i32 + 1;

        // Map position of screen pixel 0 (8.8 fixed point), low 6 bits of each product removed.
        let hx = Self::m7_clip(h - cx);
        let vy = Self::m7_clip(v - cy);
        let origin_x = ((a * hx) & !63) + ((b * vy) & !63) + ((b * line) & !63) + (cx << 8);
        let origin_y = ((c * hx) & !63) + ((d * vy) & !63) + ((d * line) & !63) + (cy << 8);

        let vram = &ppu.vram.memory;
        for x in 0..SCREEN_WIDTH {
            let map_x = ((origin_x + a * x as i32) >> 8) & (MAP_PX - 1);
            let map_y = ((origin_y + c * x as i32) >> 8) & (MAP_PX - 1);

            let tile_x = (map_x >> 3) as usize;
            let tile_y = (map_y >> 3) as usize;
            let tile = (vram[tile_y * 128 + tile_x] & 0x00FF) as usize;

            let fine_x = (map_x & 7) as usize;
            let fine_y = (map_y & 7) as usize;
            let color_index = (vram[tile * 64 + fine_y * 8 + fine_x] >> 8) as u8;

            if color_index == 0 {
                continue;
            }

            // No palette bits in Mode 7.
            let color = if ppu.regs.direct_color() {
                Self::direct_color(color_index, 0)
            } else {
                ppu.cgram.read(color_index)
            };

            let pixel = LinePixel {
                color,
                z: BG1_Z,
                layer: Layer::Bg1,
                obj_math: false,
            };
            self.deposit_bg(ppu, 0, x, pixel, to_main, to_sub);
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
    use crate::test_utils::{fb_pixel, set_color, write_sprite};

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
            ppu.write(0x2101, 0x02);
            for i in 0..128 {
                write_sprite(&mut ppu, i, 0, 0xF0, 0, 0);
            }
            write_sprite(&mut ppu, 0, 0, 0, 0, prio << 4);
            for row in 0..8 {
                ppu.vram.memory[0x4000 + row] = 0x00FF;
            }
            set_color(&mut ppu, 129, 0x03E0);

            assert_eq!(fb_pixel(&render(&ppu, 0), 0, 0), expected, "OBJ{prio}");
        }
    }

    #[test]
    fn test_mode7_has_no_other_bg() {
        let mut ppu = make_ppu_mode7();
        fill_tile(&mut ppu, 0, 1);
        set_color(&mut ppu, 1, 0x001F);
        ppu.write(0x212C, 0x0E); // TM: BG2-BG4

        assert_eq!(fb_pixel(&render(&ppu, 0), 0, 0), (0, 0, 0));
    }

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
}
