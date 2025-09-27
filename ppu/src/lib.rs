pub mod utils;
pub mod tile;
pub mod ppu;

#[cfg(test)]
#[path = "tests/ppu_cgram.rs"]
mod ppu_cgram;

#[cfg(test)]
#[path = "tests/ppu_vram.rs"]
mod ppu_vram;

#[cfg(test)]
#[path = "tests/ppu_render.rs"]
mod ppu_render;

#[cfg(test)]
#[path = "tests/ppu_tile.rs"]
mod ppu_tile;
