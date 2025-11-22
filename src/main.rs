use eframe::egui;
use egui::{Id, UiKind};
use rfd::FileDialog;

struct MyApp {}

impl Default for MyApp {
    fn default() -> Self {
        Self {}
    }
}

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

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top(Id::new("context_menu")).show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load ROM").clicked() {
                        if let Some(path) = FileDialog::new().pick_file() {
                            println!("Selected ROM: {:?}", path);
                            // TODO: load ROM into emulator
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("R-SNES Heading");
        });
    }
}
