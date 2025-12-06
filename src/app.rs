use crate::rsnes::Rsnes;
use egui::{Id, UiKind};
use rfd::FileDialog;

pub struct RsnesApp {
    pub emulator: Option<Rsnes>,

    // Raw framebuffer for now, waiting for ppu link
    framebuffer: Vec<u8>,
    width: usize,
    height: usize,
    texture: Option<egui::TextureHandle>,
}

impl Default for RsnesApp {
    fn default() -> Self {
        let width = 300;
        let height = 300;

        // Dummy framebuffer with a gradient
        let mut framebuffer = vec![0u8; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) * 4;
                framebuffer[i] = x as u8;
                framebuffer[i + 1] = y as u8;
                framebuffer[i + 2] = 50;
                framebuffer[i + 3] = 255;
            }
        }

        Self {
            emulator: None,
            framebuffer,
            width: width,
            height: height,
            texture: None,
        }
    }
}

impl eframe::App for RsnesApp {
    // This is the main update function of the window
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.top_menu_bar(ctx);
        self.central_panel(ctx);
    }
}

impl RsnesApp {
    fn top_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top(Id::new("context_menu")).show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load ROM").clicked() {
                        if let Some(path) = FileDialog::new().pick_file() {
                            // self.rom_path = Some(path)
                            self.emulator = match Rsnes::load_rom(&path) {
                                Ok(emu) => Some(emu),
                                Err(err) => None,
                            }
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

            match &self.emulator {
                Some(emu) => ui.label(emu.rom_path.display().to_string()),
                None => ui.label("No ROM loaded"),
            };

            // Create or update texture
            if self.texture.is_none() {
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [self.width, self.height],
                    &self.framebuffer,
                );

                self.texture =
                    Some(ctx.load_texture("framebuffer", img, egui::TextureOptions::NEAREST));
            } else {
                if let Some(tex) = self.texture.as_mut() {
                    let img = egui::ColorImage::from_rgba_unmultiplied(
                        [self.width, self.height],
                        &self.framebuffer,
                    );
                    tex.set(img, egui::TextureOptions::NEAREST);
                }
            }

            // Display framebuffer
            if let Some(tex) = &self.texture {
                let desired_size = egui::vec2(
                    self.width as f32, // 2x scale
                    self.height as f32,
                );
                ui.image((tex.id(), desired_size));
            }
        });
    }
}
