use crate::constants::*;
use crate::ppu::PPU;

/// Raw byte array of the framebuffer in RGB888
pub type RawFramebuffer = [u8; SCREEN_WIDTH * SCREEN_HEIGHT * 3];

// ============================================================
// Z-order (priority) values. Higher = closer to the front.
// A pixel is only overwritten when the incoming z is >= the z stored
// for that column. Every (layer, priority) pair gets a UNIQUE value, so
// draw order between layers is irrelevant and one scale serves both modes.
//
// Mode 0 (front -> back):
//   OBJ3 > BG1.1 > BG2.1 > OBJ2 > BG1.0 > BG2.0 > OBJ1 >
//   BG3.1 > BG4.1 > OBJ0 > BG3.0 > BG4.0 > backdrop
//
// Mode 1, BGMODE bit3 = 0 (front -> back):
//   OBJ3 > BG1.1 > BG2.1 > OBJ2 > BG1.0 > BG2.0 > OBJ1 >
//   BG3.1 > OBJ0 > BG3.0 > backdrop
//
// Mode 1, BGMODE bit3 = 1: BG3.1 is lifted above everything (Z_BG3_PRIO).
// ============================================================
pub const Z_BACKDROP: u8 = 0;
pub const Z_BG4_LOW: u8 = 1;
pub const Z_BG3_LOW: u8 = 2;
pub const Z_OBJ0: u8 = 3;
pub const Z_BG4_HIGH: u8 = 4;
pub const Z_BG3_HIGH: u8 = 5;
pub const Z_OBJ1: u8 = 6;
pub const Z_BG2_LOW: u8 = 7;
pub const Z_BG1_LOW: u8 = 8;
pub const Z_OBJ2: u8 = 9;
pub const Z_BG2_HIGH: u8 = 10;
pub const Z_BG1_HIGH: u8 = 11;
pub const Z_OBJ3: u8 = 12;
pub const Z_BG3_PRIO: u8 = 13; // mode 1, BGMODE bit3: BG3 high-prio above all

/// Parameters for rendering one BG layer on one scanline.
pub struct BgParams {
    pub tilemap_base: u16,
    pub tiledata_base: u16,
    pub scroll_x: usize,
    pub scroll_y: usize,
    pub bpp: u8,          // 2 or 4
    pub palette_base: u8, // CGRAM colour offset (mode 0 per-layer); 0 otherwise
    pub w64: bool,        // tilemap 64 tiles wide
    pub h64: bool,        // tilemap 64 tiles tall
    pub z_low: u8,
    pub z_high: u8,
}

pub struct Renderer {
    pub framebuffer: Box<RawFramebuffer>, // back buffer, PPU writes here
    pub presented: Box<RawFramebuffer>,   // front buffer, GUI reads here
    pub current_brightness: u8,

    /// Per-column z-order of the pixel currently written on the scanline
    /// being rendered. Reset to Z_BACKDROP at the start of each scanline.
    priority: Box<[u8; SCREEN_WIDTH]>,

