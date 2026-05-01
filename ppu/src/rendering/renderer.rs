use crate::constants::*;
use crate::ppu::PPU;
use crate::rendering::mode_1::Mode1Render;

pub struct Renderer {
    pub framebuffer: Box<[u8; SCREEN_WIDTH * SCREEN_HEIGHT * 3]>,
    pub current_brightness: u8,

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

        // Update brightness
        self.update_brightness(ppu.brightness());

        match ppu.regs.bg_mode() {
            1 => self.render_scanline_mode1(ppu, y),
            mode => {
                Self::render_full_black(self, y);
                println!("PPU mode {} not implemented", mode);
            }
        }
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

    pub fn apply_brightness(color: u16, brightness: u16) -> (u8, u8, u8) {
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

    pub fn set_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
        let index = (y * SCREEN_WIDTH + x) * 3;
        self.framebuffer[index] = r;
        self.framebuffer[index + 1] = g;
        self.framebuffer[index + 2] = b;
    }

    fn render_full_black(&mut self, y: usize) {
        for x in 0..SCREEN_WIDTH {
            self.set_pixel(x, y, 0, 0, 0);
        }
    }
}
