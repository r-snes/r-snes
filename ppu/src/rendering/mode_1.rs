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
