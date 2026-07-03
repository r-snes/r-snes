pub mod renderer;
pub mod mode_1;
pub mod mode_0;

// re-export most things so that client code doesn't have to
// use `ppu::rendering::renderer::Renderer`
pub use renderer::{RawFramebuffer, Renderer};
