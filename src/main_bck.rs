use eframe::egui;
use egui::TextureOptions;
use rfd::FileDialog;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        ..Default::default()
    };

    eframe::run_native(
        "SNES Emulator",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()) as Box<dyn eframe::App>)),
    )
}

pub struct MyApp {
    framebuffer: Vec<u8>, // RGBA pixel buffer
    width: usize,
    height: usize,
    texture: Option<egui::TextureHandle>,
}

impl Default for MyApp {
    fn default() -> Self {
        let width = 256;
        let height = 224;
        let mut framebuffer = vec![0u8; width * height * 4];

        // Simple test pattern
        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) * 4;
                framebuffer[i] = x as u8; // R
                framebuffer[i + 1] = y as u8; // G
                framebuffer[i + 2] = 100; // B
                framebuffer[i + 3] = 255; // A
            }
        }

        Self {
            framebuffer,
            width,
            height,
            texture: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ===== MENU BAR =====
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open ROM…").clicked() {
                    if let Some(path) = FileDialog::new().pick_file() {
                        println!("Selected ROM: {:?}", path);
                        // TODO: load ROM into emulator
                    }
                    ui.close_menu();
                }

                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });
        });

        // ===== DISPLAY FRAMEBUFFER =====
        egui::CentralPanel::default().show(ctx, |ui| {
            // Upload framebuffer → texture (lazy init or update)
            if self.texture.is_none() {
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [self.width, self.height],
                    &self.framebuffer,
                );

                self.texture = Some(ui.ctx().load_texture(
                    "framebuffer",
                    img,
                    egui::TextureOptions::NEAREST,
                ));
            } else {
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [self.width, self.height],
                    &self.framebuffer,
                );

                self.texture
                    .as_mut()
                    .unwrap()
                    .set(img, TextureOptions::LINEAR);
            }

            let texture = self.texture.as_ref().unwrap();

            // Draw at scaled size
            let size = egui::vec2(self.width as f32 * 2.0, self.height as f32 * 2.0);

            ui.image((texture.id(), size));
        });

        ctx.request_repaint();
    }
}
