mod app;
mod rsnes;

use app::RsnesApp;
use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1920.0, 1080.0]),
        ..Default::default()
    };
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "R-SNES",
        native_options,
        Box::new(|_cc| Ok(Box::<RsnesApp>::default())),
    )
}
