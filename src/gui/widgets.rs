use std::error::Error;

use bus::rom::header::RomHeader;
use egui_sdl2::egui::{self, RichText};

use crate::rsnes::RomInfo;

/// Decodes a SNES header size exponent into KB. `0` means "none".
fn decode_size_kb(exponent: u8) -> Option<usize> {
    if exponent == 0 {
        None
    } else {
        Some(1usize << exponent)
    }
}

/// ROM information overlay. Toggled with F1.
pub fn rom_info(ctx: &egui::Context, open: &mut bool, info: Option<&RomInfo>) {
    egui::Window::new("ROM Information")
        .open(open)
        .resizable(true)
        .collapsible(false)
        .default_width(340.0)
        .default_height(360.0)
        .vscroll(true)
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
pub fn error_box(ctx: &egui::Context, error: &mut Option<Box<dyn Error>>) {
    let mut open = error.is_some();
    let window = egui::Window::new("Error")
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
