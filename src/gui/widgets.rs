use std::error::Error;
use std::time::SystemTime;

use bus::cartridge::header::RomHeader;
use egui_sdl2::egui::{self, RichText};
use ppu::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};

use crate::rsnes::RomInfo;
use crate::save_state::slots::MAX_NAME_LENGTH;

use super::command::Command;
use super::state::{Confirmation, GuiState, Notification, PickerContent, PickerEntry, SlotPicker};
use super::{GuiFrameData, SlotAction};

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
                    item(ui, Command::SaveState, "Save state", false);
                    item(ui, Command::LoadLatestState, "Load latest state", false);
                    item(
                        ui,
                        Command::ChooseSaveState,
                        "Save states...",
                        state.slot_picker.is_some(),
                    );
                    item(ui, Command::UndoLoadState, "Undo load", false);
                });

                ui.menu_button("View", |ui| {
                    item(
                        ui,
                        Command::ToggleRomInfo,
                        "ROM information",
                        state.show_rom_info,
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

/// Decodes a SNES header size exponent into KB. `0` means "none".
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
                    ui.label(format!("{file_size_kb} KB"));
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
                                Some(kb) => format!("{kb} KB (exp. {rom_size})"),
                                None => format!("none (exp. {rom_size})",),
                            });
                            ui.end_row();

                            ui.label("SRAM size");
                            ui.label(match decode_size_kb(*ram_size) {
                                Some(kb) => format!("{kb} KB (exp. {ram_size})"),
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

/// A file size in words, e.g. "575 KB".
fn file_size_text(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    match bytes {
        0..KB => format!("{bytes} bytes"),
        KB..MB => format!("{} KB", (bytes + KB / 2) / KB),
        _ => format!("{:.1} MB", bytes as f64 / MB as f64),
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
/// R-SNES's window when it opens.
const PICKER_FULL_AREA: egui::Vec2 =
    egui::vec2(SCREEN_WIDTH as f32 * 2.0, SCREEN_HEIGHT as f32 * 2.0);

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
    let (columns, rows) = (SlotPicker::COLUMNS as f32, rows as f32);
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
pub fn slot_picker(ctx: &egui::Context, area: egui::Rect, picker: &mut Option<SlotPicker>) {
    let Some(picker_state) = picker else {
        return;
    };

    let rows = picker_state.entries.len().div_ceil(SlotPicker::COLUMNS);
    let (thumbnail, scrolls) = thumbnail_size(ctx, area, rows);
    let cell_width = thumbnail.x + ctx.global_style().spacing.button_padding.x * 2.0;
    let columns = SlotPicker::COLUMNS as f32;
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

                egui::Grid::new("slot_picker")
                    .num_columns(SlotPicker::COLUMNS)
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

                            if (index + 1) % SlotPicker::COLUMNS == 0 {
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
fn slot_details(ui: &mut egui::Ui, picker: &mut SlotPicker, now: SystemTime) {
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
            let field = ui.add(
                egui::TextEdit::singleline(new_name)
                    .id(name_field)
                    .char_limit(MAX_NAME_LENGTH)
                    .hint_text("Slot name")
                    // No taller than the buttons it replaces.
                    .margin(egui::vec2(4.0, 1.0))
                    .desired_width(field_width),
            );
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
            ui.memory_mut(|memory| memory.request_focus(name_field));
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

    // Longer texts wrap onto more lines.
    const MAX_WIDTH: f32 = 480.0;
    const MARGIN: f32 = 8.0;

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
        .default_width(MAX_WIDTH.min(area.width() - MARGIN * 2.0))
        .pivot(egui::Align2::LEFT_BOTTOM)
        .fixed_pos(area.left_bottom() + egui::vec2(MARGIN, -MARGIN))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(&shown.text);
            });
        });
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
    use std::time::Duration;

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
        assert_eq!(file_size_text(1024), "1 KB");
        assert_eq!(file_size_text(588_800), "575 KB");
        assert_eq!(file_size_text(3 * 1024 * 1024 / 2), "1.5 MB");
    }

    #[test]
    fn local_dates_have_the_day_and_the_second() {
        let shape: String = local_date(SystemTime::now())
            .chars()
            .map(|c| if c.is_ascii_digit() { '0' } else { c })
            .collect();
        assert_eq!(shape, "0000-00-00 00:00:00");
    }
}
