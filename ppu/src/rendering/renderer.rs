//! Scanline renderer: composes the PPU layers into an RGB888 framebuffer.
//!
//! Each scanline starts with the backdrop color (CGRAM entry 0), then the BG mode
//! renderer and the sprites draw on top through a per-column z-buffer.
//! Force blank outputs black and INIDISP brightness is applied to every pixel.
//! The framebuffer is double-buffered: the PPU writes to the back buffer,
//! the GUI reads the front one.

use crate::constants::*;
use crate::ppu::PPU;

/// Raw byte array of the framebuffer in RGB888
pub type RawFramebuffer = [u8; SCREEN_WIDTH * SCREEN_HEIGHT * 3];

// ============================================================
// Z-order (priority) scale. Higher = closer to the front.
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
// Mode 1, BGMODE bit3 = 1: BG3.1 is lifted above everything (Bg3Prio).
// ============================================================

/// Z-order of a pixel on the priority scale, from back to front.
/// Declaration order is the priority order: the derived `Ord` compares variants by it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Backdrop, always behind everything.
    Backdrop,
    /// Low priority BG4 tiles.
    Bg4Low,
    /// Low priority BG3 tiles.
    Bg3Low,
    /// Priority 0 sprites.
    Obj0,
    /// High priority BG4 tiles.
    Bg4High,
    /// High priority BG3 tiles.
    Bg3High,
    /// Priority 1 sprites.
    Obj1,
    /// Low priority BG2 tiles.
    Bg2Low,
    /// Low priority BG1 tiles.
    Bg1Low,
    /// Priority 2 sprites.
    Obj2,
    /// High priority BG2 tiles.
    Bg2High,
    /// High priority BG1 tiles.
    Bg1High,
    /// Priority 3 sprites.
    Obj3,
    /// High priority BG3 tiles in mode 1 with BGMODE bit 3 set, above everything.
    Bg3Prio,
}

/// Bit depth of a BG layer's tiles. Drives tile size in VRAM and palette shift.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BitDepth {
    /// 2 bits per pixel (4 colors per palette).
    Two,
    /// 4 bits per pixel (16 colors per palette).
    Four,
}

impl BitDepth {
    // Words per tile in VRAM: 2bpp = 8, 4bpp = 16.
    fn tile_words(self) -> usize {
        match self {
            BitDepth::Two => 8,
            BitDepth::Four => 16,
        }
    }

    // Palette block shift: 2bpp = 4 colours (shift 2), 4bpp = 16 colours (shift 4).
    fn pal_shift(self) -> u8 {
        match self {
            BitDepth::Two => 2,
            BitDepth::Four => 4,
        }
    }
}

/// Identity of the layer that produced a pixel. Needed by color math, which
/// enables/disables per layer (CGADSUB) and treats OBJ specially.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Backdrop color.
    Backdrop,
    /// Background layer 1.
    Bg1,
    /// Background layer 2.
    Bg2,
    /// Background layer 3.
    Bg3,
    /// Background layer 4.
    Bg4,
    /// Sprites.
    Obj,
}

impl Layer {
    /// Returns the layer of BG index `bg` (0 = BG1 .. 3 = BG4).
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
    /// BGR555 color.
    pub color: u16,
    /// Z-order of the pixel.
    pub z: Priority,
    /// Layer that produced the pixel.
    pub layer: Layer,
    /// True if color math applies to this OBJ pixel (sprite palettes 4-7 only).
    pub obj_math: bool,
}

impl LinePixel {
    const BACKDROP: LinePixel = LinePixel {
        color: 0,
        z: Priority::Backdrop,
        layer: Layer::Backdrop,
        obj_math: false,
    };
}

