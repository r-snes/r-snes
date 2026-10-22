//! Crate implementing the SNES PPU (Picture Processing Unit):
//! registers, video memories (VRAM, CGRAM, OAM),
//! scanline timing and rendering.

/// Palette memory.
pub mod cgram;
/// Hardware constants.
pub mod constants;
/// Sprite attribute memory.
pub mod oam;
/// Main PPU state and timing.
pub mod ppu;
/// PPU registers.
pub mod registers;
/// Scanline rendering.
pub mod rendering;
/// Sprite handling.
pub mod sprites;
/// Video memory.
pub mod vram;
/// Two-step byte latch.
pub mod write_twice;

// Helpers shared by the tests.
#[cfg(test)]
mod test_utils;

// re-export the most important types for easy access
pub use ppu::PPU;
pub use rendering::Renderer;
