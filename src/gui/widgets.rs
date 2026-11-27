pub mod config;

use std::time::SystemTime;
use std::{error::Error, io::ErrorKind::NotFound};

use bus::cartridge::header::RomHeader;
use egui_sdl2::egui::{self, RichText};
use ppu::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};

use crate::gui::widgets::config::{ConfigUi, ConfigUiAction};
use crate::save_state::slots::MAX_NAME_LENGTH;
use crate::{config::ConfigLoadError, rsnes::RomInfo};

use super::command::Command;
use super::state::{
    Confirmation, GuiState, Notification, PickerContent, PickerEntry, SaveStatePicker,
};
use super::{Gui, GuiFrameData, SlotAction};

#[cfg(feature = "plugins")]
use plugins::plugin::{Plugin, gui::PermOutcome};

#[cfg(feature = "plugins")]
use super::state::PendingPlugin;

/// The menu bar along the top of the window.
///
/// Entries only push the [`Command`] they stand for into `commands`; the
/// caller runs them after the egui pass, since some open blocking file dialogs.
/// Each entry shows its keyboard shortcut and is greyed out when unavailable.
///
/// Returns the bar's bottom edge in points.
pub fn menu_bar(
    ui: &mut egui::Ui,
    state: &GuiState,
    data: &GuiFrameData,
    commands: &mut Vec<Command>,
) -> f32 {
    let mut item = |ui: &mut egui::Ui, cmd: Command, text: &str, selected: bool| {
        let button = egui::Button::new(text)
            .selected(selected)
            .shortcut_text(cmd.shortcut().label());
        if ui.add_enabled(cmd.is_enabled(data), button).clicked() {
            commands.push(cmd);
        }
    };

    let panel = egui::Panel::top("menu_bar")
        .show_separator_line(false)
        .show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    item(ui, Command::OpenRom, "Open ROM...", false);
                    item(ui, Command::CloseRom, "Close ROM", false);
                    #[cfg(feature = "plugins")]
                    {
                        ui.separator();
                        item(ui, Command::LoadPlugin, "Load plugin...", false);
                    }
                    ui.separator();
                    item(ui, Command::Quit, "Quit", false);
                });

                ui.menu_button("Emulation", |ui| {
                    let pause = if data.paused { "Resume" } else { "Pause" };
                    item(ui, Command::TogglePause, pause, false);
                    item(ui, Command::Reset, "Reset", false);
                    #[cfg(feature = "plugins")]
                    item(ui, Command::RunPluginDefault, "Run plugin action", false);
                    ui.separator();
                    item(ui, Command::QuickSaveState, "Save state", false);
                    item(ui, Command::LoadLatestState, "Load latest state", false);
                    item(ui, Command::UndoLoadState, "Undo load", false);
                    item(
                        ui,
                        Command::ChooseSaveState,
                        "Save states...",
                        state.save_state_picker.is_some(),
                    );
                });

                ui.menu_button("View", |ui| {
                    item(
                        ui,
                        Command::ToggleRomInfo,
                        "ROM information",
                        state.show_rom_info,
                    );
                });

                ui.menu_button("Config", |ui| {
                    item(
                        ui,
                        Command::ReloadConfig,
                        "Reload configuration file",
                        false,
                    );
                    let verb = if state.config_ui.is_some() {
                        "Hide"
                    } else {
                        "Show"
                    };
                    let info = if state.config_error.is_some() {
                        "errors"
                    } else {
                        "info"
                    };
                    item(
                        ui,
                        Command::ToggleConfigErrors,
                        &format!("{verb} config {info}"),
                        false,
                    );
                });

                if data.paused {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Paused").weak());
                    });
                }
            });
        });

    panel.response.rect.bottom()
}

/// Decodes a SNES header size exponent into KiB. `0` means "none".
fn decode_size_kb(exponent: u8) -> Option<usize> {
    if exponent == 0 {
        None
    } else {
        Some(1usize << exponent)
    }
}

