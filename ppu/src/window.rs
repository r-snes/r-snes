use minifb::{Window, WindowOptions};
use crate::constants::*;

/// Creates a new window for the SNES emulator display
/// Returns a `Window` instance from `minifb`
pub fn create_window() -> Window {
    Window::new(
        "rsnes ppu",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions {
            resize: false,
            ..WindowOptions::default()
        },
    )
    .expect("[ERR::WindowInit] Unable to create display context.")
}

/// Updates the given window with the provided framebuffer
/// `framebuffer` is expected to be a vector of ARGB pixels of size WIDTH * HEIGHT
pub fn update_window(window: &mut Window, framebuffer: &Vec<u32>) {
    window
        .update_with_buffer(framebuffer, WIDTH, HEIGHT)
        .expect("[ERR::Render] Framebuffer refused to cooperate.");
}
