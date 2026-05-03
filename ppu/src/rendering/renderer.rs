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

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Helpers
    // ============================================================

    fn make_ppu_with_mode(mode: u8, force_blank: bool, brightness: u8) -> PPU {
        let mut ppu = PPU::new();
        // INIDISP: bit7 = force blank, bits[3:0] = brightness
        let inidisp = if force_blank { 0x80 | (brightness & 0x0F) } else { brightness & 0x0F };
        ppu.write(0x2100, inidisp);
        ppu.write(0x2105, mode & 0x07);
        ppu
    }

    // ============================================================
    // Renderer::new
    // ============================================================

    /// A freshly created Renderer must have a zeroed framebuffer and full brightness.
    #[test]
    fn test_new_initial_state() {
        let renderer = Renderer::new();
        assert!(renderer.framebuffer.iter().all(|&b| b == 0));
        assert_eq!(renderer.current_brightness, 15);
    }

    // ============================================================
    // set_pixel
    // ============================================================

    /// set_pixel must write R, G, B at the correct framebuffer offset.
    #[test]
    fn test_set_pixel_writes_correct_offset() {
        let mut renderer = Renderer::new();
        renderer.set_pixel(0, 0, 0xFF, 0x80, 0x00);
        assert_eq!(renderer.framebuffer[0], 0xFF);
        assert_eq!(renderer.framebuffer[1], 0x80);
        assert_eq!(renderer.framebuffer[2], 0x00);
    }

    /// set_pixel at (1, 0) must write at byte offset 3.
    #[test]
    fn test_set_pixel_x1_y0_offset() {
        let mut renderer = Renderer::new();
        renderer.set_pixel(1, 0, 0x11, 0x22, 0x33);
        assert_eq!(renderer.framebuffer[3], 0x11);
        assert_eq!(renderer.framebuffer[4], 0x22);
        assert_eq!(renderer.framebuffer[5], 0x33);
    }

    /// set_pixel at (0, 1) must write at byte offset SCREEN_WIDTH * 3.
    #[test]
    fn test_set_pixel_x0_y1_offset() {
        let mut renderer = Renderer::new();
        renderer.set_pixel(0, 1, 0xAA, 0xBB, 0xCC);
        let idx = SCREEN_WIDTH * 3;
        assert_eq!(renderer.framebuffer[idx],     0xAA);
        assert_eq!(renderer.framebuffer[idx + 1], 0xBB);
        assert_eq!(renderer.framebuffer[idx + 2], 0xCC);
    }

    /// set_pixel must not corrupt adjacent pixels.
    #[test]
    fn test_set_pixel_does_not_corrupt_neighbours() {
        let mut renderer = Renderer::new();
        renderer.set_pixel(5, 3, 0xFF, 0xFF, 0xFF);
        // pixel at (4, 3) and (6, 3) must stay black
        let left  = (3 * SCREEN_WIDTH + 4) * 3;
        let right = (3 * SCREEN_WIDTH + 6) * 3;
        assert_eq!(renderer.framebuffer[left],  0);
        assert_eq!(renderer.framebuffer[right], 0);
    }

    // ============================================================
    // apply_brightness
    // ============================================================

    /// At brightness 0, all colour channels must be scaled to near-zero.
    #[test]
    fn test_apply_brightness_zero_dims_all_channels() {
        // White in BGR555: 0x7FFF (r=31, g=31, b=31)
        let (r, g, b) = Renderer::apply_brightness(0x7FFF, 0);
        // brightness+1 = 1, >> 4 -> each channel = 31*1>>4 = 1
        // expanded: (1<<3)|(1>>2) = 8|0 = 8 — just verify they're all equal and small
        assert_eq!(r, g);
        assert_eq!(g, b);
        assert!(r < 16);
    }

    /// At full brightness (15), white must map to (255, 255, 255).
    #[test]
    fn test_apply_brightness_full_white() {
        let (r, g, b) = Renderer::apply_brightness(0x7FFF, 15);
        // 31 * 16 >> 4 = 31; expanded: (31<<3)|(31>>2) = 248|7 = 255
        assert_eq!(r, 255);
        assert_eq!(g, 255);
        assert_eq!(b, 255);
    }

    /// At full brightness, black (0x0000) must map to (0, 0, 0).
    #[test]
    fn test_apply_brightness_full_black_color() {
        let (r, g, b) = Renderer::apply_brightness(0x0000, 15);
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    /// apply_brightness must extract R from bits[4:0], G from bits[9:5], B from bits[14:10].
    #[test]
    fn test_apply_brightness_channel_extraction() {
        // Pure red in BGR555: bits[4:0]=31, rest=0 -> 0x001F
        let (r, g, b) = Renderer::apply_brightness(0x001F, 15);
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        // Pure green: bits[9:5]=31 -> 0x03E0
        let (r, g, b) = Renderer::apply_brightness(0x03E0, 15);
        assert_eq!(r, 0);
        assert_eq!(g, 255);
        assert_eq!(b, 0);

        // Pure blue: bits[14:10]=31 -> 0x7C00
        let (r, g, b) = Renderer::apply_brightness(0x7C00, 15);
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 255);
    }

    /// apply_brightness must produce monotonically brighter output on all channels as brightness increases.
    #[test]
    fn test_apply_brightness_mid_brightness_monotone() {
        let mut prev_r = 0u8;
        let mut prev_g = 0u8;
        let mut prev_b = 0u8;
        for brightness in 0u16..=15 {
            let (r, g, b) = Renderer::apply_brightness(0x7FFF, brightness);
            assert!(r >= prev_r, "R not monotone at brightness {}", brightness);
            assert!(g >= prev_g, "G not monotone at brightness {}", brightness);
            assert!(b >= prev_b, "B not monotone at brightness {}", brightness);
            prev_r = r;
            prev_g = g;
            prev_b = b;
        }
    }

    // ============================================================
    // render_scanline — force blank
    // ============================================================

    /// When force blank is active, render_scanline must output a fully black scanline.
    #[test]
    fn test_render_scanline_force_blank_outputs_black() {
        let mut renderer = Renderer::new();
        // Pre-fill with non-black to detect overwrite
        for b in renderer.framebuffer.iter_mut() { *b = 0xFF; }
        let ppu = make_ppu_with_mode(1, true, 15);
        renderer.render_scanline(&ppu, 0);
        for x in 0..SCREEN_WIDTH {
            let idx = x * 3;
            assert_eq!(renderer.framebuffer[idx],     0, "R not black at x={}", x);
            assert_eq!(renderer.framebuffer[idx + 1], 0, "G not black at x={}", x);
            assert_eq!(renderer.framebuffer[idx + 2], 0, "B not black at x={}", x);
        }
    }

    /// Force blank must only black out the requested scanline, not the entire framebuffer.
    #[test]
    fn test_render_scanline_force_blank_only_affects_target_scanline() {
        let mut renderer = Renderer::new();
        for b in renderer.framebuffer.iter_mut() { *b = 0xFF; }
        let ppu = make_ppu_with_mode(1, true, 15);
        renderer.render_scanline(&ppu, 1); // blank scanline 1
        // Scanline 0 must be untouched
        assert_eq!(renderer.framebuffer[0], 0xFF);
    }

    // ============================================================
    // render_scanline — unimplemented mode falls back to black
    // ============================================================

    /// An unimplemented BG mode must output black for the scanline without panicking.
    #[test]
    fn test_render_scanline_unknown_mode_outputs_black() {
        let mut renderer = Renderer::new();
        for b in renderer.framebuffer.iter_mut() { *b = 0xFF; }
        let ppu = make_ppu_with_mode(0, false, 15); // mode 0 not implemented
        renderer.render_scanline(&ppu, 0);
        for x in 0..SCREEN_WIDTH {
            let idx = x * 3;
            assert_eq!(renderer.framebuffer[idx],     0);
            assert_eq!(renderer.framebuffer[idx + 1], 0);
            assert_eq!(renderer.framebuffer[idx + 2], 0);
        }
    }

    // ============================================================
    // update_brightness (tested via render_scanline)
    // ============================================================

    /// When target brightness equals current, current_brightness must not change.
    #[test]
    fn test_brightness_no_change_when_already_at_target() {
        let mut renderer = Renderer::new();
        renderer.current_brightness = 15;
        let ppu = make_ppu_with_mode(1, false, 15);
        renderer.render_scanline(&ppu, 0);
        assert_eq!(renderer.current_brightness, 15);
    }

    /// When target differs, the first call must set the delay without changing brightness.
    #[test]
    fn test_brightness_first_change_sets_delay() {
        let mut renderer = Renderer::new();
        renderer.current_brightness = 15;
        let ppu = make_ppu_with_mode(1, false, 0); // target = 0
        renderer.render_scanline(&ppu, 0);
        // First call: delay was 0 -> set to 72, brightness unchanged
        assert_eq!(renderer.current_brightness, 15);
    }

     /// After the delay counts down, brightness must step by 1 toward the target each call.
    #[test]
    fn test_brightness_steps_toward_target_after_delay() {
        let mut renderer = Renderer::new();
        renderer.current_brightness = 15;
        let ppu = make_ppu_with_mode(1, false, 0);

        // Call 1: delay was 0 -> set to 72, no brightness change yet
        renderer.render_scanline(&ppu, 0);
        assert_eq!(renderer.current_brightness, 15);

        // Call 2: delay 72 -> 71, brightness steps 15 -> 14
        renderer.render_scanline(&ppu, 0);
        assert_eq!(renderer.current_brightness, 14);
    }
}