    brightness_delay: u8,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            framebuffer: Box::new([0; SCREEN_WIDTH * SCREEN_HEIGHT * 3]),
            current_brightness: 15, // full brightness
            priority: Box::new([Z_BACKDROP; SCREEN_WIDTH]),
            brightness_delay: 0,
            presented: Box::new([0; SCREEN_WIDTH * SCREEN_HEIGHT * 3]),
        }
    }

    pub fn swap_buffers(&mut self) {
        std::mem::swap(&mut self.framebuffer, &mut self.presented);
    }

    pub fn presented(&self) -> &RawFramebuffer {
        &self.presented
    }

    pub fn render_scanline(&mut self, ppu: &PPU, y: usize) {
        // Hardware force blank: output black
        if ppu.force_blank() {
            self.render_full_black(y);
            return;
        }

        // Update brightness
        self.update_brightness(ppu.brightness());

        // Reset priorities and fill with the backdrop
        self.priority.fill(Z_BACKDROP);
        let backdrop = ppu.cgram.read(0);
        let (br, bg, bb) = Self::apply_brightness(backdrop, self.current_brightness as u16);
        for x in 0..SCREEN_WIDTH {
            self.set_pixel(x, y, br, bg, bb);
        }

        // Background layers
        match ppu.regs.bg_mode() {
            0 => self.render_scanline_mode0(ppu, y),
            1 => self.render_scanline_mode1(ppu, y),
            mode => {
                if y == 0 {
                    println!("bg_mode = {}", mode);
                }
                self.render_scanline_mode1(ppu, y); // TEMPORARY DEBUG - TODO
            }
        }

        // Sprites, if OBJ is enabled on the main screen (TM bit 4)
        if ppu.regs.tm & 0x10 != 0 {
            self.render_sprites(ppu, y);
        }
    }

    /// Render one BG layer for one scanline. Handles 2bpp/4bpp, tilemap sizes,
    /// per-tile flip/priority, and mode-0 palette offsets.
    pub fn render_bg_scanline(&mut self, ppu: &PPU, y: usize, p: &BgParams) {
        // if y == 0 {
        //     println!(
        //         "frame {} mode {} tm {:02X} ts {:02X} forceblank {} bright {}",
        //         ppu.frame, ppu.regs.bg_mode(), ppu.regs.tm, ppu.regs.ts,
        //         ppu.force_blank(), ppu.brightness()
        //     );
        // }
        let map_w = if p.w64 { 512 } else { 256 };
        let map_h = if p.h64 { 512 } else { 256 };
        let screens_wide = if p.w64 { 2 } else { 1 };

        let (tile_words, pal_shift) = if p.bpp == 2 { (8usize, 2u8) } else { (16, 4) };

        for x in 0..SCREEN_WIDTH {
            let px = (x + p.scroll_x) & (map_w - 1);
            let py = (y + p.scroll_y) & (map_h - 1);

            let tile_col = px >> 3;
            let tile_row = py >> 3;
            let fine_x = px & 7;
            let fine_y = py & 7;

            // Pick the 0x400-word sub-screen for maps larger than 32x32.
            let screen = (tile_row >> 5) * screens_wide + (tile_col >> 5);
            let map_word_addr = p.tilemap_base as usize
                + screen * 0x400
                + (tile_row & 0x1F) * 32
                + (tile_col & 0x1F);

            let entry = ppu.vram.memory[map_word_addr];
            let tile_index = entry & 0x03FF; // bits 9:0
            let palette_num = ((entry >> 10) & 0x07) as u8; // bits 12:10
            let priority = (entry & 0x2000) != 0; // bit 13
            let flip_x = (entry & 0x4000) != 0; // bit 14
            let flip_y = (entry & 0x8000) != 0; // bit 15

            let fx = if flip_x { 7 - fine_x } else { fine_x };
            let fy = if flip_y { 7 - fine_y } else { fine_y };

            let tile_word_base = p.tiledata_base as usize + tile_index as usize * tile_words;
            let color_index = if p.bpp == 2 {
                Self::decode_2bpp_tile_pixel_from(&ppu.vram.memory, tile_word_base, fx, fy)
            } else {
                Self::decode_4bpp_tile_pixel_from(&ppu.vram.memory, tile_word_base, fx, fy)
            };

            // Transparent pixel -> do nothing
            if color_index == 0 {
                continue;
            }

            let palette_entry = p.palette_base + (palette_num << pal_shift) + color_index;
            let color = ppu.cgram.read(palette_entry);

            let (r, g, b) = Self::apply_brightness(color, self.current_brightness as u16);
            let z = if priority { p.z_high } else { p.z_low };
            self.set_pixel_z(x, y, r, g, b, z);
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
        let mut r = color & 0x1F;
        let mut g = (color >> 5) & 0x1F;
        let mut b = (color >> 10) & 0x1F;

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

    /// Write a pixel only if its z-order is at least the one already stored
    /// for this column. On success, updates the stored z-order.
    pub fn set_pixel_z(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8, z: u8) {
        if z >= self.priority[x] {
            self.set_pixel(x, y, r, g, b);
            self.priority[x] = z;
        }
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
        let inidisp = if force_blank {
            0x80 | (brightness & 0x0F)
        } else {
            brightness & 0x0F
        };
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
        assert_eq!(renderer.framebuffer[idx], 0xAA);
        assert_eq!(renderer.framebuffer[idx + 1], 0xBB);
        assert_eq!(renderer.framebuffer[idx + 2], 0xCC);
    }

    /// set_pixel must not corrupt adjacent pixels.
    #[test]
    fn test_set_pixel_does_not_corrupt_neighbours() {
        let mut renderer = Renderer::new();
        renderer.set_pixel(5, 3, 0xFF, 0xFF, 0xFF);
        // pixel at (4, 3) and (6, 3) must stay black
        let left = (3 * SCREEN_WIDTH + 4) * 3;
        let right = (3 * SCREEN_WIDTH + 6) * 3;
        assert_eq!(renderer.framebuffer[left], 0);
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
        // expanded: (1<<3)|(1>>2) = 8|0 = 8 - just verify they're all equal and small
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
    // render_scanline - force blank
    // ============================================================

    /// When force blank is active, render_scanline must output a fully black scanline.
    #[test]
    fn test_render_scanline_force_blank_outputs_black() {
        let mut renderer = Renderer::new();
        // Pre-fill with non-black to detect overwrite
        for b in renderer.framebuffer.iter_mut() {
            *b = 0xFF;
        }
        let ppu = make_ppu_with_mode(1, true, 15);
        renderer.render_scanline(&ppu, 0);
        for x in 0..SCREEN_WIDTH {
            let idx = x * 3;
            assert_eq!(renderer.framebuffer[idx], 0, "R not black at x={}", x);
            assert_eq!(renderer.framebuffer[idx + 1], 0, "G not black at x={}", x);
            assert_eq!(renderer.framebuffer[idx + 2], 0, "B not black at x={}", x);
        }
    }

    /// Force blank must only black out the requested scanline, not the entire framebuffer.
    #[test]
    fn test_render_scanline_force_blank_only_affects_target_scanline() {
        let mut renderer = Renderer::new();
        for b in renderer.framebuffer.iter_mut() {
            *b = 0xFF;
        }
        let ppu = make_ppu_with_mode(1, true, 15);
        renderer.render_scanline(&ppu, 1); // blank scanline 1
        // Scanline 0 must be untouched
        assert_eq!(renderer.framebuffer[0], 0xFF);
    }

    // ============================================================
    // render_scanline - unimplemented mode falls back to black
    // ============================================================

    /// An unimplemented BG mode must output black for the scanline without panicking.
    #[test]
    fn test_render_scanline_unknown_mode_outputs_black() {
        let mut renderer = Renderer::new();
        for b in renderer.framebuffer.iter_mut() {
            *b = 0xFF;
        }
        let ppu = make_ppu_with_mode(0, false, 15); // mode 0 not implemented
        renderer.render_scanline(&ppu, 0);
        for x in 0..SCREEN_WIDTH {
            let idx = x * 3;
            assert_eq!(renderer.framebuffer[idx], 0);
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
