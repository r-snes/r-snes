use crate::constants::*;
use crate::ppu::PPU;
use crate::rendering::renderer::{Renderer, Z_BG1_HIGH, Z_BG1_LOW};
use crate::vram::RawVRAM;

impl Renderer {
    pub fn render_scanline_mode0(&mut self, ppu: &PPU, y: usize) {
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
            let priority = (entry & 0x2000) != 0; // bit 13
            let flip_x = (entry & 0x4000) != 0; // bit 14
            let flip_y = (entry & 0x8000) != 0; // bit 15

            // Apply flip
            let fx = if flip_x { 7 - fine_x } else { fine_x };
            let fy = if flip_y { 7 - fine_y } else { fine_y };

            // ============================================================
            // Decode 2bpp pixel from CHR data
            // ============================================================
            let tile_word_base = tiledata_base as usize + tile_index as usize * 8;
            let color_index =
                Self::decode_2bpp_tile_pixel_from(&ppu.vram.memory, tile_word_base, fx, fy);

            // Transparent pixel -> do nothing
            if color_index == 0 {
                continue;
            }

            let palette_entry = ((palette_num as u8) << 2) + color_index;
            let color = ppu.cgram.read(palette_entry);

            let (r, g, b) = Self::apply_brightness(color, self.current_brightness as u16);
            let z = if priority { Z_BG1_HIGH } else { Z_BG1_LOW };
            self.set_pixel_z(x, y, r, g, b, z);
        }
    }

    pub fn decode_2bpp_tile_pixel_from(
        vram: &RawVRAM,
        tile_word_base: usize,
        x: usize,
        y: usize,
    ) -> u8 {
        // Planes 0+1: words 0-7
        let w = vram[tile_word_base + y];
        let p0 = (w & 0xFF) as u8;
        let p1 = (w >> 8) as u8;

        let bit = 7 - x;
        ((p0 >> bit) & 1) | (((p1 >> bit) & 1) << 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{SCREEN_WIDTH, VRAM_SIZE};
    use crate::ppu::PPU;

    // ============================================================
    // Helpers
    // ============================================================

    // Minimal PPU in mode 0: tilemap at word 0x0000, BG1 CHR at word 0x1000, no scroll, full brightness.
    // CHR_BASE = 0x1000 to avoid overlap with the tilemap (32x32 = 1024 words starting at 0x0000).
    const CHR_BASE: usize = 0x1000;

    fn make_ppu_mode0() -> PPU {
        let mut ppu = PPU::new();
        ppu.write(0x2105, 0x00); // BG mode 0
        ppu.write(0x2107, 0x00); // BG1SC: tilemap at word 0x0000
        ppu.write(0x210B, 0x01); // BG12NBA: BG1 CHR at word 0x1000 (nibble 1 -> 1 * 0x1000)
        ppu.write(0x2100, 0x0F); // INIDISP: full brightness, no force blank
        ppu.write(0x212C, 0x01); // TM: BG1 enabled on main screen
        ppu
    }

    fn make_renderer() -> Renderer {
        let mut r = Renderer::new();
        r.current_brightness = 15;
        r
    }

    fn pixel(renderer: &Renderer, x: usize, y: usize) -> (u8, u8, u8) {
        let idx = (y * SCREEN_WIDTH + x) * 3;
        (
            renderer.framebuffer[idx],
            renderer.framebuffer[idx + 1],
            renderer.framebuffer[idx + 2],
        )
    }

    fn set_cgram_white(ppu: &mut PPU, entry: u8) {
        ppu.write(0x2121, entry);
        ppu.write(0x2122, 0xFF);
        ppu.write(0x2122, 0x7F);
    }

    // ============================================================
    // decode_2bpp_tile_pixel_from
    // ============================================================

    // plane 0 only -> index 1, plane 1 only -> index 2, both -> index 3, neither -> index 0.
    #[test]
    fn test_decode_2bpp_color_indices() {
        let mut vram = Box::new([0u16; VRAM_SIZE / 2]);

        // All zero -> index 0
        for x in 0..8 {
            assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 0, x, 0), 0);
        }

        // Plane 0 only -> index 1
        vram[0] = 0x00FF;
        for x in 0..8 {
            assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 0, x, 0), 1);
        }

        // Plane 1 only -> index 2
        vram[0] = 0xFF00;
        for x in 0..8 {
            assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 0, x, 0), 2);
        }

        // Both planes -> index 3
        vram[0] = 0xFFFF;
        for x in 0..8 {
            assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 0, x, 0), 3);
        }
    }

    // bit 7 = x=0 (leftmost), bit 0 = x=7 (rightmost).
    #[test]
    fn test_decode_2bpp_bit_ordering() {
        let mut vram = Box::new([0u16; VRAM_SIZE / 2]);

        // Bit 7 set -> only x=0 is color 1
        vram[0] = 0x0080;
        assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 0, 0, 0), 1);
        assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 0, 1, 0), 0);
        assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 0, 7, 0), 0);

        // Bit 0 set -> only x=7 is color 1
        vram[0] = 0x0001;
        assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 0, 7, 0), 1);
        assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 0, 0, 0), 0);
    }

    // y selects the row within the tile; tile_word_base offsets into VRAM.
    #[test]
    fn test_decode_2bpp_addressing() {
        let mut vram = Box::new([0u16; VRAM_SIZE / 2]);

        // Row offset: only row 3 has data
        vram[3] = 0x00FF;
        for x in 0..8 {
            assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 0, x, 3), 1);
            assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 0, x, 0), 0);
        }

        // tile_word_base offset: data at base 16
        vram[3] = 0x0000;
        vram[16] = 0x00FF;
        for x in 0..8 {
            assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 16, x, 0), 1);
            assert_eq!(Renderer::decode_2bpp_tile_pixel_from(&vram, 0, x, 0), 0);
        }
    }

    // ============================================================
    // render_scanline_mode0 - backdrop and transparency
    // ============================================================

    // Transparent tiles show the backdrop; color index 0 is always skipped.
    #[test]
    fn test_render_mode0_backdrop_and_transparency() {
        let mut renderer = make_renderer();
        let ppu = make_ppu_mode0();

        // Default CGRAM[0] = 0x0000 -> black backdrop
        renderer.render_scanline(&ppu, 0);
        for x in 0..SCREEN_WIDTH {
            assert_eq!(pixel(&renderer, x, 0), (0, 0, 0), "x={}", x);
        }

        // Set backdrop to white, all tiles transparent -> full white scanline
        let mut ppu = make_ppu_mode0();
        set_cgram_white(&mut ppu, 0);
        ppu.vram.memory[0] = 0x0000;
        renderer.render_scanline(&ppu, 0);
        let (br, bg, bb) = Renderer::apply_brightness(ppu.cgram.read(0), 15);
        for x in 0..SCREEN_WIDTH {
            assert_eq!(pixel(&renderer, x, 0), (br, bg, bb), "x={}", x);
        }

        // Tile with CHR all zero (color index 0) -> still shows backdrop
        let mut ppu2 = make_ppu_mode0();
        set_cgram_white(&mut ppu2, 0);
        ppu2.vram.memory[0] = 0x0001; // tile 1, CHR stays all zero
        renderer.render_scanline(&ppu2, 0);
        let (br, bg, bb) = Renderer::apply_brightness(ppu2.cgram.read(0), 15);
        for x in 0..SCREEN_WIDTH {
            assert_eq!(pixel(&renderer, x, 0), (br, bg, bb), "x={}", x);
        }
    }

    // ============================================================
    // render_scanline_mode0 - palette
    // ============================================================

    // color index selects the right CGRAM entry; palette_num shifts by blocks of 4.
    #[test]
    fn test_render_mode0_palette() {
        let mut renderer = make_renderer();
        let mut ppu = make_ppu_mode0();

        // Palette 0, color index 1 -> CGRAM[1]
        set_cgram_white(&mut ppu, 1);
        for col in 0..32 {
            ppu.vram.memory[col] = 0x0001; // all 32 tilemap columns: tile 1, palette 0
        }
        for row in 0..8 {
            ppu.vram.memory[CHR_BASE + 8 + row] = 0x00FF; // tile 1 CHR: all pixels color index 1
        }
        renderer.render_scanline_mode0(&ppu, 0);
        let expected = Renderer::apply_brightness(ppu.cgram.read(1), 15);
        for x in 0..SCREEN_WIDTH {
            assert_eq!(pixel(&renderer, x, 0), expected, "x={}", x);
        }

        // Palette 1, color index 1 -> CGRAM[5] (1 * 4 + 1)
        let mut ppu2 = make_ppu_mode0();
        let test_color: u16 = 0x001F; // red in BGR555
        ppu2.write(0x2121, 0x05);
        ppu2.write(0x2122, (test_color & 0xFF) as u8);
        ppu2.write(0x2122, (test_color >> 8) as u8);
        for col in 0..32 {
            ppu2.vram.memory[col] = 0x0401; // all 32 tilemap columns: tile 1, palette 1
        }
        for row in 0..8 {
            ppu2.vram.memory[CHR_BASE + 8 + row] = 0x00FF;
        }
        renderer.render_scanline_mode0(&ppu2, 0);
        let expected = Renderer::apply_brightness(ppu2.cgram.read(5), 15);
        for x in 0..SCREEN_WIDTH {
            assert_eq!(pixel(&renderer, x, 0), expected, "x={}", x);
        }
    }

    // ============================================================
    // render_scanline_mode0 - flip
    // ============================================================

    // flip_x mirrors pixels horizontally; flip_y mirrors rows vertically.
    #[test]
    fn test_render_mode0_flip() {
        let white;
        let black = (0u8, 0u8, 0u8);

        // Horizontal flip: leftmost pixel becomes rightmost
        {
            let mut r_normal = make_renderer();
            let mut r_flipped = make_renderer();
            let mut ppu_n = make_ppu_mode0();
            let mut ppu_f = make_ppu_mode0();

            for ppu in [&mut ppu_n, &mut ppu_f] {
                set_cgram_white(ppu, 1);
            }
            ppu_n.vram.memory[CHR_BASE + 8] = 0x0080; // tile 1 CHR row 0: only x=0 set (bit 7 of plane 0)
            ppu_f.vram.memory[CHR_BASE + 8] = 0x0080;
            ppu_n.vram.memory[0] = 0x0001; // no flip
            ppu_f.vram.memory[0] = 0x4001; // flip_x (bit 14)

            white = Renderer::apply_brightness(ppu_n.cgram.read(1), 15);

            r_normal.render_scanline_mode0(&ppu_n, 0);
            r_flipped.render_scanline_mode0(&ppu_f, 0);

            assert_eq!(pixel(&r_normal, 0, 0), white, "normal x=0");
            assert_eq!(pixel(&r_normal, 7, 0), black, "normal x=7");
            assert_eq!(pixel(&r_flipped, 0, 0), black, "flipped x=0");
            assert_eq!(pixel(&r_flipped, 7, 0), white, "flipped x=7");
        }

        // Vertical flip: row 0 becomes row 7
        {
            let mut r_normal = make_renderer();
            let mut r_flipped = make_renderer();
            let mut ppu_n = make_ppu_mode0();
            let mut ppu_f = make_ppu_mode0();

            for ppu in [&mut ppu_n, &mut ppu_f] {
                set_cgram_white(ppu, 1);
            }
            ppu_n.vram.memory[CHR_BASE + 8] = 0x00FF; // tile 1 CHR row 0: all pixels set
            ppu_f.vram.memory[CHR_BASE + 8] = 0x00FF;
            ppu_n.vram.memory[0] = 0x0001; // no flip
            ppu_f.vram.memory[0] = 0x8001; // flip_y (bit 15)

            // Scanline 0: normal sees row 0 (full), flipped sees row 7 (empty)
            r_normal.render_scanline_mode0(&ppu_n, 0);
            r_flipped.render_scanline_mode0(&ppu_f, 0);
            assert_eq!(pixel(&r_normal, 0, 0), white, "normal scanline 0");
            assert_eq!(pixel(&r_flipped, 0, 0), black, "flipped scanline 0");

            // Scanline 7: normal sees row 7 (empty), flipped sees row 0 (full)
            r_normal.render_scanline_mode0(&ppu_n, 7);
            r_flipped.render_scanline_mode0(&ppu_f, 7);
            assert_eq!(pixel(&r_normal, 0, 7), black, "normal scanline 7");
            assert_eq!(pixel(&r_flipped, 0, 7), white, "flipped scanline 7");
        }
    }

    // ============================================================
    // render_scanline_mode0 - scroll
    // ============================================================

    // Horizontal scroll shifts the visible tile column by the configured amount.
    #[test]
    fn test_render_mode0_scroll_x() {
        let mut renderer = make_renderer();
        let mut ppu = make_ppu_mode0();

        set_cgram_white(&mut ppu, 1);

        // Column 0: tile 0 (transparent), column 1: tile 1 (all pixels color 1)
        ppu.vram.memory[0] = 0x0000;
        ppu.vram.memory[1] = 0x0001;
        for row in 0..8 {
            ppu.vram.memory[CHR_BASE + 8 + row] = 0x00FF;
        }

        // Scroll by 8 -> column 1 appears at screen x=0
        ppu.write(0x210D, 0x08);
        ppu.write(0x210D, 0x00);

        renderer.render_scanline_mode0(&ppu, 0);

        let white = Renderer::apply_brightness(ppu.cgram.read(1), 15);
        assert_eq!(pixel(&renderer, 0, 0), white);
    }
}
