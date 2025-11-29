use std::path::PathBuf;

use eframe::egui;
use egui::{Id, UiKind};
use rfd::FileDialog;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1920.0, 1080.0]),
        ..Default::default()
    };
    eframe::run_native(
        "R-SNES",
        options,
        Box::new(|cc| {
            // egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::default())
        }),
    )
}

struct MyApp {
    pub rom_path: Option<PathBuf>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self { rom_path: None }
    }
}

impl eframe::App for MyApp {
    // This is the main update function of the window
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.top_menu_bar(ctx);
        self.central_panel(ctx);
    }
}

impl MyApp {
    fn top_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top(Id::new("context_menu")).show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load ROM").clicked() {
                        if let Some(path) = FileDialog::new().pick_file() {
                            self.rom_path = Some(path)
                        }
                        ui.close_kind(UiKind::Menu);
                    }

                    if ui.button("Quit").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    if ui.button("Zoom In").clicked() {
                        egui::gui_zoom::zoom_in(ctx);
                    }

                    if ui.button("Zoom Out").clicked() {
                        egui::gui_zoom::zoom_out(ctx);
                    }
                });
            });
        });
    }

    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("R-SNES");
        });
    }
}
