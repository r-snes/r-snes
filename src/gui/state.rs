use std::error::Error;
use std::time::{Duration, Instant, SystemTime};

use egui_sdl2::egui;

#[cfg(feature = "plugins")]
use plugins::plugin::Plugin;

use super::SlotAction;

/// What a slot of the picker holds.
pub enum PickerContent {
    /// No save state.
    Empty,
    /// A save state that can be loaded.
    Saved {
        created_at: SystemTime,
        /// Size of the file, in bytes.
        file_size: u64,
        /// The saved screen.
        screen: egui::TextureHandle,
    },
    /// A file that can't be loaded.
    Unreadable {
        modified_at: SystemTime,
        /// Why the file can't be loaded.
        reason: String,
    },
}

/// One save slot, as the slot picker shows it.
pub struct PickerEntry {
    /// Slot number, from 1 to `SLOT_COUNT`.
    pub slot: usize,
    pub content: PickerContent,
    /// The slot's name, empty if none.
    pub name: String,
    pub locked: bool,
    /// Whether this is the most recent save.
    pub latest: bool,
}

impl PickerEntry {
    /// When the slot was written, `None` if it is empty.
    pub fn time(&self) -> Option<SystemTime> {
        match &self.content {
            PickerContent::Empty => None,
            PickerContent::Saved { created_at, .. } => Some(*created_at),
            PickerContent::Unreadable { modified_at, .. } => Some(*modified_at),
        }
    }
}

/// An action of the picker that waits for the user to confirm it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Confirmation {
    /// Save over the selected slot.
    Overwrite,
    /// Delete the selected slot.
    Delete,
}

/// The save state window: every slot of the game, and what the user is
/// doing with the selected one.
pub struct SlotPicker {
    /// Every slot, by number.
    pub entries: Vec<PickerEntry>,
    /// Index in `entries` of the selected slot.
    pub selected: usize,
    /// An action on the selected slot waiting for confirmation.
    pub confirming: Option<Confirmation>,
    /// The name being typed for the selected slot.
    pub new_name: Option<String>,
    /// The action the user chose, picked up after the egui pass.
    pub action: Option<SlotAction>,
}

impl SlotPicker {
    /// Entries per row.
    pub const COLUMNS: usize = 5;

    /// A picker on `entries` (which must not be empty), with `selected`
    /// selected.
    pub fn new(entries: Vec<PickerEntry>, selected: usize) -> Self {
        assert!(!entries.is_empty(), "a picker needs slots");
        let selected = selected.min(entries.len() - 1);
        Self {
            entries,
            selected,
            confirming: None,
            new_name: None,
            action: None,
        }
    }

    pub fn selected_entry(&self) -> &PickerEntry {
        &self.entries[self.selected]
    }

    /// Selects an entry, stopping at the last one. Selecting another slot
    /// cancels a confirmation or a name being typed.
    pub fn select(&mut self, index: usize) {
        let index = index.min(self.entries.len() - 1);
        if index != self.selected {
            self.selected = index;
            self.confirming = None;
            self.new_name = None;
        }
    }

    /// Moves the selection by `delta` entries (+1 for a column, +`COLUMNS`
    /// for a row), stopping at the first and last entries.
    pub fn move_selection(&mut self, delta: isize) {
        self.select(self.selected.saturating_add_signed(delta));
    }

    /// Loads the selected slot, if it holds a loadable state and nothing is
    /// waiting for confirmation.
    pub fn load_selected(&mut self) {
        let entry = self.selected_entry();
        if self.confirming.is_none() && matches!(entry.content, PickerContent::Saved { .. }) {
            self.action = Some(SlotAction::Load(entry.slot));
        }
    }
}

/// A short message shown over the game for a moment, e.g. "Saved to slot 3".
pub struct Notification {
    pub text: String,
    pub shown_at: Instant,
    /// Whether it was laid out once, andatory for widget size recomputing.
    pub sized: bool,
}

impl Notification {
    /// How long a notification stays on screen.
    pub const DURATION: Duration = Duration::from_millis(2500);

    pub fn new(text: String) -> Self {
        Self {
            text,
            shown_at: Instant::now(),
            sized: false,
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