/// ROM information overlay. Toggled with F1.
pub fn rom_info(ctx: &egui::Context, area: egui::Rect, open: &mut bool, info: Option<&RomInfo>) {
    egui::Window::new("ROM Information")
        .constrain_to(area)
        .open(open)
        .auto_sized()
        .collapsible(false)
        .vscroll(true)
        .hscroll(true)
        .show(ctx, |ui| {
            let Some(info) = info else {
                ui.centered_and_justified(|ui| {
                    ui.label("No ROM loaded.");
                });
                return;
            };

            let RomInfo {
                path,
                file_size_kb,
                header: _,
            } = &info;

            let RomHeader {
                bytes: _,
                title,
                rom_speed,
                mapping_mode,
                hardware,
                rom_size,
                ram_size,
                country,
                video_standard,
                developer_id,
                rom_version,
                checksum_complement,
                checksum,
            } = &info.header;

            // --- Primary info: what you actually want at a glance ---
            egui::Grid::new("rom_info_primary")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Title");
                    // Header titles are space-padded to 21 bytes.
                    ui.label(title.trim());
                    ui.end_row();

                    ui.label("Mapping mode");
                    ui.label(mapping_mode.to_string());
                    ui.end_row();

                    ui.label("ROM speed");
                    ui.label(rom_speed.to_string());
                    ui.end_row();

                    ui.label("File size");
                    ui.label(format!("{file_size_kb} KiB"));
                    ui.end_row();

                    ui.label("Region");
                    ui.label(format!("{country} ({video_standard})"));
                    ui.end_row();
                });

            ui.add_space(4.0);

            // --- Secondary info: folded away by default ---
            egui::CollapsingHeader::new("Cartridge hardware")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("rom_info_hardware")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Layout");
                            ui.label(hardware.layout.to_string());
                            ui.end_row();

                            ui.label("Coprocessor");
                            ui.label(if hardware.has_coprocessor() {
                                match &hardware.coprocessor {
                                    Some(c) => c.to_string(),
                                    None => "Unknown".to_owned(),
                                }
                            } else {
                                "None".to_owned()
                            });
                            ui.end_row();

                            ui.label("ROM size (header)");
                            ui.label(match decode_size_kb(*rom_size) {
                                Some(kb) => format!("{kb} KiB (exp. {rom_size})"),
                                None => format!("none (exp. {rom_size})",),
                            });
                            ui.end_row();

                            ui.label("SRAM size (header)");
                            ui.label(match decode_size_kb(*ram_size) {
                                Some(kb) => format!("{kb} KiB (exp. {ram_size})"),
                                None => "None".to_owned(),
                            });
                            ui.end_row();
                        });
                });

            egui::CollapsingHeader::new("Publishing")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("rom_info_publishing")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Developer ID");
                            ui.label(format!("${developer_id:02X}"));
                            ui.end_row();

                            ui.label("ROM version");
                            ui.label(format!("1.{rom_version}"));
                            ui.end_row();

                            ui.label("Checksum");
                            ui.label(format!("${checksum:04X}"));
                            ui.end_row();

                            ui.label("Complement");
                            // The two should XOR to 0xFFFF on a valid cartridge.
                            let valid = checksum ^ checksum_complement == 0xFFFF;
                            ui.label(format!(
                                "${checksum_complement:04X} {}",
                                if valid { "[OK]" } else { "[BAD]" }
                            ));
                            ui.end_row();
                        });
                });

            egui::CollapsingHeader::new("File")
                .default_open(false)
                .show(ui, |ui| {
                    // Paths get long; wrap rather than stretching the window.
                    ui.add(egui::Label::new(path.display().to_string()).wrap());
                });
        });
}

/// Renders a window showing any runtime error
pub fn error_box(ctx: &egui::Context, area: egui::Rect, error: &mut Option<Box<dyn Error>>) {
    let mut open = error.is_some();
    let window = egui::Window::new("Error")
        .constrain_to(area)
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .default_width(250.0)
        .default_height(100.0)
        .vscroll(true);

    window.show(ctx, |ui| {
        let Some(e) = error else {
            ui.centered_and_justified(|ui| {
                ui.label("No error");
                ui.label("how did you even get this window to open?");
            });
            return;
        };
        ui.label(RichText::new("Encountered error:").heading());
        ui.indent((), |ui| {
            show_error(ui, e.as_ref());
        });
    });

    if !open {
        *error = None;
    }
}

/// How long ago `time` was, in words (e.g. "5 minutes ago").
fn time_ago(now: SystemTime, time: SystemTime) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;

    // A time in the future (the clock went back) counts as now.
    let seconds = now.duration_since(time).map_or(0, |elapsed| elapsed.as_secs());
    let (count, unit) = match seconds {
        0..MINUTE => return "just now".to_owned(),
        MINUTE..HOUR => (seconds / MINUTE, "minute"),
        HOUR..DAY => (seconds / HOUR, "hour"),
        _ => (seconds / DAY, "day"),
    };
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {unit}{plural} ago")
}

/// The date and time of `time` on the local clock, e.g. "2026-11-19 14:32:07".
fn local_date(time: SystemTime) -> String {
    jiff::Zoned::try_from(time).map_or_else(
        |_| "unknown date".to_owned(),
        |date| date.strftime("%Y-%m-%d %H:%M:%S").to_string(),
    )
}

/// A file size in words, in binary units, e.g. "575 KiB".
fn file_size_text(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    match bytes {
        0..KIB => format!("{bytes} bytes"),
        KIB..MIB => format!("{} KiB", (bytes + KIB / 2) / KIB),
        _ => format!("{:.1} MiB", bytes as f64 / MIB as f64),
    }
}

// Sizes of the save state window in points, with egui's default style and a
// little to spare, used to size the thumbnails so that the window fits the
// game area.
/// The window's margins and border, left and right.
const PICKER_SIDE_SPACE: f32 = 24.0;
/// The title bar and the window's margins, top and bottom.
const PICKER_TOP_SPACE: f32 = 48.0;
/// The details of the selected slot and its buttons, under the grid.
const PICKER_DETAILS_HEIGHT: f32 = 70.0;
/// The two lines under a thumbnail.
const PICKER_CAPTION_HEIGHT: f32 = 40.0;
/// Space between two cells of the grid.
const PICKER_GAP: f32 = 8.0;
/// Width taken by the scroll bar, when the window is too low for the grid.
const PICKER_SCROLL_BAR_SPACE: f32 = 20.0;
/// Thumbnail width under which the window scrolls rather than shrinking them.
const PICKER_MIN_THUMBNAIL_WIDTH: f32 = 32.0;
/// Share of a large game area the window may take, so that some of the game
/// stays visible around it.
const PICKER_AREA_SHARE: f32 = 0.8;
/// Up to this size, the window may take the whole game area: the size of
/// the game area when R-SNES opens.
const PICKER_FULL_AREA: egui::Vec2 = Gui::START_GAME_AREA;

/// Id of the save state window.
const PICKER_WINDOW_ID: &str = "save_state_window";

/// Id of the field where a slot name is typed.
const NAME_FIELD_ID: &str = "slot_name_field";

