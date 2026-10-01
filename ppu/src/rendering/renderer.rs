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

/// Z-order of a pixel on the priority scale. The discriminant is the actual
/// z value; `value()` exposes it for the `>=` comparison in `deposit`.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Priority {
    Backdrop = 0,
    Bg4Low = 1,
    Bg3Low = 2,
    Obj0 = 3,
    Bg4High = 4,
    Bg3High = 5,
    Obj1 = 6,
    Bg2Low = 7,
    Bg1Low = 8,
    Obj2 = 9,
    Bg2High = 10,
    Bg1High = 11,
    Obj3 = 12,
    Bg3Prio = 13, // mode 1, BGMODE bit3: BG3 high-prio above all
}

impl Priority {
    pub fn value(self) -> u8 {
        self as u8
    }
}

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
        z: Priority::Backdrop as u8,
        layer: Layer::Backdrop,
        obj_math: false,
    };
}

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
    pub z_low: Priority,
    pub z_high: Priority,
    pub layer: Layer,
    pub to_main: bool, // enabled on main screen (TM)
    pub to_sub: bool,  // enabled on sub screen (TS)
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
            let prio = if priority { p.z_high } else { p.z_low };

            let pixel = LinePixel {
                color,
                z: prio.value(),
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
                z: prio.value(),
                layer,
                obj_math,
            },
        );
    }

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
                z: prio.value(),
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
