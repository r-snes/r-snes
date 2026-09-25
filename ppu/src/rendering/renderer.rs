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

/// Identity of the layer that produced a pixel. Needed by color math, which
/// enables/disables per layer (CGADSUB) and treats OBJ specially.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Backdrop,
    Bg1,
    Bg2,
    Bg3,
    Bg4,
    Obj,
}

impl Layer {
    pub fn from_bg(bg: usize) -> Layer {
        match bg {
            0 => Layer::Bg1,
            1 => Layer::Bg2,
            2 => Layer::Bg3,
            _ => Layer::Bg4,
        }
    }

    // CGADSUB layer-enable bit index (BG1=0..BG4=3, OBJ=4, backdrop=5)
    fn math_bit(self) -> u8 {
        match self {
            Layer::Bg1 => 0,
            Layer::Bg2 => 1,
            Layer::Bg3 => 2,
            Layer::Bg4 => 3,
            Layer::Obj => 4,
            Layer::Backdrop => 5,
        }
    }
}

/// One composited pixel of a screen (main or sub), before color math.
#[derive(Clone, Copy)]
pub struct LinePixel {
    pub color: u16,
    pub z: u8,
    pub layer: Layer,
    // OBJ pixels only do color math when the sprite uses palettes 4-7.
    pub obj_math: bool,
}

impl LinePixel {
    const BACKDROP: LinePixel = LinePixel {
        color: 0,
        z: Z_BACKDROP,
        layer: Layer::Backdrop,
        obj_math: false,
    };
}

/// Parameters for rendering one BG layer on one scanline.
pub struct BgParams<'a> {
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
    pub layer: Layer,
    pub to_main: bool, // enabled on main screen (TM)
    pub to_sub: bool,  // enabled on sub screen (TS)
    pub window: &'a [bool; 256], // per-column window region for this layer
    pub win_main: bool, // window removes this layer from main (TMW)
    pub win_sub: bool,  // window removes this layer from sub (TSW)
}

pub struct Renderer {
    pub framebuffer: Box<RawFramebuffer>, // back buffer, PPU writes here
    pub presented: Box<RawFramebuffer>,   // front buffer, GUI reads here
    pub current_brightness: u8,

    // Per-column top pixel of each screen for the scanline being rendered.
    main_line: Box<[LinePixel; SCREEN_WIDTH]>,
    sub_line: Box<[LinePixel; SCREEN_WIDTH]>,

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
            main_line: Box::new([LinePixel::BACKDROP; SCREEN_WIDTH]),
            sub_line: Box::new([LinePixel::BACKDROP; SCREEN_WIDTH]),
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

        // Main backdrop = CGRAM[0]; sub backdrop = fixed colour (COLDATA).
        let main_bd = LinePixel {
            color: ppu.cgram.read(0),
            ..LinePixel::BACKDROP
        };
        let sub_bd = LinePixel {
            color: ppu.regs.coldata,
            ..LinePixel::BACKDROP
        };
        self.main_line.fill(main_bd);
        self.sub_line.fill(sub_bd);

        // Background layers -> deposit into main_line / sub_line
        match ppu.regs.bg_mode() {
            0 => self.render_scanline_mode0(ppu, y),
            1 => self.render_scanline_mode1(ppu, y),
            mode => self.render_scanline_mode1(ppu, y),
            // mode => {
            //     self.render_full_black(y);
            //     println!("PPU mode {} not implemented", mode);
            //     return;
            // }
        }

        // Sprites deposit into main_line / sub_line too (gated on TM/TS bit 4).
        self.render_sprites(ppu, y);

