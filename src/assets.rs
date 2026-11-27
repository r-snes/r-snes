//! Module for all assets loaded at compile time from our
//! assets folder with [`include_str`] or [`include_bytes`]

/// The default PPU framebuffer rendered when no ROM is loaded
pub const DEFAULT_FRAMEBUFFER: &ppu::rendering::RawFramebuffer =
    include_bytes!("../assets/logo_framebuffer.raw");

/// The default R-SNES configuration file
pub const DEFAULT_CONFIG_FILE: &str = include_str!("../assets/r-snes_config.toml");
