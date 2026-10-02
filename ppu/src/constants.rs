//! Hardware constants: memory sizes, screen size and NTSC timings.

/// VRAM size in bytes.
pub const VRAM_SIZE: usize = 64 * 1024;
/// CGRAM size in bytes.
pub const CGRAM_SIZE: usize = 512;

/// Screen width in pixels.
pub const SCREEN_WIDTH: usize = 256;
/// Screen height in pixels (without overscan).
pub const SCREEN_HEIGHT: usize = 224;

/// Master cycles per scanline (NTSC).
pub const MASTER_CYCLES_PER_SCANLINE: u32 = 1364;
/// Master cycles in the short scanline.
pub const MASTER_CYCLES_SHORT_SCANLINE: u32 = 1360;
/// Dots per scanline.
pub const DOTS_PER_SCANLINE: u16 = 340;
/// Scanlines per frame (NTSC).
pub const SCANLINES_PER_FRAME: u16 = 262;
/// Index of the short scanline.
pub const SHORT_SCANLINE: u16 = 240;

/// Dot where HBlank starts.
pub const HBLANK_START_DOT: u16 = 274;
/// Dot where HDMA starts.
pub const HDMA_START_DOT: u16 = 278;
/// First VBlank line.
pub const VBLANK_START_LINE: u16 = 225;
/// First VBlank line with overscan (SETINI $2133 bit 2).
pub const VBLANK_START_LINE_OVERSCAN: u16 = 240;

/// TIMEUP is set this many cycles after dot 0.0, plus H*4.
pub const IRQ_TRIGGER_OFFSET: u32 = 14;
/// For H=0, 1374 cycles after the *previous* line's dot 0.0.
pub const V_IRQ_TRIGGER_CYCLES: u32 = 1374 - MASTER_CYCLES_PER_SCANLINE;
