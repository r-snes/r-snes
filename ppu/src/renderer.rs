use crate::constants::*;
use crate::ppu::PPU;

pub struct Renderer {
    pub framebuffer: Vec<u8>,

    current_brightness: u8,
    brightness_delay: u8,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            framebuffer: vec![0; SCREEN_WIDTH * SCREEN_HEIGHT * 3],
            current_brightness: 15, // full brightness 
            brightness_delay: 0,
        }
    }

    fn update_brightness(&mut self, target: u8) {
        if self.current_brightness != target {
            if self.brightness_delay == 0 {
                self.brightness_delay = 72; // 1-chip fade
            }

            if self.brightness_delay > 0 {
                self.brightness_delay -= 1;

                if self.current_brightness < target {
                    self.current_brightness += 1;
                } else if self.current_brightness > target {
                    self.current_brightness -= 1;
                }
            }
        }
    }

    // pub fn render_scanline(&mut self, ppu: &PPU, y: usize) {
    //     let force_blank = ppu.force_blank();
    //     let target_brightness = ppu.brightness() & 0x0F;

    //     // Update brightness once per scanline
    //     self.update_brightness(target_brightness);
    //     let brightness = self.current_brightness as u16;

    //     for x in 0..SCREEN_WIDTH {
    //         if force_blank {
    //             self.set_pixel(x, y, 0, 0, 0);
    //             continue;
    //         }

    //         // Placeholder gradient
    //         let palette_index = ((x / 16) & 0xFF) as u8;
    //         let color = ppu.cgram.current_word(palette_index);
    //         // let color = ((x as u16) & 31) | (((x as u16) & 31) << 5) | (((x as u16) & 31) << 10); // tmp fix

    //         let mut r5 = (color & 0x1F) as u16;
    //         let mut g5 = ((color >> 5) & 0x1F) as u16;
    //         let mut b5 = ((color >> 10) & 0x1F) as u16;

    //         // Apply brightness (SNES: scale by /16)
    //         r5 = (r5 * brightness) >> 4;
    //         g5 = (g5 * brightness) >> 4;
    //         b5 = (b5 * brightness) >> 4;

    //         // Clamp safety (normally unnecessary but safe)
    //         r5 = r5.min(31);
    //         g5 = g5.min(31);
    //         b5 = b5.min(31);

    //         // Convert 5-bit -> 8-bit
    //         let r8 = ((r5 << 3) | (r5 >> 2)) as u8;
    //         let g8 = ((g5 << 3) | (g5 >> 2)) as u8;
    //         let b8 = ((b5 << 3) | (b5 >> 2)) as u8;

    //         self.set_pixel(x, y, r8, g8, b8);
    //     }
    // }

    pub fn render_scanline(&mut self, ppu: &PPU, y: usize) {
        let force_blank = ppu.force_blank();
        let target_brightness = ppu.brightness() & 0x0F;

        // Check BG mode and layer enable
        let bg_mode = ppu.regs.bg_mode();
        let bg1_enabled = ppu.regs.bg1_enabled();

        // Update brightness once per scanline
        self.update_brightness(target_brightness);
        let brightness = self.current_brightness as u16;

        for x in 0..SCREEN_WIDTH {
            // Hardware force blank
            if force_blank {
                self.set_pixel(x, y, 0, 0, 0);
                continue;
            }

            // Only render if Mode 1 and BG1 enabled
            if bg_mode != 1 || !bg1_enabled {
                self.set_pixel(x, y, 0, 0, 0);
                continue;
            }

            // Placeholder gradient (temporary until tile renderer)
            let palette_index = ((x / 16) & 0xFF) as u8;
            let color = ppu.cgram.current_word(palette_index);

            let mut r5 = (color & 0x1F) as u16;
            let mut g5 = ((color >> 5) & 0x1F) as u16;
            let mut b5 = ((color >> 10) & 0x1F) as u16;

            // Apply SNES brightness
            r5 = (r5 * brightness) >> 4;
            g5 = (g5 * brightness) >> 4;
            b5 = (b5 * brightness) >> 4;

            r5 = r5.min(31);
            g5 = g5.min(31);
            b5 = b5.min(31);

            // Convert BGR555 -> RGB888
            let r8 = ((r5 << 3) | (r5 >> 2)) as u8;
            let g8 = ((g5 << 3) | (g5 >> 2)) as u8;
            let b8 = ((b5 << 3) | (b5 >> 2)) as u8;

            self.set_pixel(x, y, r8, g8, b8);
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
        let index = (y * SCREEN_WIDTH + x) * 3;
        self.framebuffer[index] = r;
        self.framebuffer[index + 1] = g;
        self.framebuffer[index + 2] = b;
    }

    fn decode_tile_pixel(ppu: &PPU, tile_addr: u16, x: usize, y: usize) -> u8 {
        let base = (tile_addr as usize) * 2;
        let row = y;

        let p0 = ppu.vram.memory[base + row * 2];
        let p1 = ppu.vram.memory[base + row * 2 + 1];

        let p2 = ppu.vram.memory[base + 16 + row * 2];
        let p3 = ppu.vram.memory[base + 16 + row * 2 + 1];

        let bit = 7 - x;

        ((p0 >> bit) & 1)
            | (((p1 >> bit) & 1) << 1)
            | (((p2 >> bit) & 1) << 2)
            | (((p3 >> bit) & 1) << 3)
    }
}
