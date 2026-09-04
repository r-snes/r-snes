pub const VRAM_SIZE: usize = 64 * 1024; // 64 KB
pub const CGRAM_SIZE: usize = 512; // 512 octets

pub const SCREEN_WIDTH: usize = 256;
pub const SCREEN_HEIGHT: usize = 224;

pub const MASTER_CYCLES_PER_SCANLINE: u32 = 1364; // NTSC
pub const MASTER_CYCLES_SHORT_SCANLINE: u32 = 1360;
pub const DOTS_PER_SCANLINE: u16 = 340;
pub const SCANLINES_PER_FRAME: u16 = 262; // NTSC

pub const HBLANK_START_DOT: u16 = 274;
pub const VBLANK_START_LINE: u16 = 225;
pub const VBLANK_START_LINE_OVERSCAN: u16 = 240; // SETINI ($2133) bit 2

pub const HDMA_START_DOT: u16 = 278;
