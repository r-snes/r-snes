use std::path::PathBuf;

use egui_sdl2::canvas::EguiCanvas;
use egui_sdl2::{egui, sdl2};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use ppu::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub struct Gui {
    _sdl_ctx: sdl2::Sdl,
    egui_canvas: EguiCanvas,
    event_pump: sdl2::EventPump,
}

#[derive(PartialEq, Eq, Debug)]
pub enum RSnesEvent {
    /// Load a new ROM, showing a file picker (closes current game)
    LoadRom { path: PathBuf },

    /// Close the currently open game (or quit if no game open)
    Close,

    /// Quit the emulator program altogether
    Quit,

    /// An key mapped to an emulated button has been pressed
    ButtonDown,

    /// An key mapped to an emulated button has been released
    ButtonUp,

    /// Run the `default` action of a plugin (if a plugin is loaded, and if it defines one)
    RunPluginDefault,
}

#[cfg(not(tarpaulin_include))]
impl Gui {
    pub const FRAME_RATE: u16 = 60;
    pub const FRAME_DURATION: f64 = 1.0 / Self::FRAME_RATE as f64;

    pub fn new() -> Result<Self, String> {
        let sdl_ctx = sdl2::init()?;
        let video_subsystem = sdl_ctx.video()?;

        let window = video_subsystem
            .window("R-SNES", SCREEN_WIDTH as u32 * 2, SCREEN_HEIGHT as u32 * 2)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        let egui_canvas = EguiCanvas::new(window);
        let event_pump = sdl_ctx.event_pump()?;

        Ok(Gui {
            _sdl_ctx: sdl_ctx,
            egui_canvas,
            event_pump,
        })
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        self.egui_canvas.clear([r, g, b, 255]);
    }

    pub fn present(&mut self) {
        self.egui_canvas.present();
    }

    fn map_event(event: &Event) -> Option<RSnesEvent> {
        use sdl2::keyboard::Mod;

        match event {
            Event::Quit { .. } => Some(RSnesEvent::Quit),
            Event::KeyDown {
                keycode: Some(Keycode::Q),
                keymod,
                ..
            } if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => Some(RSnesEvent::Quit),

            Event::KeyDown {
                keycode: Some(Keycode::Escape),
                repeat: false,
                ..
            } => Some(RSnesEvent::Close),

            Event::KeyDown {
                keycode: Some(Keycode::L),
                keymod,
                ..
            } if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => {
                match rfd::FileDialog::new().pick_file() {
                    Some(path) => Some(RSnesEvent::LoadRom { path }),
                    None => None,
                }
            }

            Event::KeyDown {
                keycode: Some(Keycode::Space),
                repeat: false,
                keymod,
                ..
            } if !keymod
                .intersects(Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LALTMOD | Mod::RALTMOD) =>
            {
                Some(RSnesEvent::ButtonDown)
            }

            Event::KeyUp {
                keycode: Some(Keycode::Space),
                ..
            } => Some(RSnesEvent::ButtonUp),

            Event::KeyDown {
                keycode: Some(Keycode::R),
                keymod,
                ..
            } if !keymod
                .intersects(Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LALTMOD | Mod::RALTMOD) =>
            {
                Some(RSnesEvent::RunPluginDefault)
            }

            _ => None,
        }
    }

    fn handle_events(&mut self) -> Vec<RSnesEvent> {
        let pending: Vec<Event> = self.event_pump.poll_iter().collect();

        let mut out = Vec::new();
        for event in pending {
            let response = self.egui_canvas.on_event(&event);
            if response.consumed {
                continue;
            }
            if let Some(rsnes_event) = Self::map_event(&event) {
                out.push(rsnes_event);
            }
        }
        out
    }

    pub fn draw_framebuffer(
        &mut self,
        framebuffer: &ppu::rendering::RawFramebuffer,
    ) -> Result<(), String> {
        use sdl2::pixels::PixelFormatEnum;

        let canvas = &mut self.egui_canvas.painter.canvas;
        let texture_creator = canvas.texture_creator();

        let mut texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::RGB24,
                SCREEN_WIDTH as u32,
                SCREEN_HEIGHT as u32,
            )
            .map_err(|e| e.to_string())?;

        texture
            .update(None, framebuffer, SCREEN_WIDTH * 3)
            .map_err(|e| e.to_string())?;

        canvas.copy(&texture, None, None)?;

        Ok(())
    }

    pub fn update(
        &mut self,
        framebuffer: &ppu::rendering::RawFramebuffer,
        run_ui: impl FnMut(&egui::Context),
    ) -> Vec<RSnesEvent> {
        let events = self.handle_events();

        self.clear(30, 30, 35);
        let _ = self.draw_framebuffer(framebuffer);

        self.egui_canvas.run(run_ui);
        self.egui_canvas.paint();
        self.present();

        events
    }
}
