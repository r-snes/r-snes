use egui_sdl2::egui::{self, accesskit::Invalid::True};

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
pub fn rom_info(ctx: &egui::Context, open: &mut bool, info: Option<RomInfo>) {
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

            let h = &info.header;

            // --- Primary info: what you actually want at a glance ---
            egui::Grid::new("rom_info_primary")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Title");
                    // Header titles are space-padded to 21 bytes.
                    ui.label(h.title.trim());
                    ui.end_row();

                    ui.label("Mapping mode");
                    ui.label(h.mapping_mode.to_string());
                    ui.end_row();

                    ui.label("ROM speed");
                    ui.label(h.rom_speed.to_string());
                    ui.end_row();

                    ui.label("File size");
                    ui.label(format!("{} KB", info.file_size_kb));
                    ui.end_row();

                    ui.label("Region");
                    ui.label(format!("{} ({})", h.country, h.video_standard));
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
                            ui.label(h.hardware.layout.to_string());
                            ui.end_row();

                            ui.label("Coprocessor");
                            ui.label(if h.hardware.has_coprocessor() {
                                match &h.hardware.coprocessor {
                                    Some(c) => c.to_string(),
                                    None => "Unknown".to_owned(),
                                }
                            } else {
                                "None".to_owned()
                            });
                            ui.end_row();

                            ui.label("ROM size (header)");
                            ui.label(match decode_size_kb(h.rom_size) {
                                Some(kb) => format!("{} KB (exp. {})", kb, h.rom_size),
                                None => format!("none (exp. {})", h.rom_size),
                            });
                            ui.end_row();

                            ui.label("SRAM size");
                            ui.label(match decode_size_kb(h.ram_size) {
                                Some(kb) => format!("{} KB (exp. {})", kb, h.ram_size),
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
                            ui.label(format!("${:02X}", h.developer_id));
                            ui.end_row();

                            ui.label("ROM version");
                            ui.label(format!("1.{}", h.rom_version));
                            ui.end_row();

                            ui.label("Checksum");
                            ui.label(format!("${:04X}", h.checksum));
                            ui.end_row();

                            ui.label("Complement");
                            // The two should XOR to 0xFFFF on a valid cartridge.
                            let valid = h.checksum ^ h.checksum_complement == 0xFFFF;
                            ui.label(format!(
                                "${:04X} {}",
                                h.checksum_complement,
                                if valid { "✓" } else { "✗" }
                            ));
                            ui.end_row();
                        });
                });

            egui::CollapsingHeader::new("File")
                .default_open(false)
                .show(ui, |ui| {
                    // Paths get long; wrap rather than stretching the window.
                    ui.add(egui::Label::new(info.path.display().to_string()).wrap());
                });
        });
}
