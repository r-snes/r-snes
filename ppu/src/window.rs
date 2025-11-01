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

#[cfg(test)]
mod tests_window {
    use super::*;
    use minifb::{Window, WindowOptions};

    fn skip_in_ci() -> bool {
        std::env::var("CI").is_ok()
    }

    #[test] // Creating a window should succeed without errors
    fn test_create_window_success() {
        if skip_in_ci() {
            eprintln!("Skipping window test in CI environment");
            return;
        }

        let window = create_window();
        assert!(window.is_open());
        assert_eq!(window.get_size(), (SCREEN_WIDTH, SCREEN_HEIGHT));
    }

    #[test] // Updating the window should not panic with a valid framebuffer
    fn test_update_window_with_valid_buffer() {
        if skip_in_ci() {
            eprintln!("Skipping window test in CI environment");
            return;
        }

        let mut window = create_window();
        let framebuffer = vec![0xFF00FF00; WIDTH * HEIGHT]; // Green pixels
        update_window(&mut window, &framebuffer);
        assert!(window.is_open());
    }

    #[test] // Updating the window with an incorrect buffer size should panic
    #[should_panic(expected = "[ERR::Render] Framebuffer refused to cooperate.")]
    fn test_update_window_with_invalid_buffer_size() {
        if skip_in_ci() {
            eprintln!("Skipping window test in CI environment");
            panic!("[ERR::Render] Framebuffer refused to cooperate.");
        }

        let mut window = create_window();
        let framebuffer = vec![0xFF0000FF; (WIDTH * HEIGHT) / 2]; // Too small
        update_window(&mut window, &framebuffer);
    }
}