        // Final pass: main + sub -> color math -> brightness -> framebuffer.
        self.composite_line(ppu, y);
    }

    /// Render one BG layer for one scanline into the main and/or sub screen.
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
            let z = if priority { p.z_high } else { p.z_low };

            let pixel = LinePixel {
                color,
                z,
                layer: p.layer,
                obj_math: false,
            };
            if p.to_main {
                Self::deposit(&mut self.main_line, x, pixel);
            }
            if p.to_sub {
                Self::deposit(&mut self.sub_line, x, pixel);
            }

                        let pixel = LinePixel {
                color,
                z,
                layer: p.layer,
                obj_math: false,
            };

            // Window removes the layer per-screen where TMW/TSW enable it
            let masked = p.window[x];
            if p.to_main && !(p.win_main && masked) {
                Self::deposit(&mut self.main_line, x, pixel);
            }
            if p.to_sub && !(p.win_sub && masked) {
                Self::deposit(&mut self.sub_line, x, pixel);
            }
        }
    }

    /// Deposit an OBJ pixel. Called by the sprite renderer.
    /// `obj_math` must be true if the sprite uses palette 4-7.
    pub fn deposit_main(&mut self, x: usize, color: u16, z: u8, layer: Layer, obj_math: bool) {
        Self::deposit(
            &mut self.main_line,
            x,
            LinePixel { color, z, layer, obj_math },
        );
    }

    pub fn deposit_sub(&mut self, x: usize, color: u16, z: u8, layer: Layer, obj_math: bool) {
        Self::deposit(
            &mut self.sub_line,
            x,
            LinePixel { color, z, layer, obj_math },
        );
    }

    fn deposit(line: &mut [LinePixel; SCREEN_WIDTH], x: usize, px: LinePixel) {
        if px.z >= line[x].z {
            line[x] = px;
        }
    }

    /// Combine main + sub per pixel, apply color math and brightness.
    fn composite_line(&mut self, ppu: &PPU, y: usize) {
        let cgwsel = ppu.regs.cgwsel;
        let cgadsub = ppu.regs.cgadsub;

        let subtract = cgadsub & 0x80 != 0;
        let half = cgadsub & 0x40 != 0;
        let use_subscreen = cgwsel & 0x02 != 0; // CGWSEL bit1 (A)
        let fixed = ppu.regs.coldata;
        let brightness = self.current_brightness as u16;

        let clip_mode = (cgwsel >> 6) & 0x03; // 0=never 1=out 2=in 3=always
        let math_region = (cgwsel >> 4) & 0x03; // 0=always 1=in 2=out 3=never

        for x in 0..SCREEN_WIDTH {
            let main = self.main_line[x];

            // Clip main screen to black. Window-relative cases (1/2) need the
            // color window -> handled when window masking lands.
            let force_black = clip_mode == 3;
            let main_color = if force_black { 0 } else { main.color };

            // Is color math active on this pixel?
            let region_ok = math_region != 3; // 1/2 need color window
            let layer_enabled = match main.layer {
                Layer::Obj => (cgadsub & 0x10 != 0) && main.obj_math,
                l => cgadsub & (1 << l.math_bit()) != 0,
            };

            let out = if region_ok && layer_enabled {
                let sub = if use_subscreen {
                    self.sub_line[x].color
                } else {
                    fixed
                };
                Self::color_math(main_color, sub, subtract, half)
            } else {
                main_color
            };

            let (r, g, b) = Self::apply_brightness(out, brightness);
            self.set_pixel(x, y, r, g, b);
        }
    }

    /// Per-channel BGR555 add/subtract with optional halving.
    fn color_math(main: u16, sub: u16, subtract: bool, half: bool) -> u16 {
        let (mr, mg, mb) = (main & 0x1F, (main >> 5) & 0x1F, (main >> 10) & 0x1F);
        let (sr, sg, sb) = (sub & 0x1F, (sub >> 5) & 0x1F, (sub >> 10) & 0x1F);

        let (mut r, mut g, mut b) = if subtract {
            (mr.saturating_sub(sr), mg.saturating_sub(sg), mb.saturating_sub(sb))
        } else {
            ((mr + sr).min(31), (mg + sg).min(31), (mb + sb).min(31))
        };

        if half {
            r >>= 1;
            g >>= 1;
            b >>= 1;
        }

        r | (g << 5) | (b << 10)
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