/// Size of a thumbnail, so that a grid of `rows` rows fits `area`, and
/// whether the window must scroll. The thumbnails grow with the game area;
/// past the size of R-SNES's opening window, the window only takes
/// [`PICKER_AREA_SHARE`] of it. In a game area too low for even small
/// thumbnails, the width alone counts and the window scrolls.
fn thumbnail_size(ctx: &egui::Context, area: egui::Rect, rows: usize) -> (egui::Vec2, bool) {
    let padding = ctx.global_style().spacing.button_padding * 2.0;
    let (columns, rows) = (SaveStatePicker::COLUMNS as f32, rows as f32);
    let aspect = SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32;
    let room = (area.size() * PICKER_AREA_SHARE).max(area.size().min(PICKER_FULL_AREA));

    let by_width =
        (room.x - PICKER_SIDE_SPACE - PICKER_GAP * (columns - 1.0)) / columns - padding.x;
    let free_height = room.y - PICKER_TOP_SPACE - PICKER_DETAILS_HEIGHT;
    let by_height =
        ((free_height - PICKER_GAP * (rows - 1.0)) / rows - PICKER_CAPTION_HEIGHT - padding.y)
            * aspect;

    let scrolls = by_height < PICKER_MIN_THUMBNAIL_WIDTH;
    let width = if scrolls {
        // The scroll bar takes some of the width.
        (by_width - PICKER_SCROLL_BAR_SPACE / columns).min(PICKER_MIN_THUMBNAIL_WIDTH)
    } else {
        by_width.min(by_height)
    };
    let width = width.max(16.0);
    (egui::vec2(width, width / aspect), scrolls)
}