/// Parameters for rendering one BG layer on one scanline.
pub struct BgParams {
    /// Tilemap word address in VRAM.
    pub tilemap_base: u16,
    /// CHR data word address in VRAM.
    pub tiledata_base: u16,
    /// Horizontal scroll.
    pub scroll_x: usize,
    /// Vertical scroll.
    pub scroll_y: usize,
    /// Tile bit depth.
    pub bpp: BitDepth,
    /// CGRAM color offset (mode 0 per-layer block); 0 otherwise.
    pub palette_base: u8,
    /// Tilemap is 64 tiles wide.
    pub w64: bool,
    /// Tilemap is 64 tiles tall.
    pub h64: bool,
    /// Z-order of low priority tiles.
    pub z_low: Priority,
    /// Z-order of high priority tiles.
    pub z_high: Priority,
    /// Layer identity, for color math.
    pub layer: Layer,
    /// Enabled on the main screen (TM).
    pub to_main: bool,
    /// Enabled on the sub screen (TS).
    pub to_sub: bool,
}

/// Double-buffered framebuffer and per-scanline rendering state.
pub struct Renderer {
    /// Back buffer, the PPU writes here.
    pub framebuffer: Box<RawFramebuffer>,
    /// Front buffer, the GUI reads here.
    pub presented: Box<RawFramebuffer>,
    /// Brightness currently applied to the output (0-15).
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
    /// Creates a renderer with black buffers at full brightness.
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

    /// Swaps the back and front buffers, making the last rendered frame visible.
    pub fn swap_buffers(&mut self) {
        std::mem::swap(&mut self.framebuffer, &mut self.presented);
    }

    /// Returns the front buffer (last complete frame).
    pub fn presented(&self) -> &RawFramebuffer {
        &self.presented
    }

    /// Renders framebuffer row `y`: backdrop, BG layers, then sprites.
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
            _ => self.render_scanline_mode1(ppu, y),
            // _ => {
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

        let tile_words = p.bpp.tile_words();
        let pal_shift = p.bpp.pal_shift();

