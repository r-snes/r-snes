//! Module for all assets loaded at compile time from our
//! assets folder with [`include_str`] or [`include_bytes`]

/// The default R-SNES configuration file
pub const DEFAULT_CONFIG_FILE: &'static str = include_str!("../assets/r-snes_config.toml");