/// The save state picker: the used slots, the most recent first, each with
/// its saved screen, number and age. Selection of the slot is possible with
/// the mouse or the keyboard arrows and Enter.
pub fn save_state_picker(
    ctx: &egui::Context,
    area: egui::Rect,
    picker: &mut Option<SaveStatePicker>,
) {
    let Some(picker_state) = picker else {
        return;
    };

    let rows = picker_state.entries.len().div_ceil(SaveStatePicker::COLUMNS);
    let (thumbnail, scrolls) = thumbnail_size(ctx, area, rows);
    let cell_width = thumbnail.x + ctx.global_style().spacing.button_padding.x * 2.0;
    let columns = SaveStatePicker::COLUMNS as f32;
    let grid_width = columns * cell_width + (columns - 1.0) * PICKER_GAP;

    let now = SystemTime::now();
    // egui sets it to false when [x] is clicked.
    let mut open = true;

    egui::Window::new("Save states")
        .id(egui::Id::new(PICKER_WINDOW_ID))
        .constrain_to(area)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(area.center())
        .open(&mut open)
        .collapsible(false)
        .auto_sized()
        .show(ctx, |ui| {
            let mut contents = |ui: &mut egui::Ui| {
                // The details below are cut to the grid's width.
                ui.set_max_width(grid_width);

                egui::Grid::new("save_state_picker")
                    .num_columns(SaveStatePicker::COLUMNS)
                    .spacing([PICKER_GAP, PICKER_GAP])
                    .show(ui, |ui| {
                        for index in 0..picker_state.entries.len() {
                            let entry = &picker_state.entries[index];
                            let selected = index == picker_state.selected;
                            let response = slot_cell(ui, entry, selected, thumbnail, now);
                            if response.clicked() {
                                picker_state.select(index);
                            }
                            if response.double_clicked() {
                                picker_state.load_selected();
                            }

                            if (index + 1) % SaveStatePicker::COLUMNS == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.separator();
                slot_details(ui, picker_state, now);
            };
            // Only when needed: a scroll area takes the height the window
            // remembers, and egui never lets that height grow back after
            // the game area was small.
            if scrolls {
                egui::ScrollArea::vertical().show(ui, contents);
            } else {
                contents(ui);
            }
        });

    if !open {
        *picker = None;
    }
}

/// One slot of the grid: its thumbnail, as a button, then its number and
/// its name (or age). Returns the button's response.
fn slot_cell(
    ui: &mut egui::Ui,
    entry: &PickerEntry,
    selected: bool,
    thumbnail: egui::Vec2,
    now: SystemTime,
) -> egui::Response {
    ui.vertical(|ui| {
        let button_size = thumbnail + ui.spacing().button_padding * 2.0;
        ui.set_width(button_size.x);

        let response = match &entry.content {
            PickerContent::Saved { screen, .. } => {
                let image = egui::Image::from_texture(screen).fit_to_exact_size(thumbnail);
                ui.add(egui::Button::image(image).selected(selected))
            }
            PickerContent::Empty => ui.add(
                egui::Button::new(RichText::new("Empty").weak())
                    .min_size(button_size)
                    .selected(selected),
            ),
            PickerContent::Unreadable { reason, .. } => ui
                .add(
                    egui::Button::new(RichText::new("Unreadable").weak())
                        .min_size(button_size)
                        .selected(selected),
                )
                .on_hover_text(reason),
        };
        // A thumbnail has no text, and the badges are only painted: name the
        // button for screen readers (and for the tests, which find widgets
        // the same way), e.g. "Slot 1, latest".
        response.widget_info(|| {
            let mut label = format!("Slot {}", entry.slot);
            if entry.latest {
                label.push_str(", latest");
            }
            if entry.locked {
                label.push_str(", locked");
            }
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label)
        });
        if entry.latest {
            badge(ui, response.rect, egui::Align2::LEFT_TOP, "Latest");
        }
        if entry.locked {
            badge(ui, response.rect, egui::Align2::RIGHT_TOP, "Locked");
        }

        ui.label(format!("Slot {}", entry.slot));
        let caption = match entry.time() {
            _ if !entry.name.is_empty() => entry.name.clone(),
            Some(time) => time_ago(now, time),
            None => String::new(),
        };
        ui.add(egui::Label::new(RichText::new(caption).weak().small()).truncate());
        response
    })
    .inner
}

/// A small label drawn over a corner of `rect`.
fn badge(ui: &egui::Ui, rect: egui::Rect, corner: egui::Align2, text: &str) {
    let painter = ui.painter();
    let color = ui.visuals().strong_text_color();
    let galley = painter.layout_no_wrap(text.to_owned(), egui::FontId::proportional(10.0), color);
    let text_rect = corner.align_size_within_rect(galley.size(), rect.shrink(6.0));
    painter.rect_filled(text_rect.expand(2.0), 3.0, ui.visuals().window_fill);
    painter.galley(text_rect.min, galley, color);
}

/// The details of the selected slot, then its actions. An action that needs
/// confirming, or a name being typed, takes the place of the action buttons.
fn slot_details(ui: &mut egui::Ui, picker: &mut SaveStatePicker, now: SystemTime) {
    const UNLOCK_FIRST: &str = "The slot is locked. Unlock it first.";

    let entry = picker.selected_entry();
    let slot = entry.slot;
    let used = !matches!(entry.content, PickerContent::Empty);
    let loadable = matches!(entry.content, PickerContent::Saved { .. });
    let locked = entry.locked;
    let name = entry.name.clone();

    let title = if name.is_empty() {
        format!("Slot {slot}")
    } else {
        format!("Slot {slot}: {name}")
    };
    ui.add(egui::Label::new(RichText::new(title).strong()).truncate());

    let info = match &entry.content {
        PickerContent::Empty => "Empty".to_owned(),
        PickerContent::Saved {
            created_at,
            file_size,
            ..
        } => format!(
            "Saved {} ({}), {}",
            local_date(*created_at),
            time_ago(now, *created_at),
            file_size_text(*file_size)
        ),
        PickerContent::Unreadable {
            modified_at,
            reason,
        } => format!(
            "Unreadable, written {} ({}): {reason}",
            local_date(*modified_at),
            time_ago(now, *modified_at)
        ),
    };
    ui.add(egui::Label::new(&info).truncate());

    let name_field = egui::Id::new(NAME_FIELD_ID);
    if let Some(new_name) = &mut picker.new_name {
        let (mut done, mut cancel) = (false, false);
        ui.horizontal(|ui| {
            // Leaves room for the two buttons.
            let field_width = (ui.available_width() - 100.0).clamp(40.0, 160.0);
            // Not drawn last frame: it just appeared, and takes the keyboard.
            // Focusing it from the Rename button would point at a widget
            // that doesn't exist yet.
            let just_shown = ui.ctx().read_response(name_field).is_none();
            let field = ui.add(
                egui::TextEdit::singleline(new_name)
                    .id(name_field)
                    .char_limit(MAX_NAME_LENGTH)
                    .hint_text("Slot name")
                    // No taller than the buttons it replaces.
                    .margin(egui::vec2(4.0, 1.0))
                    .desired_width(field_width),
            );
            if just_shown {
                field.request_focus();
            }
            done = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            done |= ui.button("OK").clicked();
            cancel = ui.button("Cancel").clicked();
        });
        if done {
            let new_name = picker.new_name.take().unwrap_or_default();
            picker.action = Some(SlotAction::Rename(slot, new_name));
        } else if cancel {
            picker.new_name = None;
        }
        return;
    }

    if let Some(confirmation) = picker.confirming {
        let (question, verb, action) = match confirmation {
            Confirmation::Overwrite => (
                format!("Save over slot {slot}?"),
                "Save over",
                SlotAction::Save(slot),
            ),
            Confirmation::Delete => (
                format!("Delete slot {slot}?"),
                "Delete",
                SlotAction::Delete(slot),
            ),
        };
        ui.horizontal(|ui| {
            ui.label(question);
            if ui.button(verb).clicked() {
                picker.action = Some(action);
                picker.confirming = None;
            }
            if ui.button("Cancel").clicked() {
                picker.confirming = None;
            }
        });
        return;
    }

    ui.horizontal_wrapped(|ui| {
        if ui.add_enabled(loadable, egui::Button::new("Load")).clicked() {
            picker.action = Some(SlotAction::Load(slot));
        }

        let save = ui
            .add_enabled(!locked, egui::Button::new("Save here"))
            .on_disabled_hover_text(UNLOCK_FIRST);
        if save.clicked() {
            if used {
                picker.confirming = Some(Confirmation::Overwrite);
            } else {
                picker.action = Some(SlotAction::Save(slot));
            }
        }

        if ui.add_enabled(used, egui::Button::new("Rename")).clicked() {
            picker.new_name = Some(name);
        }

        let mut delete = ui.add_enabled(used && !locked, egui::Button::new("Delete"));
        if locked {
            delete = delete.on_disabled_hover_text(UNLOCK_FIRST);
        }
        if delete.clicked() {
            picker.confirming = Some(Confirmation::Delete);
        }

        let lock = if locked { "Unlock" } else { "Lock" };
        let lock = ui
            .add_enabled(used, egui::Button::new(lock))
            .on_hover_text("A locked slot is never saved over nor deleted.");
        if lock.clicked() {
            picker.action = Some(SlotAction::SetLocked(slot, !locked));
        }

        let open_folder = "Open folder";
        let open_folder_width = egui::WidgetText::from(open_folder)
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Button,
            )
            .size()
            .x
            + ui.spacing().button_padding.x * 2.0;
        // In a wrapping row, `available_width` is the whole row's width.
        let spare = ui.max_rect().right() - ui.cursor().left() - open_folder_width;
        if spare > 0.0 {
            ui.add_space(spare);
        }
        if ui.button(open_folder).clicked() {
            picker.action = Some(SlotAction::OpenFolder);
        }
    });
}

/// Width of a notification at most: longer texts wrap onto more lines.
const NOTIFICATION_MAX_WIDTH: f32 = 480.0;
/// Space between a notification and the edges of the game area.
const NOTIFICATION_MARGIN: f32 = 8.0;

/// The notification at the bottom-left of the game, until it expires.
pub fn notification(
    ctx: &egui::Context,
    area: egui::Rect,
    notification: &mut Option<Notification>,
) {
    let Some(shown) = notification else {
        return;
    };
    if shown.expired() {
        *notification = None;
        return;
    }

    // The area keeps its size from frame to frame, and the text wraps at
    // it: a new notification needs a sizing pass, where it may take up to
    // its default width.
    let sizing_pass = !shown.sized;
    shown.sized = true;

    egui::Area::new(egui::Id::new("notification"))
        .order(egui::Order::Foreground)
        .interactable(false)
        .constrain_to(area)
        .sizing_pass(sizing_pass)
        .default_width(NOTIFICATION_MAX_WIDTH.min(area.width() - NOTIFICATION_MARGIN * 2.0))
        .pivot(egui::Align2::LEFT_BOTTOM)
        .fixed_pos(area.left_bottom() + egui::vec2(NOTIFICATION_MARGIN, -NOTIFICATION_MARGIN))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(&shown.text);
            });
        });
}

