use crate::constants::*;
use crate::ppu::PPU;

pub struct Renderer {
    pub framebuffer: Box<[u8; SCREEN_WIDTH * SCREEN_HEIGHT * 3]>,

    current_brightness: u8,
    brightness_delay: u8,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            framebuffer: Box::new([0; SCREEN_WIDTH * SCREEN_HEIGHT * 3]),
            current_brightness: 15, // full brightness 
            brightness_delay: 0,
        }
    }

    pub fn render_scanline(&mut self, ppu: &PPU, y: usize) {
        // Hardware force blank: output black
        if ppu.force_blank() {
            Self::render_full_black(self, y);
            return;
        }

        // Update brightness once per scanline
        self.update_brightness(ppu.brightness());
        let brightness = self.current_brightness as u16;

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
            let flip_x = (entry & 0x4000) != 0; // bit 14
            let flip_y = (entry & 0x8000) != 0; // bit 15

            // Apply flip
            let fx = if flip_x { 7 - fine_x } else { fine_x };
            let fy = if flip_y { 7 - fine_y } else { fine_y };

            // ============================================================
            // Decode 4bpp pixel from CHR data
            // ============================================================
            let tile_word_base = tiledata_base as usize + tile_index as usize * 16;
            let color_index = Self::decode_tile_pixel_from(&ppu.vram.memory, tile_word_base, fx, fy);

            // Transparent pixel -> do nothing
            if color_index == 0 {
                continue;
            }

            let palette_entry = ((palette_num as u8) << 4) | color_index;
            let color = ppu.cgram.read(palette_entry);

            let (r, g, b) = Self::apply_brightness(color, brightness);
            self.set_pixel(x, y, r, g, b);
        }
    }

    fn apply_brightness(color: u16, brightness: u16) -> (u8, u8, u8) {
        let mut r = (color & 0x1F) as u16;
        let mut g = ((color >> 5) & 0x1F) as u16;
        let mut b = ((color >> 10) & 0x1F) as u16;

        r = (r * (brightness + 1)) >> 4;
        g = (g * (brightness + 1)) >> 4;
        b = (b * (brightness + 1)) >> 4;

        let r8 = ((r << 3) | (r >> 2)) as u8;
        let g8 = ((g << 3) | (g >> 2)) as u8;
        let b8 = ((b << 3) | (b >> 2)) as u8;

        (r8, g8, b8)
    }

    fn update_brightness(&mut self, target: u8) {
        if self.current_brightness == target {
            return;
        }

        if self.brightness_delay == 0 {
            self.brightness_delay = 72;
            return;
        }

        self.brightness_delay -= 1;

        if self.current_brightness < target {
            self.current_brightness += 1;
        } else {
            self.current_brightness -= 1;
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
        let index = (y * SCREEN_WIDTH + x) * 3;
        self.framebuffer[index] = r;
        self.framebuffer[index + 1] = g;
        self.framebuffer[index + 2] = b;
    }

    fn decode_tile_pixel_from(vram: &[u16], tile_word_base: usize, x: usize, y: usize) -> u8 {
        let w01 = vram[tile_word_base + y];
        let p0 = (w01 & 0xFF) as u8;       // lo byte = plane 0
        let p1 = (w01 >> 8) as u8;         // hi byte = plane 1

        // Plane 2+3 : 8 mots plus loin (tiles 4bpp = 16 mots total)
        let w23 = vram[tile_word_base + 8 + y];
        let p2 = (w23 & 0xFF) as u8;       // lo byte = plane 2
        let p3 = (w23 >> 8) as u8;         // hi byte = plane 3

        let bit = 7 - x;
        ((p0 >> bit) & 1)
            | (((p1 >> bit) & 1) << 1)
            | (((p2 >> bit) & 1) << 2)
            | (((p3 >> bit) & 1) << 3)
    }

    fn render_full_black(&mut self, y: usize) {
        for x in 0..SCREEN_WIDTH {
            self.set_pixel(x, y, 0, 0, 0);
        }
    }
}
