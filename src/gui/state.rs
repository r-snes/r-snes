use std::error::Error;
use std::time::{Duration, Instant, SystemTime};

use egui_sdl2::egui;

#[cfg(feature = "plugins")]
use plugins::plugin::Plugin;

/// One used save slot, as the slot picker shows it.
pub struct PickerEntry {
    /// Slot number, from 1 to `SLOT_COUNT`.
    pub slot: usize,
    /// When the slot was written.
    pub time: SystemTime,
    /// The saved screen, or why the slot can't be loaded.
    pub screen: Result<egui::TextureHandle, String>,
}

/// The save state picker, open while the user chooses a slot to load.
pub struct SlotPicker {
    /// The used slots, the most recent first.
    pub entries: Vec<PickerEntry>,
    /// Index in `entries` of the entry the keyboard moves.
    pub selected: usize,
    /// The slot the user chose, picked up after the egui pass.
    pub chosen: Option<usize>,
}

impl SlotPicker {
    /// Entries per row.
    pub const COLUMNS: usize = 5;

    pub fn new(entries: Vec<PickerEntry>) -> Self {
        Self {
            entries,
            selected: 0,
            chosen: None,
        }
    }

    /// Moves the selection by `delta` entries (+1 for a column, +`COLUMNS`
    /// for a row), stopping at the first and last entries.
    pub fn move_selection(&mut self, delta: isize) {
        let last = self.entries.len().saturating_sub(1);
        self.selected = self.selected.saturating_add_signed(delta).min(last);
    }

    /// Chooses the selected entry, unless its slot can't be loaded.
    pub fn choose_selected(&mut self) {
        if let Some(entry) = self.entries.get(self.selected)
            && entry.screen.is_ok()
        {
            self.chosen = Some(entry.slot);
        }
    }
}

/// A short message shown over the game for a moment, e.g. "Saved to slot 3".
pub struct Notification {
    pub text: String,
    pub shown_at: Instant,
}

impl Notification {
    /// How long a notification stays on screen.
    pub const DURATION: Duration = Duration::from_millis(2500);

    pub fn new(text: String) -> Self {
        Self {
            text,
            shown_at: Instant::now(),
        }
    }

    pub fn expired(&self) -> bool {
        self.shown_at.elapsed() >= Self::DURATION
    }
}

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

    /// The save state picker. Emulation stands still while it is open.
    pub slot_picker: Option<SlotPicker>,

    /// The message shown over the game, if any. Not an overlay: it neither
    /// pauses the game nor takes Escape.
    pub notification: Option<Notification>,
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
        self.slot_picker = None;
        self.notification = None;
        #[cfg(feature = "plugins")]
        {
            self.pending_plugin = None;
        }
        was_open
    }

    pub fn any_open(&self) -> bool {
        let mut open =
            self.show_rom_info || self.error_popup.is_some() || self.slot_picker.is_some();
        #[cfg(feature = "plugins")]
        {
            open = open || self.pending_plugin.is_some();
        }
        open
    }
}