pub fn config_error_box(
    ctx: &egui::Context,
    area: egui::Rect,
    config_ui: Option<&mut ConfigUi>,
    config_err: &Option<ConfigLoadError>,
) {
    let Some(config_ui) = config_ui else {
        return;
    };
    let mut action_button = |ui: &mut egui::Ui, msg, action| {
        if ui.button(msg).clicked() {
            config_ui.action = Some(action);
        }
    };

    let window = egui::Window::new("Config file errors")
        .constrain_to(area)
        .resizable(true)
        .collapsible(false)
        .vscroll(true);

    match config_err {
        None => {
            let mut open = true;
            window
                .default_width(200.0)
                .default_height(40.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Config file OK!");
                    ui.horizontal(|ui| {
                        action_button(ui, "Open config file", ConfigUiAction::OpenConfigFile);
                        action_button(ui, "Reload config", ConfigUiAction::Reload);
                    });
                });
            if !open {
                config_ui.action = Some(ConfigUiAction::Close)
            }
        }
        Some(error) => {
            window
                .default_width(400.0)
                .default_height(300.0)
                .show(ctx, |ui| {
                    ui.label("Error loading the R-SNES configuration file");
                    match error {
                        ConfigLoadError::NoConfigPath => {
                            ui.label("We don't even know where to look for a config file on your system");
                            action_button(ui, "Ignore and run default config for now", ConfigUiAction::Close);
                        }
                        ConfigLoadError::IoError(io_err, path) if matches!(io_err.kind(), NotFound) => {
                            ui.label(format!("It seems like you have no config file at {path:?}"));
                            ui.label("Using the default config for now, please create an empty file at that path to silence this error");
                            ui.horizontal(|ui| {
                                action_button(ui, "Retry loading config", ConfigUiAction::Reload);
                                action_button(ui, "Ignore and run default config for now", ConfigUiAction::Close);
                                action_button(ui, "Create empty config file", ConfigUiAction::CreateEmptyConfig);
                                action_button(ui, "Create default config file", ConfigUiAction::CreateDefaultConfig);
                            });
                        }
                        ConfigLoadError::IoError(io_err, _) => {
                            ui.label(io_err.to_string());
                            ui.horizontal(|ui| {
                                action_button(ui, "Retry loading config", ConfigUiAction::Reload);
                                action_button(ui, "Ignore and run default config for now", ConfigUiAction::Close);
                            });
                        }
                        ConfigLoadError::TomlError(toml_err, path) => {
                            ui.label(format!("Your config file at {path:?} isn't a valid R-SNES configuration file, see the error below:"));
                            ui.code(toml_err.to_string());
                            ui.horizontal(|ui| {
                                action_button(ui, "Retry loading config", ConfigUiAction::Reload);
                                action_button(ui, "Ignore and run default config for now", ConfigUiAction::Close);
                                action_button(ui, "Open config file", ConfigUiAction::OpenConfigFile);
                            });
                        }
                    }
                });
        }
    }
}

/// Recursively show errors and their sources
fn show_error(ui: &mut egui::Ui, err: &dyn Error) {
    ui.label(err.to_string());
    if let Some(source) = err.source() {
        ui.collapsing("Caused by:", |ui| {
            show_error(ui, source);
        });
    }
    ui.collapsing("Debug representation:", |ui| {
        ui.label(format!("{err:?}"));
    });
}

