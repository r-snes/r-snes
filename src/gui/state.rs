use std::error::Error;

#[cfg(feature = "plugins")]
use plugins::plugin::Plugin;

use crate::config::{ConfigLoadError, RSnesConfig};

/// A plugin loaded and awaiting the user's grant/deny decision, plus the
/// transient UI state of its permission window (persisted across frames).
#[cfg(feature = "plugins")]
pub struct PendingPlugin {
    pub plugin: Plugin,
    pub show_none: bool,
}

#[cfg(feature = "plugins")]
impl PendingPlugin {
    pub fn new(plugin: Plugin) -> Self {
        Self {
            plugin,
            show_none: false,
        }
    }
}

/// Which overlay windows are currently open.
///
/// This lives in `Gui` and persists across frames — egui is immediate-mode,
/// so the *decision* to draw a window must be stored somewhere that outlives
/// a single frame. This is that somewhere.
#[derive(Default)]
pub struct GuiState {
    pub show_rom_info: bool,
    pub error_popup: Option<Box<dyn Error>>,

    /// A plugin loaded and awaiting the user's grant/deny decision.
    #[cfg(feature = "plugins")]
    pub pending_plugin: Option<PendingPlugin>,

    /// A plugin the user granted, waiting to be handed back to `gui_loop`
    /// when the idle loop exits.
    #[cfg(feature = "plugins")]
    pub granted_plugin: Option<Plugin>,

    pub config: RSnesConfig,
    pub config_error: Option<ConfigLoadError>,
    pub show_config_error: bool,
}

impl GuiState {
    /// Closes every open overlay. Bound to Escape.
    /// Returns true if anything was actually closed, so the caller can
    /// distinguish "Escape closed a window" from "Escape should close the ROM".
    ///
    /// Dismissing a pending permission prompt this way counts as a denial.
    pub fn close_all(&mut self) -> bool {
        let was_open = self.any_open();
        self.show_rom_info = false;
        self.error_popup = None;
        #[cfg(feature = "plugins")]
        {
            self.pending_plugin = None;
        }
        was_open
    }

    /// Creates a new [`GuiState`] with the result of loading a config file
    pub fn new(config_load: Result<RSnesConfig, ConfigLoadError>) -> Self {
        let mut self_ = Self::default();
        self_.apply_config_load_result(config_load);

        self_
    }

    /// Fully reload the configuration file from the filesystem, and
    /// set the GUI state to match the result of the load
    pub fn reload_config(&mut self) {
        use crate::config::{read_config, config_path};
        self.apply_config_load_result(read_config(config_path()));
    }

    fn apply_config_load_result(&mut self, config_load: Result<RSnesConfig, ConfigLoadError>) {
        let (config, config_error) = match config_load {
            Ok(c) => (c, None),
            Err(e) => (Default::default(), Some(e)),
        };
        let show_config_error = config_error.is_some();

        self.config = config;
        self.config_error = config_error;
        self.show_config_error = show_config_error;
    }

    pub fn any_open(&self) -> bool {
        let mut open = self.show_rom_info || self.error_popup.is_some();
        #[cfg(feature = "plugins")]
        {
            open = open || self.pending_plugin.is_some();
        }
        open
    }
}
