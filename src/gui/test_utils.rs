//! Utility functions for the unit tests of the GUI: a save state window
//! with every kind of slot.

// Test helpers: left out of the coverage report.
#![cfg(not(tarpaulin_include))]

use std::time::SystemTime;

use egui_sdl2::egui;
use ppu::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};

use crate::save_state::slots::MAX_NAME_LENGTH;

use super::state::{PickerContent, PickerEntry, SlotPicker};

/// The unreadable slots' reason: longer than the window is wide.
pub const LONG_REASON: &str = "a reason much longer than the window is wide, to check that \
                               it doesn't make the window grow, a reason much longer than \
                               the window is wide, to check that it doesn't make it grow";

/// A save state window on every kind of slot, slot 1 selected:
/// - 1 to 5 saved; 1 is the latest, 2 has the longest name, 3 is locked;
/// - 6 and 7 unreadable, with [`LONG_REASON`];
/// - 8 to 10 empty.
pub fn test_picker(ctx: &egui::Context) -> SlotPicker {
    let screen = ctx.load_texture(
        "screen",
        egui::ColorImage::from_rgb(
            [SCREEN_WIDTH, SCREEN_HEIGHT],
            &vec![0; SCREEN_WIDTH * SCREEN_HEIGHT * 3],
        ),
        egui::TextureOptions::LINEAR,
    );
    let entries = (1..=10)
        .map(|slot| PickerEntry {
            slot,
            content: match slot {
                1..=5 => PickerContent::Saved {
                    created_at: SystemTime::now(),
                    file_size: 588_800,
                    screen: screen.clone(),
                },
                6 | 7 => PickerContent::Unreadable {
                    modified_at: SystemTime::now(),
                    reason: LONG_REASON.to_owned(),
                },
                _ => PickerContent::Empty,
            },
            name: if slot == 2 {
                "W".repeat(MAX_NAME_LENGTH)
            } else {
                String::new()
            },
            locked: slot == 3,
            latest: slot == 1,
        })
        .collect();
    SlotPicker::new(entries, 0)
}
