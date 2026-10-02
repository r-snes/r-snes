//! Rendering of the PPU output into an RGB888 framebuffer.

/// BG Mode 0 renderer.
pub mod mode_0;
/// BG Mode 1 renderer.
pub mod mode_1;
/// Scanline composition and framebuffer.
pub mod renderer;

// re-export most things so that client code doesn't have to
// use `ppu::rendering::renderer::Renderer`
pub use renderer::{RawFramebuffer, Renderer};
