// --- Memory sizes ---
pub const VRAM_SIZE: usize = 64 * 1024; // 64 KB
pub const CGRAM_SIZE: usize = 256; // 256 color palette
pub const OAM_SIZE: usize = 544; // 512 low table + 32 high table
pub const OAM_MAX_SPRITES: usize = 128;

// --- Tile layout ---
pub const TILE_SIZE: usize = 8;
pub const TILES_X: usize = 32;
pub const TILES_Y: usize = 32;

// --- Screen dimensions ---
pub const WIDTH: usize = TILES_X * TILE_SIZE;
pub const HEIGHT: usize = TILES_Y * TILE_SIZE;
pub const SCALE: usize = 2;
pub const SCREEN_WIDTH: usize = WIDTH * SCALE;
pub const SCREEN_HEIGHT: usize = HEIGHT * SCALE;