/// Plugin permission prompt. Rendered only while a plugin is awaiting a
/// decision. Granting moves the plugin into `granted` for the host to inject;
/// denying (buttons, window close button, or Escape) drops it.
#[cfg(feature = "plugins")]
pub fn plugin_perm_request(
    ctx: &egui::Context,
    area: egui::Rect,
    pending: &mut Option<PendingPlugin>,
    granted: &mut Option<Plugin>,
) {
    if pending.is_none() {
        return;
    }

    let mut window_open = true;

    // Borrow the pending plugin to render its request, then pull the outcome
    // and checkbox state out so the borrow ends before we mutate `pending`.
    let (outcome, show_none) = {
        let p = pending.as_ref().unwrap();
        let mut req = p.plugin.perm_request();
        req.show_none = p.show_none;

        egui::Window::new("Plugin Permissions")
            .constrain_to(area)
            .open(&mut window_open)
            .collapsible(false)
            .resizable(true)
            .default_width(320.0)
            .default_height(400.0)
            .vscroll(true)
            .show(ctx, |ui| {
                let name = p
                    .plugin
                    .path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "<in-memory plugin>".to_owned());
                ui.label(RichText::new(name).monospace());
                req.show_gui(ui);
            });

        (req.outcome, req.show_none)
    };

    if let Some(p) = pending.as_mut() {
        p.show_none = show_none;
    }

    match outcome {
        PermOutcome::Granted => {
            let taken = pending.take().unwrap();
            *granted = Some(taken.plugin);
        }
        PermOutcome::Denied => *pending = None,
        // [x] or Escape while undecided : deny.
        PermOutcome::Pending => {
            if !window_open {
                *pending = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::test_utils::*;
    use egui_kittest::kittest::{NodeT, Queryable};
    use std::time::Duration;

    /// The game area when R-SNES opens.
    const GAME_AREA: egui::Vec2 = Gui::START_GAME_AREA;

    // ---- Wording ----

    #[test]
    fn ages_are_written_in_words() {
        let now = SystemTime::now();
        let ago = |seconds| time_ago(now, now - Duration::from_secs(seconds));

        assert_eq!(ago(0), "just now");
        assert_eq!(ago(59), "just now");
        assert_eq!(ago(60), "1 minute ago");
        assert_eq!(ago(59 * 60), "59 minutes ago");
        assert_eq!(ago(3600), "1 hour ago");
        assert_eq!(ago(23 * 3600), "23 hours ago");
        assert_eq!(ago(86_400), "1 day ago");
        assert_eq!(ago(3 * 86_400), "3 days ago");
        // A clock set back: the save looks like it is from the future.
        assert_eq!(time_ago(now, now + Duration::from_secs(90)), "just now");
    }

    #[test]
    fn file_sizes_are_written_in_words() {
        assert_eq!(file_size_text(0), "0 bytes");
        assert_eq!(file_size_text(1023), "1023 bytes");
        assert_eq!(file_size_text(1024), "1 KiB");
        assert_eq!(file_size_text(588_800), "575 KiB");
        assert_eq!(file_size_text(3 * 1024 * 1024 / 2), "1.5 MiB");
    }

    #[test]
    fn local_dates_have_the_day_and_the_second() {
        let shape: String = local_date(SystemTime::now())
            .chars()
            .map(|c| if c.is_ascii_digit() { '0' } else { c })
            .collect();
        assert_eq!(shape, "0000-00-00 00:00:00");
    }

    // ---- Menu bar ----

    #[test]
    fn the_menu_opens_the_save_state_window() {
        let mut harness = menu_harness(false);

        click(&mut harness, "Emulation");
        harness.get_by_label_contains("Save states").click();
        harness.run();

        assert_eq!(harness.state().commands, [Command::ChooseSaveState]);
    }

    #[test]
    fn the_menu_shows_the_save_state_window_open() {
        for open in [false, true] {
            let mut harness = menu_harness(open);
            click(&mut harness, "Emulation");

            let toggled = harness
                .get_by_label_contains("Save states")
                .accesskit_node()
                .toggled();
            let expected = if open {
                egui::accesskit::Toggled::True
            } else {
                egui::accesskit::Toggled::False
            };
            assert_eq!(toggled, Some(expected), "window open: {open}");
        }
    }

    // ---- Save state window: actions ----

    #[test]
    fn a_click_selects_a_slot() {
        let mut harness = picker_harness(GAME_AREA, |_| {});

        click(&mut harness, "Slot 4");

        assert_eq!(picker(&harness).selected, 3);
        assert_eq!(picker(&harness).action, None, "a click only selects");
        let slot = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Slot 4");
        assert_eq!(
            slot.accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::True),
            "the selected slot is shown selected"
        );
    }

    #[test]
    fn a_double_click_loads_a_saved_slot() {
        let mut harness = picker_harness(GAME_AREA, |_| {});
        let slot = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Slot 4");
        slot.click();
        slot.click();
        harness.run();

        assert_eq!(picker(&harness).action, Some(SlotAction::Load(4)));
    }

    #[test]
    fn load_loads_the_selected_slot() {
        let mut harness = picker_harness(GAME_AREA, |picker| picker.select(4));

        click(&mut harness, "Load");

        assert_eq!(picker(&harness).action, Some(SlotAction::Load(5)));
    }

    #[test]
    fn only_a_saved_slot_can_be_loaded() {
        for (index, what) in [(5, "unreadable"), (9, "empty")] {
            let harness = picker_harness(GAME_AREA, |picker| picker.select(index));
            assert!(is_disabled(&harness, "Load"), "{what}");
        }
    }

    #[test]
    fn saving_over_a_used_slot_asks_first() {
        let mut harness = picker_harness(GAME_AREA, |picker| picker.select(3));

        click(&mut harness, "Save here");
        assert_eq!(picker(&harness).action, None, "nothing saved yet");
        harness.get_by_label("Save over slot 4?");

        click(&mut harness, "Save over");
        assert_eq!(picker(&harness).action, Some(SlotAction::Save(4)));
        assert_eq!(picker(&harness).confirming, None);
    }

    #[test]
    fn a_confirmation_can_be_cancelled() {
        let mut harness = picker_harness(GAME_AREA, |picker| picker.select(3));
        click(&mut harness, "Delete");
        harness.get_by_label("Delete slot 4?");

        click(&mut harness, "Cancel");

        assert_eq!(picker(&harness).confirming, None);
        assert_eq!(picker(&harness).action, None);
        assert!(harness.query_by_label("Delete slot 4?").is_none());
    }

    #[test]
    fn saving_into_an_empty_slot_does_not_ask() {
        let mut harness = picker_harness(GAME_AREA, |picker| picker.select(9));

        click(&mut harness, "Save here");

        assert_eq!(picker(&harness).action, Some(SlotAction::Save(10)));
    }

    #[test]
    fn deleting_asks_first() {
        let mut harness = picker_harness(GAME_AREA, |picker| picker.select(3));

        click(&mut harness, "Delete");
        assert_eq!(picker(&harness).action, None, "nothing deleted yet");
        click(&mut harness, "Delete");

        assert_eq!(picker(&harness).action, Some(SlotAction::Delete(4)));
    }

    #[test]
    fn an_empty_slot_can_only_be_saved_into() {
        let harness = picker_harness(GAME_AREA, |picker| picker.select(9));

        for button in ["Load", "Rename", "Lock", "Delete"] {
            assert!(is_disabled(&harness, button), "{button}");
        }
        assert!(!is_disabled(&harness, "Save here"));
        assert!(!is_disabled(&harness, "Open folder"));
    }

    #[test]
    fn a_locked_slot_can_be_loaded_and_unlocked_only() {
        let mut harness = picker_harness(GAME_AREA, |picker| picker.select(2));

        assert!(is_disabled(&harness, "Save here"));
        assert!(is_disabled(&harness, "Delete"));
        assert!(!is_disabled(&harness, "Load"));

        click(&mut harness, "Unlock");
        assert_eq!(
            picker(&harness).action,
            Some(SlotAction::SetLocked(3, false))
        );
    }

    #[test]
    fn lock_locks_the_selected_slot() {
        let mut harness = picker_harness(GAME_AREA, |picker| picker.select(3));

        click(&mut harness, "Lock");

        assert_eq!(
            picker(&harness).action,
            Some(SlotAction::SetLocked(4, true))
        );
    }

    #[test]
    fn a_name_is_typed_then_confirmed_with_enter() {
        let mut harness = picker_harness(GAME_AREA, |picker| picker.select(3));
        click(&mut harness, "Rename");

        let field = harness.get_by_role(egui::accesskit::Role::TextInput);
        assert!(field.is_focused(), "the field takes the keyboard");
        field.type_text("boss");
        harness.run();
        harness.key_press(egui::Key::Enter);
        harness.run();

        assert_eq!(
            picker(&harness).action,
            Some(SlotAction::Rename(4, "boss".into()))
        );
        assert_eq!(picker(&harness).new_name, None);
    }

    #[test]
    fn a_name_is_confirmed_with_ok_or_dropped_with_cancel() {
        let mut harness = picker_harness(GAME_AREA, |picker| picker.select(3));
        click(&mut harness, "Rename");
        harness
            .get_by_role(egui::accesskit::Role::TextInput)
            .type_text("boss");
        harness.run();
        click(&mut harness, "Cancel");
        assert_eq!(picker(&harness).action, None);
        assert_eq!(picker(&harness).new_name, None);

        click(&mut harness, "Rename");
        harness
            .get_by_role(egui::accesskit::Role::TextInput)
            .type_text("boss");
        harness.run();
        click(&mut harness, "OK");
        assert_eq!(
            picker(&harness).action,
            Some(SlotAction::Rename(4, "boss".into()))
        );
    }

    #[test]
    fn open_folder_opens_the_folder() {
        let mut harness = picker_harness(GAME_AREA, |_| {});

        click(&mut harness, "Open folder");

        assert_eq!(picker(&harness).action, Some(SlotAction::OpenFolder));
    }

    #[test]
    fn the_close_button_closes_the_window() {
        let mut harness = picker_harness(GAME_AREA, |_| {});

        click(&mut harness, "Close window");

        assert!(harness.state().is_none());
    }

    #[test]
    fn slots_show_their_name_or_their_age() {
        let harness = picker_harness(GAME_AREA, |_| {});

        harness.get_by_label(&"W".repeat(MAX_NAME_LENGTH));
        assert!(harness.query_all_by_label("just now").next().is_some());
    }

    #[test]
    fn the_latest_and_locked_slots_say_so() {
        let harness = picker_harness(GAME_AREA, |_| {});

        harness.get_by_role_and_label(egui::accesskit::Role::Button, "Slot 1, latest");
        harness.get_by_role_and_label(egui::accesskit::Role::Button, "Slot 3, locked");
        harness.get_by_role_and_label(egui::accesskit::Role::Button, "Slot 2");
    }

    #[test]
    fn the_details_show_the_selected_slot() {
        let harness = picker_harness(GAME_AREA, |picker| picker.select(1));
        harness.get_by_label(&format!("Slot 2: {}", "W".repeat(MAX_NAME_LENGTH)));
        harness.get_by_label_contains("575 KiB");

        let harness = picker_harness(GAME_AREA, |picker| picker.select(5));
        harness.get_by_label_contains("Unreadable, written");
    }

    // ---- Save state window: layout ----

    /// Game areas from the smallest R-SNES window to a large screen, some
    /// limited by their width, others by their height.
    const GAME_AREAS: [egui::Vec2; 7] = [
        Gui::MIN_GAME_AREA,
        GAME_AREA,
        egui::vec2(400.0, 300.0),
        egui::vec2(700.0, 1000.0),
        egui::vec2(1920.0, 350.0),
        egui::vec2(1920.0, 400.0),
        egui::vec2(1920.0, 1000.0),
    ];

    /// Sets a picker up before it is laid out.
    type Prepare = fn(&mut SaveStatePicker);

    /// Every slot selected, then a name being typed and both confirmations.
    const EVERY_SELECTION: [Prepare; 13] = [
        |picker| picker.select(0),
        |picker| picker.select(1),
        |picker| picker.select(2),
        |picker| picker.select(3),
        |picker| picker.select(4),
        |picker| picker.select(5),
        |picker| picker.select(6),
        |picker| picker.select(7),
        |picker| picker.select(8),
        |picker| picker.select(9),
        |picker| picker.new_name = Some("W".repeat(MAX_NAME_LENGTH)),
        |picker| picker.confirming = Some(Confirmation::Overwrite),
        |picker| {
            picker.select(1);
            picker.confirming = Some(Confirmation::Delete);
        },
    ];

    /// The rectangle of the save state window.
    fn window_rect<State>(harness: &egui_kittest::Harness<'_, State>) -> egui::Rect {
        harness
            .ctx
            .memory(|memory| memory.area_rect(egui::Id::new(PICKER_WINDOW_ID)))
            .expect("the window was drawn")
    }

    #[test]
    fn the_save_state_window_fits_the_game_area() {
        for size in GAME_AREAS {
            for prepare in EVERY_SELECTION {
                let harness = picker_harness(size, prepare);
                let window = window_rect(&harness);
                let area = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
                // Up to rounding.
                assert!(
                    area.expand(0.5).contains_rect(window),
                    "{window:?} is out of {area:?}"
                );
            }
        }
    }

    #[test]
    fn the_save_state_window_keeps_its_size_whatever_the_selection() {
        for size in [GAME_AREA, egui::vec2(1920.0, 1000.0)] {
            let first = window_rect(&picker_harness(size, |_| {}));
            for prepare in EVERY_SELECTION {
                let window = window_rect(&picker_harness(size, prepare));
                assert!(
                    (window.size() - first.size()).length() < 0.5,
                    "{:?} instead of {:?} in {size:?}",
                    window.size(),
                    first.size()
                );
            }
        }
    }

    #[test]
    fn the_save_state_window_grows_back_after_a_shrink() {
        for big in GAME_AREAS {
            let fresh = window_rect(&picker_harness(big, |_| {}));

            let mut harness = picker_harness(big, |_| {});
            for size in [Gui::MIN_GAME_AREA, big] {
                harness.set_size(size);
                harness.run();
            }

            let resized = window_rect(&harness);
            assert!(
                (resized.size() - fresh.size()).length() < 0.5,
                "{:?} instead of {:?} in {big:?}",
                resized.size(),
                fresh.size()
            );
        }
    }

    #[test]
    fn the_save_state_window_grows_with_the_game_area() {
        let default = window_rect(&picker_harness(GAME_AREA, |_| {}));
        let large = window_rect(&picker_harness(egui::vec2(1920.0, 1000.0), |_| {}));

        assert!(large.width() > 2.0 * default.width(), "{large:?}");
        assert!(
            large.width() <= PICKER_AREA_SHARE * 1920.0 + 0.5,
            "a border stays: {large:?}"
        );
    }

    #[test]
    fn the_thumbnails_use_the_height_of_a_low_window() {
        let harness = picker_harness(egui::vec2(1920.0, 400.0), |_| {});
        let window = window_rect(&harness);

        assert!(
            window.height() > 400.0 - 40.0,
            "{} high in 400",
            window.height()
        );
    }

    #[test]
    fn the_open_folder_button_is_at_the_right_edge() {
        let harness = picker_harness(GAME_AREA, |_| {});
        let button = harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Open folder")
            .rect();
        let lock = harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Lock")
            .rect();
        let window = window_rect(&harness);

        assert!(
            window.right() - button.right() < 20.0,
            "{button:?} in {window:?}"
        );
        assert!(
            (button.center().y - lock.center().y).abs() < 1.0,
            "same row"
        );
    }

    // ---- Notification ----

    /// The rectangle of each notification in `texts`, shown one after the
    /// other in the default game area.
    fn notification_rects(texts: &[&str]) -> Vec<egui::Rect> {
        let mut harness = notification_harness(GAME_AREA);
        texts
            .iter()
            .map(|text| {
                *harness.state_mut() = Some(Notification::new((*text).to_owned()));
                harness.run();
                harness
                    .ctx
                    .memory(|memory| memory.area_rect(egui::Id::new("notification")))
                    .expect("the notification was drawn")
            })
            .collect()
    }

    #[test]
    fn a_notification_is_not_squeezed_by_the_one_before() {
        let long = "Couldn't read slot 3: the file was written by another version of R-SNES";
        let rects = notification_rects(&[long, "Saved to slot 1", long]);

        assert!(
            (rects[2].size() - rects[0].size()).length() < 0.5,
            "{:?} after a short notification, {:?} alone",
            rects[2].size(),
            rects[0].size()
        );
    }

    #[test]
    fn a_long_notification_wraps_inside_the_game_area() {
        let rects = notification_rects(&[&"a long notification ".repeat(20)]);
        let area = egui::Rect::from_min_size(egui::Pos2::ZERO, GAME_AREA);

        assert!(area.contains_rect(rects[0]), "{:?}", rects[0]);
        assert!(
            rects[0].width() <= NOTIFICATION_MAX_WIDTH + 0.5,
            "{:?}",
            rects[0]
        );
    }

    #[test]
    fn an_expired_notification_is_removed() {
        let mut harness = notification_harness(GAME_AREA);
        let mut old = Notification::new("Saved to slot 1".into());
        old.shown_at -= Notification::DURATION;
        *harness.state_mut() = Some(old);

        harness.run();

        assert!(harness.state().is_none());
        assert!(harness.query_by_label("Saved to slot 1").is_none());
    }
}
