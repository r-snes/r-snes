use std::error::Error;

use bus::cartridge::header::RomHeader;
use egui_sdl2::egui::{self, RichText};

use crate::rsnes::RomInfo;

use super::GuiFrameData;
use super::command::Command;
use super::state::GuiState;

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
                    item(ui, Command::OpenRom, "Open ROM…", false);
                    item(ui, Command::CloseRom, "Close ROM", false);
                    #[cfg(feature = "plugins")]
                    {
                        ui.separator();
                        item(ui, Command::LoadPlugin, "Load plugin…", false);
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
/// denying (buttons, [x], or Escape) drops it.
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
