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

    // Raw framebuffer for now, waiting for ppu link
    framebuffer: Vec<u8>,
    fb_width: usize,
    fb_height: usize,
    fb_texture: Option<egui::TextureHandle>,
}

impl Default for MyApp {
    fn default() -> Self {
        let fb_width = 256;
        let fb_height = 224;

        // Dummy framebuffer with a gradient
        let mut framebuffer = vec![0u8; fb_width * fb_height * 4];
        for y in 0..fb_height {
            for x in 0..fb_width {
                let i = (y * fb_width + x) * 4;
                framebuffer[i] = x as u8; // R
                framebuffer[i + 1] = y as u8; // G
                framebuffer[i + 2] = 50; // B
                framebuffer[i + 3] = 255; // A
            }
        }

        Self {
            rom_path: None,
            framebuffer,
            fb_width,
            fb_height,
            fb_texture: None,
        }
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

            ui.separator();

            ui.label("Loaded ROM:");

            match &self.rom_path {
                Some(path) => ui.label(path.display().to_string()),
                None => ui.label("No ROM loaded"),
            };

            // Create or update texture
            if self.fb_texture.is_none() {
                // First time: allocate the texture
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [self.fb_width, self.fb_height],
                    &self.framebuffer,
                );

                self.fb_texture =
                    Some(ctx.load_texture("framebuffer", img, egui::TextureOptions::NEAREST));
            } else {
                // Update existing texture
                if let Some(tex) = self.fb_texture.as_mut() {
                    let img = egui::ColorImage::from_rgba_unmultiplied(
                        [self.fb_width, self.fb_height],
                        &self.framebuffer,
                    );
                    tex.set(img, egui::TextureOptions::NEAREST);
                }
            }

            // Display framebuffer
            if let Some(tex) = &self.fb_texture {
                let desired_size = egui::vec2(
                    self.fb_width as f32 * 2.0, // 2x scale
                    self.fb_height as f32 * 2.0,
                );
                ui.image((tex.id(), desired_size));
            }
        });
    }
}