        for x in 0..SCREEN_WIDTH {
            let px = (x + p.scroll_x) & (map_w - 1);
            let py = (y + p.scroll_y) & (map_h - 1);

            let tile_col = px >> 3;
            let tile_row = py >> 3;
            let fine_x = px & 7;
            let fine_y = py & 7;

            // Pick the 0x400-word sub-screen for maps larger than 32x32.
            let screen = (tile_row >> 5) * screens_wide + (tile_col >> 5);
            let map_word_addr = (p.tilemap_base as usize
                + screen * 0x400
                + (tile_row & 0x1F) * 32
                + (tile_col & 0x1F))
                & 0x7FFF;

            let entry = ppu.vram.memory[map_word_addr];
            let tile_index = entry & 0x03FF; // bits 9:0
            let palette_num = ((entry >> 10) & 0x07) as u8; // bits 12:10
            let priority = (entry & 0x2000) != 0; // bit 13
            let flip_x = (entry & 0x4000) != 0; // bit 14
            let flip_y = (entry & 0x8000) != 0; // bit 15

            let fx = if flip_x { 7 - fine_x } else { fine_x };
            let fy = if flip_y { 7 - fine_y } else { fine_y };

            let tile_word_base = p.tiledata_base as usize + tile_index as usize * tile_words;
            let color_index = match p.bpp {
                BitDepth::Two => {
                    Self::decode_2bpp_tile_pixel_from(&ppu.vram.memory, tile_word_base, fx, fy)
                }
                BitDepth::Four => {
                    Self::decode_4bpp_tile_pixel_from(&ppu.vram.memory, tile_word_base, fx, fy)
                }
            };

            // Transparent pixel -> do nothing
            if color_index == 0 {
                continue;
            }

            let palette_entry = p.palette_base + (palette_num << pal_shift) + color_index;
            let color = ppu.cgram.read(palette_entry);
            let prio = if priority { p.z_high } else { p.z_low };

            let pixel = LinePixel {
                color,
                z: prio,
                layer: p.layer,
                obj_math: false,
            };
            if p.to_main {
                Self::deposit(&mut self.main_line, x, pixel);
            }
            if p.to_sub {
                Self::deposit(&mut self.sub_line, x, pixel);
            }
        }
    }

    /// Deposit an OBJ pixel. Called by the sprite renderer.
    /// `obj_math` must be true if the sprite uses palette 4-7.
    pub fn deposit_main(
        &mut self,
        x: usize,
        color: u16,
        prio: Priority,
        layer: Layer,
        obj_math: bool,
    ) {
        Self::deposit(
            &mut self.main_line,
            x,
            LinePixel {
                color,
                z: prio,
                layer,
                obj_math,
            },
        );
    }

    /// Deposits an OBJ pixel on the sub screen. `obj_math` must be true if the sprite uses palette 4-7.
    pub fn deposit_sub(
        &mut self,
        x: usize,
        color: u16,
        prio: Priority,
        layer: Layer,
        obj_math: bool,
    ) {
        Self::deposit(
            &mut self.sub_line,
            x,
            LinePixel {
                color,
                z: prio,
                layer,
                obj_math,
            },
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
            (
                mr.saturating_sub(sr),
                mg.saturating_sub(sg),
                mb.saturating_sub(sb),
            )
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

    /// Converts a BGR555 color to RGB888, scaled by `brightness` (0-15).
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

    /// Writes an RGB pixel to the back buffer, ignoring the z-buffer.
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
    use crate::ppu::PPU;

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

    fn make_renderer() -> Renderer {
        let mut r = Renderer::new();
        r.current_brightness = 15;
        r
    }

    // Mode 1 PPU, full brightness, no force blank.
    fn make_ppu() -> PPU {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0x0F);
        ppu.write(0x2105, 0x01);
        ppu
    }

    fn set_color(ppu: &mut PPU, entry: u8, color: u16) {
        ppu.write(0x2121, entry);
        ppu.write(0x2122, (color & 0xFF) as u8);
        ppu.write(0x2122, (color >> 8) as u8);
    }

    fn fb_pixel(r: &Renderer, x: usize) -> (u8, u8, u8) {
        let i = x * 3;
        (r.framebuffer[i], r.framebuffer[i + 1], r.framebuffer[i + 2])
    }

    // BG1 (mode 1, 4bpp): tilemap word 0x0000, CHR word 0x1000, palette 0
    // entry 1 = `color`. Renders `color` on every pixel of the layer.
    fn setup_bg1_uniform(ppu: &mut PPU, color: u16) {
        ppu.write(0x2107, 0x00); // BG1SC tilemap 0x0000, 32x32
        let nba = (ppu.regs.bg12nba & 0xF0) | 0x01;
        ppu.write(0x210B, nba); // BG1 CHR nibble 1 -> word 0x1000
        for row in 0..8 {
            ppu.vram.memory[0x1000 + row] = 0x00FF; // plane 0 -> color index 1
        }
        set_color(ppu, 0x01, color);
    }

    // BG2 (mode 1, 4bpp): tilemap word 0x0800, CHR word 0x2000, palette 1
    // entry 1 = `color`.
    fn setup_bg2_uniform(ppu: &mut PPU, color: u16) {
        ppu.write(0x2108, 0x08); // BG2SC tilemap word 0x0800
        let nba = (ppu.regs.bg12nba & 0x0F) | (0x02 << 4);
        ppu.write(0x210B, nba); // BG2 CHR nibble 2 -> word 0x2000
        for i in 0..(32 * 32) {
            ppu.vram.memory[0x0800 + i] = 0x0400; // tile 0, palette 1
        }
        for row in 0..8 {
            ppu.vram.memory[0x2000 + row] = 0x00FF;
        }
        set_color(ppu, 0x11, color); // palette 1 entry 1
    }

    // BG3 (mode 1, 2bpp): tilemap word 0x0C00, CHR word 0x3000, palette 2,
    // priority bit set on every tile. entry = `color`.
    fn setup_bg3_high_prio(ppu: &mut PPU, color: u16) {
        ppu.write(0x2109, 0x0C); // BG3SC tilemap word 0x0C00
        ppu.write(0x210C, 0x03); // BG34NBA: BG3 CHR nibble 3 -> word 0x3000
        for i in 0..(32 * 32) {
            ppu.vram.memory[0x0C00 + i] = 0x2800; // tile 0, palette 2, priority bit
        }
        for row in 0..8 {
            ppu.vram.memory[0x3000 + row] = 0x00FF; // 2bpp plane 0 -> index 1
        }
        set_color(ppu, 9, color); // palette 2 entry 1 (2*4 + 1)
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

    // ============================================================
    // color_math (pure)
    // ============================================================

    #[test]
    fn test_color_math_add() {
        assert_eq!(Renderer::color_math(0x0001, 0x0002, false, false), 0x0003);
    }

    #[test]
    fn test_color_math_add_saturates() {
        // 31 + 31 clamps to 31
        assert_eq!(Renderer::color_math(0x001F, 0x001F, false, false), 0x001F);
    }

    #[test]
    fn test_color_math_subtract() {
        assert_eq!(Renderer::color_math(0x0005, 0x0002, true, false), 0x0003);
    }

    #[test]
    fn test_color_math_subtract_clamps_to_zero() {
        assert_eq!(Renderer::color_math(0x0002, 0x0005, true, false), 0x0000);
    }

    #[test]
    fn test_color_math_half_add() {
        // (8 + 4) / 2 = 6
        assert_eq!(Renderer::color_math(0x0008, 0x0004, false, true), 0x0006);
    }

    #[test]
    fn test_color_math_half_subtract() {
        // (12 - 4) / 2 = 4
        assert_eq!(Renderer::color_math(0x000C, 0x0004, true, true), 0x0004);
    }

    #[test]
    fn test_color_math_channels_independent() {
        let main = 1 | (2 << 5) | (3 << 10);
        let sub = 4 | (5 << 5) | (6 << 10);
        let expected = 5 | (7 << 5) | (9 << 10);
        assert_eq!(Renderer::color_math(main, sub, false, false), expected);
    }

    // ============================================================
    // deposit / z-order
    // ============================================================

    #[test]
    fn test_deposit_higher_z_wins() {
        let mut r = Renderer::new();
        r.deposit_main(0, 0x0001, Priority::Bg2Low, Layer::Bg2, false);
        r.deposit_main(0, 0x0002, Priority::Bg1Low, Layer::Bg1, false); // higher z
        assert_eq!(r.main_line[0].color, 0x0002);
        assert!(r.main_line[0].layer == Layer::Bg1);
    }

    #[test]
    fn test_deposit_lower_z_ignored() {
        let mut r = Renderer::new();
        r.deposit_main(0, 0x0002, Priority::Bg1Low, Layer::Bg1, false);
        r.deposit_main(0, 0x0001, Priority::Bg2Low, Layer::Bg2, false); // lower z
        assert_eq!(r.main_line[0].color, 0x0002);
        assert!(r.main_line[0].layer == Layer::Bg1);
    }

    // ============================================================
    // Compositing / color math integration (via composite_line)
    // ============================================================

    // Deposit one BG1 main pixel, run color math against the fixed colour, and
    // return the framebuffer pixel at x=0.
    fn composite_bg1_with_fixed(main: u16, coldata_write: u8, cgadsub: u8) -> (u8, u8, u8) {
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        ppu.write(0x2132, coldata_write); // fixed colour
        ppu.write(0x2130, 0x00); // CGWSEL bit1=0 -> fixed colour operand
        ppu.write(0x2131, cgadsub);
        r.deposit_main(0, main, Priority::Bg1Low, Layer::Bg1, false);
        r.composite_line(&ppu, 0);
        fb_pixel(&r, 0)
    }

    #[test]
    fn test_composite_add() {
        // main R=8, fixed R=4, BG1 math + add -> R=12
        let got = composite_bg1_with_fixed(0x0008, 0x24, 0x01);
        assert_eq!(got, Renderer::apply_brightness(0x000C, 15));
    }

    #[test]
    fn test_composite_subtract() {
        // main R=12, fixed R=4, BG1 math + subtract -> R=8
        let got = composite_bg1_with_fixed(0x000C, 0x24, 0x81);
        assert_eq!(got, Renderer::apply_brightness(0x0008, 15));
    }

    #[test]
    fn test_composite_half() {
        // main R=8, fixed R=4, BG1 math + half add -> R=6
        // NOTE: real HW inhibits half against the fixed colour; this asserts
        // current behaviour (half always applied).
        let got = composite_bg1_with_fixed(0x0008, 0x24, 0x41);
        assert_eq!(got, Renderer::apply_brightness(0x0006, 15));
    }

    #[test]
    fn test_composite_math_disabled_when_layer_bit_clear() {
        // CGADSUB=0 -> BG1 not enabled for math -> main colour passes through
        let got = composite_bg1_with_fixed(0x0008, 0x24, 0x00);
        assert_eq!(got, Renderer::apply_brightness(0x0008, 15));
    }

    #[test]
    fn test_composite_add_saturates() {
        // main R=31, fixed R=31, add -> clamps to 31
        let got = composite_bg1_with_fixed(0x001F, 0x3F, 0x01);
        assert_eq!(got, Renderer::apply_brightness(0x001F, 15));
    }

    #[test]
    fn test_composite_clip_to_black() {
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        ppu.write(0x2130, 0xC0); // CGWSEL bits7-6 = 11 -> clip main to black always
        ppu.write(0x2131, 0x00); // no math
        r.deposit_main(0, 0x7FFF, Priority::Bg1Low, Layer::Bg1, false);
        r.composite_line(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0), (0, 0, 0));
    }

    #[test]
    fn test_composite_obj_math_requires_high_palette() {
        let mut ppu = make_ppu();
        ppu.write(0x2132, 0x24); // fixed R=4
        ppu.write(0x2130, 0x00); // fixed operand
        ppu.write(0x2131, 0x10); // CGADSUB: OBJ math enable, add

        // Palette 0-3 sprite (obj_math=false): no math, main passes through.
        let mut r = make_renderer();
        r.deposit_main(0, 0x0008, Priority::Obj2, Layer::Obj, false);
        r.composite_line(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0), Renderer::apply_brightness(0x0008, 15));

        // Palette 4-7 sprite (obj_math=true): math applies -> R=12.
        let mut r = make_renderer();
        r.deposit_main(0, 0x0008, Priority::Obj2, Layer::Obj, true);
        r.composite_line(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0), Renderer::apply_brightness(0x000C, 15));
    }

    #[test]
    fn test_composite_fixed_vs_subscreen_operand() {
        // main R=8 (BG1), sub pixel R=2 (BG2), fixed colour R=4.
        // CGWSEL bit1=1 -> operand is sub (R=2) -> R=10.
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        ppu.write(0x2132, 0x24); // fixed R=4
        ppu.write(0x2130, 0x02); // use subscreen
        ppu.write(0x2131, 0x01); // BG1 math, add
        r.deposit_main(0, 0x0008, Priority::Bg1Low, Layer::Bg1, false);
        r.deposit_sub(0, 0x0002, Priority::Bg2Low, Layer::Bg2, false);
        r.composite_line(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0), Renderer::apply_brightness(0x000A, 15));

        // CGWSEL bit1=0 -> operand is fixed colour (R=4) -> R=12.
        let mut r = make_renderer();
        ppu.write(0x2130, 0x00);
        r.deposit_main(0, 0x0008, Priority::Bg1Low, Layer::Bg1, false);
        r.deposit_sub(0, 0x0002, Priority::Bg2Low, Layer::Bg2, false);
        r.composite_line(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0), Renderer::apply_brightness(0x000C, 15));
    }

    #[test]
    fn test_sub_backdrop_is_fixed_colour() {
        // No layers. main backdrop = CGRAM[0] (R=8), sub backdrop = COLDATA (R=4).
        // Backdrop math enabled, use subscreen -> operand = sub backdrop = COLDATA.
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        set_color(&mut ppu, 0, 0x0008); // main backdrop R=8
        ppu.write(0x2132, 0x24); // COLDATA R=4
        ppu.write(0x2130, 0x02); // use subscreen
        ppu.write(0x2131, 0x20); // CGADSUB backdrop math enable, add
        ppu.write(0x212C, 0x00);
        ppu.write(0x212D, 0x00);
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0), Renderer::apply_brightness(0x000C, 15));
    }

    // ============================================================
    // Layer routing (TM / TS) and gating
    // ============================================================

    #[test]
    fn test_layer_routing_main_only() {
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        setup_bg1_uniform(&mut ppu, 0x1234);
        ppu.write(0x212C, 0x01); // TM: BG1 on main
        ppu.write(0x212D, 0x00);
        r.render_scanline_mode1(&ppu, 0);
        assert!(r.main_line[0].layer == Layer::Bg1);
        assert_eq!(r.main_line[0].color, 0x1234);
        assert!(r.sub_line[0].layer == Layer::Backdrop);
    }

    #[test]
    fn test_layer_routing_sub_only() {
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        setup_bg1_uniform(&mut ppu, 0x1234);
        ppu.write(0x212C, 0x00);
        ppu.write(0x212D, 0x01); // TS: BG1 on sub
        r.render_scanline_mode1(&ppu, 0);
        assert!(r.main_line[0].layer == Layer::Backdrop);
        assert!(r.sub_line[0].layer == Layer::Bg1);
        assert_eq!(r.sub_line[0].color, 0x1234);
    }

    #[test]
    fn test_layer_routing_both() {
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        setup_bg1_uniform(&mut ppu, 0x1234);
        ppu.write(0x212C, 0x01);
        ppu.write(0x212D, 0x01);
        r.render_scanline_mode1(&ppu, 0);
        assert!(r.main_line[0].layer == Layer::Bg1);
        assert!(r.sub_line[0].layer == Layer::Bg1);
    }

    #[test]
    fn test_tm_gating_disabled_layer_not_on_main() {
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        setup_bg1_uniform(&mut ppu, 0x1234);
        ppu.write(0x212C, 0x00); // BG1 disabled on main
        ppu.write(0x212D, 0x00);
        r.render_scanline_mode1(&ppu, 0);
        assert!(r.main_line[0].layer == Layer::Backdrop);
    }

    // ============================================================
    // Priority ordering (full pipeline)
    // ============================================================

    #[test]
    fn test_priority_bg1_over_bg2() {
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        setup_bg1_uniform(&mut ppu, 0x001F); // red
        setup_bg2_uniform(&mut ppu, 0x7C00); // blue
        ppu.write(0x212C, 0x03); // BG1 + BG2 on main
        r.render_scanline(&ppu, 0);
        // BG1 (Z=8) beats BG2 (Z=7)
        assert_eq!(fb_pixel(&r, 0), Renderer::apply_brightness(0x001F, 15));
    }

    #[test]
    fn test_bg3_priority_bit_lifts_above_bg1() {
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        setup_bg1_uniform(&mut ppu, 0x001F); // red, BG1 low prio
        setup_bg3_high_prio(&mut ppu, 0x7C00); // blue, BG3 high prio
        ppu.write(0x212C, 0x05); // BG1 + BG3 on main

        // Without BGMODE bit3: BG1 (Z=8) beats BG3 high (Z=5).
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0), Renderer::apply_brightness(0x001F, 15));

        // With BGMODE bit3: BG3 high (Z=13) beats everything.
        let mut r = make_renderer();
        ppu.write(0x2105, 0x01 | 0x08);
        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0), Renderer::apply_brightness(0x7C00, 15));
    }

    // ============================================================
    // Tilemap sizes (sub-screen selection)
    // ============================================================

    #[test]
    fn test_tilemap_64x32_second_screen() {
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        ppu.write(0x2107, 0x01); // BG1SC: tilemap 0x0000, w64
        let nba = (ppu.regs.bg12nba & 0xF0) | 0x01;
        ppu.write(0x210B, nba);
        ppu.write(0x212C, 0x01);

        // Screen 1 (cols 32-63) at tilemap 0x0400: tile 1 opaque.
        ppu.vram.memory[0x0400] = 0x0001;
        for row in 0..8 {
            ppu.vram.memory[0x1000 + 16 + row] = 0x00FF; // tile 1 CHR (4bpp)
        }
        set_color(&mut ppu, 0x01, 0x001F);

        // Scroll x=256 -> screen x=0 maps to tile column 32 (screen 1).
        ppu.write(0x210D, 0x00);
        ppu.write(0x210D, 0x01);

        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0), Renderer::apply_brightness(0x001F, 15));
    }

    #[test]
    fn test_tilemap_32x64_second_screen() {
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        ppu.write(0x2107, 0x02); // BG1SC: tilemap 0x0000, h64
        let nba = (ppu.regs.bg12nba & 0xF0) | 0x01;
        ppu.write(0x210B, nba);
        ppu.write(0x212C, 0x01);

        // Lower screen (rows 32-63) at tilemap 0x0400: tile 1 opaque.
        ppu.vram.memory[0x0400] = 0x0001;
        for row in 0..8 {
            ppu.vram.memory[0x1000 + 16 + row] = 0x00FF;
        }
        set_color(&mut ppu, 0x01, 0x001F);

        // Scroll y=256 -> screen y=0 maps to tile row 32 (lower screen).
        ppu.write(0x210E, 0x00);
        ppu.write(0x210E, 0x01);

        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0), Renderer::apply_brightness(0x001F, 15));
    }

    #[test]
    fn test_tilemap_64x64_bottom_right_screen() {
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        ppu.write(0x2107, 0x03); // BG1SC: w64 + h64
        let nba = (ppu.regs.bg12nba & 0xF0) | 0x01;
        ppu.write(0x210B, nba);
        ppu.write(0x212C, 0x01);

        // Bottom-right screen (SC3) at offset 0xC00: tile 1 opaque.
        ppu.vram.memory[0x0C00] = 0x0001;
        for row in 0..8 {
            ppu.vram.memory[0x1000 + 16 + row] = 0x00FF;
        }
        set_color(&mut ppu, 0x01, 0x001F);

        // Scroll x=256, y=256 -> screen (0,0) maps to tile (row 32, col 32) = SC3.
        ppu.write(0x210D, 0x00);
        ppu.write(0x210D, 0x01);
        ppu.write(0x210E, 0x00);
        ppu.write(0x210E, 0x01);

        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0), Renderer::apply_brightness(0x001F, 15));
    }

    #[test]
    fn test_tilemap_address_wraps_within_vram() {
        let mut r = make_renderer();
        let mut ppu = make_ppu();
        ppu.write(0x2107, 0x7F); // BG1SC: tilemap 0x7C00, w64 + h64
        let nba = (ppu.regs.bg12nba & 0xF0) | 0x01;
        ppu.write(0x210B, nba); // BG1 CHR at 0x1000
        ppu.write(0x212C, 0x01);

        // SC3 sits at 0x7C00 + 0xC00 = 0x8800, which wraps to 0x0800.
        ppu.vram.memory[0x0800] = 0x0001;
        for row in 0..8 {
            ppu.vram.memory[0x1000 + 16 + row] = 0x00FF;
        }
        set_color(&mut ppu, 0x01, 0x001F);

        // Scroll x=256, y=256 -> screen (0,0) maps to SC3.
        ppu.write(0x210D, 0x00);
        ppu.write(0x210D, 0x01);
        ppu.write(0x210E, 0x00);
        ppu.write(0x210E, 0x01);

        r.render_scanline(&ppu, 0);
        assert_eq!(fb_pixel(&r, 0), Renderer::apply_brightness(0x001F, 15));
    }
}
