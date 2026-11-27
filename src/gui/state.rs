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

/// One save slot, as the save state picker shows it.
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
pub struct SaveStatePicker {
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

impl SaveStatePicker {
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
    pub save_state_picker: Option<SaveStatePicker>,

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
        self.save_state_picker = None;
        self.notification = None;
        #[cfg(feature = "plugins")]
        {
            self.pending_plugin = None;
        }
        was_open
    }

    pub fn any_open(&self) -> bool {
        let mut open =
            self.show_rom_info || self.error_popup.is_some() || self.save_state_picker.is_some();
        #[cfg(feature = "plugins")]
        {
            open = open || self.pending_plugin.is_some();
        }
        open
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::test_utils::test_picker;

    fn picker() -> SaveStatePicker {
        test_picker(&egui::Context::default())
    }

    #[test]
    fn a_new_picker_selects_at_most_the_last_slot() {
        let ctx = egui::Context::default();
        assert_eq!(
            SaveStatePicker::new(test_picker(&ctx).entries, 4).selected,
            4
        );
        assert_eq!(
            SaveStatePicker::new(test_picker(&ctx).entries, 99).selected,
            9
        );
    }

    #[test]
    #[should_panic(expected = "a picker needs slots")]
    fn a_picker_needs_slots() {
        SaveStatePicker::new(Vec::new(), 0);
    }

    #[test]
    fn selecting_another_slot_cancels_what_was_going_on() {
        let mut picker = picker();
        picker.confirming = Some(Confirmation::Delete);
        picker.new_name = Some("boss".into());

        picker.select(0);
        assert_eq!(picker.confirming, Some(Confirmation::Delete), "same slot");
        assert!(picker.new_name.is_some(), "same slot");

        picker.select(3);
        assert_eq!(picker.selected_entry().slot, 4);
        assert_eq!(picker.confirming, None);
        assert_eq!(picker.new_name, None);

        picker.select(99);
        assert_eq!(picker.selected, 9, "stops at the last slot");
    }

    #[test]
    fn the_selection_moves_by_columns_and_rows_and_stops_at_the_ends() {
        let mut picker = picker();
        let row = SaveStatePicker::COLUMNS as isize;

        picker.move_selection(1);
        assert_eq!(picker.selected, 1);
        picker.move_selection(row);
        assert_eq!(picker.selected, 6);
        picker.move_selection(row);
        assert_eq!(picker.selected, 9, "stops at the last slot");
        picker.move_selection(-row);
        assert_eq!(picker.selected, 4);
        picker.move_selection(-row);
        assert_eq!(picker.selected, 0, "stops at the first slot");
        picker.move_selection(-1);
        assert_eq!(picker.selected, 0);
    }

    #[test]
    fn only_a_saved_slot_is_loaded() {
        let mut picker = picker();

        picker.load_selected();
        assert_eq!(picker.action, Some(SlotAction::Load(1)));

        for unreadable_or_empty in [5, 9] {
            let mut picker = self::picker();
            picker.select(unreadable_or_empty);
            picker.load_selected();
            assert_eq!(picker.action, None, "index {unreadable_or_empty}");
        }
    }

    #[test]
    fn nothing_is_loaded_while_an_action_waits_for_confirmation() {
        let mut picker = picker();
        picker.confirming = Some(Confirmation::Overwrite);

        picker.load_selected();

        assert_eq!(picker.action, None);
    }

    #[test]
    fn an_entry_is_dated_by_its_save_or_its_file() {
        let picker = picker();

        assert!(picker.entries[0].time().is_some(), "saved");
        assert!(picker.entries[5].time().is_some(), "unreadable");
        assert_eq!(picker.entries[9].time(), None, "empty");
    }

    #[test]
    fn the_save_state_window_counts_as_open_and_escape_closes_it() {
        let mut state = GuiState::default();
        assert!(!state.any_open());

        state.save_state_picker = Some(picker());
        assert!(state.any_open());

        assert!(state.close_all());
        assert!(state.save_state_picker.is_none());
        assert!(!state.close_all(), "nothing left to close");
    }

    #[test]
    fn a_notification_is_not_an_open_window() {
        let mut state = GuiState {
            notification: Some(Notification::new("Saved to slot 1".into())),
            ..Default::default()
        };

        assert!(!state.any_open());
        assert!(!state.close_all());
        assert!(state.notification.is_none(), "Escape clears it too");
    }

    #[test]
    fn a_notification_expires() {
        let mut notification = Notification::new("Saved to slot 1".into());
        assert!(!notification.expired());
        assert!(!notification.sized);

        notification.shown_at = Instant::now() - Notification::DURATION;
        assert!(notification.expired());
    }
}
