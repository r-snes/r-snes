use std::error::Error;
use std::time::SystemTime;

use bus::cartridge::header::RomHeader;
use egui_sdl2::egui::{self, RichText};
use ppu::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};

use crate::rsnes::RomInfo;

use super::GuiFrameData;
use super::command::Command;
use super::state::{GuiState, Notification, SlotPicker};

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
                    item(ui, Command::ChooseSaveState, "Load state...", false);
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

/// The save state picker: the used slots, the most recent first, each with
/// its saved screen, number and age. Selection of the slot is possible with
/// the mouse or the keyboard arrows and Enter.
pub fn slot_picker(ctx: &egui::Context, area: egui::Rect, picker: &mut Option<SlotPicker>) {
    let Some(picker_state) = picker else {
        return;
    };

    let columns = SlotPicker::COLUMNS as f32;
    let width = ((area.width() - 40.0) / columns - 16.0).clamp(32.0, SCREEN_WIDTH as f32 / 2.0);
    let thumbnail = egui::vec2(width, width * SCREEN_HEIGHT as f32 / SCREEN_WIDTH as f32);

    let now = SystemTime::now();
    let mut open = true;
    let mut clicked = None;

    egui::Window::new("Load state")
        .constrain_to(area)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .vscroll(true)
        .show(ctx, |ui| {
            if picker_state.entries.is_empty() {
                ui.label("No save state for this game yet.");
                return;
            }

            egui::Grid::new("slot_picker")
                .num_columns(SlotPicker::COLUMNS)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    for (index, entry) in picker_state.entries.iter().enumerate() {
                        ui.vertical(|ui| {
                            let selected = index == picker_state.selected;
                            let response = match &entry.screen {
                                Ok(texture) => {
                                    let image = egui::Image::from_texture(texture)
                                        .fit_to_exact_size(thumbnail);
                                    ui.add(egui::Button::image(image).selected(selected))
                                }
                                Err(reason) => ui
                                    .add_enabled(
                                        false,
                                        egui::Button::new("Unreadable")
                                            .min_size(thumbnail)
                                            .selected(selected),
                                    )
                                    .on_disabled_hover_text(reason),
                            };
                            if response.clicked() {
                                clicked = Some(index);
                            }
                            ui.label(format!("Slot {}", entry.slot));
                            ui.label(RichText::new(time_ago(now, entry.time)).weak().small());
                        });

                        if (index + 1) % SlotPicker::COLUMNS == 0 {
                            ui.end_row();
                        }
                    }
                });
        });

    if let Some(index) = clicked {
        picker_state.selected = index;
        picker_state.choose_selected();
    }
    if !open {
        *picker = None;
    }
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

    egui::Area::new(egui::Id::new("notification"))
        .order(egui::Order::Foreground)
        .interactable(false)
        .pivot(egui::Align2::LEFT_BOTTOM)
        .fixed_pos(area.left_bottom() + egui::vec2(8.0, -8.0))
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
        assert_eq!(ago(5 * 60 + 30), "5 minutes ago");
        assert_eq!(ago(3600), "1 hour ago");
        assert_eq!(ago(23 * 3600), "23 hours ago");
        assert_eq!(ago(24 * 3600), "1 day ago");
        assert_eq!(ago(40 * 24 * 3600), "40 days ago");
        assert_eq!(time_ago(now, now + Duration::from_secs(90)), "just now");
    }
}
