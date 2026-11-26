//! Utility functions for the unit tests of the GUI: a save state window
//! with every kind of slot, and kittest harnesses that run the widgets
//! without a window or a GPU.

// Test helpers: left out of the coverage report.
#![cfg(not(tarpaulin_include))]

use std::time::SystemTime;

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_sdl2::egui;
use ppu::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};

use crate::rsnes::RomInfo;
use crate::save_state::slots::MAX_NAME_LENGTH;

use super::command::Command;
use super::state::{GuiState, Notification, PickerContent, PickerEntry, SaveStatePicker};
use super::{Gui, GuiFrameData, widgets};

/// The unreadable slots' reason: longer than the window is wide.
pub const LONG_REASON: &str = "a reason much longer than the window is wide, to check that \
                               it doesn't make the window grow, a reason much longer than \
                               the window is wide, to check that it doesn't make it grow";

/// A save state window on every kind of slot, slot 1 selected:
/// - 1 to 5 saved; 1 is the latest, 2 has the longest name, 3 is locked;
/// - 6 and 7 unreadable, with [`LONG_REASON`];
/// - 8 to 10 empty.
pub fn test_picker(ctx: &egui::Context) -> SaveStatePicker {
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
    SaveStatePicker::new(entries, 0)
}

/// The save state window, filling a game area of `size`, on
/// [`test_picker`] once `prepare` has set it up. The state is the picker,
/// `None` once the window is closed.
pub fn picker_harness(
    size: egui::Vec2,
    prepare: impl FnOnce(&mut SaveStatePicker),
) -> Harness<'static, Option<SaveStatePicker>> {
    // One frame per input event, at 60 frames per second as in R-SNES: two
    // clicks are then close enough in time to make a double click.
    let mut harness = Harness::builder()
        .with_size(size)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            |ui, picker: &mut Option<SaveStatePicker>| {
                let area = ui.ctx().content_rect();
                widgets::save_state_picker(ui.ctx(), area, picker);
            },
            None,
        );
    let mut picker = test_picker(&harness.ctx);
    prepare(&mut picker);
    *harness.state_mut() = Some(picker);
    harness.run();
    harness
}

/// The picker of a harness from [`picker_harness`], which must be open.
pub fn picker<'h>(harness: &'h Harness<'_, Option<SaveStatePicker>>) -> &'h SaveStatePicker {
    harness.state().as_ref().expect("the window is open")
}

/// Clicks the button labeled `label`, then lets egui handle the click.
pub fn click<State>(harness: &mut Harness<'_, State>, label: &str) {
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, label)
        .click();
    harness.run();
}

/// Whether the button labeled `label` is greyed out.
pub fn is_disabled<State>(harness: &Harness<'_, State>, label: &str) -> bool {
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, label)
        .accesskit_node()
        .is_disabled()
}

/// The notification widget in a game area of `size`, showing nothing yet.
/// The state is the notification shown.
pub fn notification_harness(size: egui::Vec2) -> Harness<'static, Option<Notification>> {
    Harness::builder().with_size(size).build_ui_state(
        |ui, shown: &mut Option<Notification>| {
            let area = ui.ctx().content_rect();
            widgets::notification(ui.ctx(), area, shown);
        },
        None,
    )
}

/// The menu bar of [`menu_harness`] and what it was given.
pub struct MenuTest {
    /// The windows open.
    pub state: GuiState,
    /// The commands chosen in the menus.
    pub commands: Vec<Command>,
    /// The ROM running.
    pub rom: RomInfo,
}

/// The menu bar with a ROM running, and the save state window open if
/// `picker_open`.
pub fn menu_harness(picker_open: bool) -> Harness<'static, MenuTest> {
    let test = MenuTest {
        state: GuiState::default(),
        commands: Vec::new(),
        rom: crate::test_utils::make_rsnes().rom_info(),
    };
    let mut harness = Harness::builder().with_size(Gui::START_GAME_AREA).build_ui_state(
        |ui, test: &mut MenuTest| {
            let data = GuiFrameData {
                rom_info: Some(&test.rom),
                paused: false,
                #[cfg(feature = "plugins")]
                plugin_loaded: false,
            };
            widgets::menu_bar(ui, &test.state, &data, &mut test.commands);
        },
        test,
    );
    if picker_open {
        harness.state_mut().state.save_state_picker = Some(test_picker(&harness.ctx));
    }
    harness.run();
    harness
}
