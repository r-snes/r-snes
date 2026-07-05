use std::path::PathBuf;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use ppu::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub struct Gui {
    _sdl_ctx: sdl2::Sdl,
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    event_pump: sdl2::EventPump,
}

#[derive(PartialEq, Eq)]
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

        let canvas = window
            .into_canvas()
            .accelerated()
            .build()
            .map_err(|e| e.to_string())?;

        let event_pump = sdl_ctx.event_pump()?;

        Ok(Gui {
            _sdl_ctx: sdl_ctx,
            canvas,
            event_pump,
        })
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        self.canvas
            .set_draw_color(sdl2::pixels::Color::RGB(r, g, b));
        self.canvas.clear();
    }

    pub fn present(&mut self) {
        self.canvas.present();
    }

    fn map_event(event: sdl2::event::Event) -> Option<RSnesEvent> {
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

            _ => None,
        }
    }

    fn handle_events(&mut self) -> impl Iterator<Item = RSnesEvent> {
        self.event_pump.poll_iter().filter_map(Self::map_event)
    }

    pub fn wait_for_event(&mut self) -> RSnesEvent {
        loop {
            match Self::map_event(self.event_pump.wait_event()) {
                Some(e) => return e,
                None => {}
            }
        }
    }

    pub fn draw_framebuffer(
        &mut self,
        framebuffer: &ppu::rendering::RawFramebuffer,
    ) -> Result<(), String> {
        use sdl2::pixels::PixelFormatEnum;

        let texture_creator = self.canvas.texture_creator();

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

        self.canvas.copy(&texture, None, None)?;

        Ok(())
    }

    pub fn update(
        &mut self,
        framebuffer: &ppu::rendering::RawFramebuffer,
    ) -> impl Iterator<Item = RSnesEvent> + use<'_> {
        self.clear(30, 30, 35);
        let _ = self.draw_framebuffer(framebuffer); // TODO: Handle error properly
        self.present();

        self.handle_events() // Handle events after presenting window because it's borrowing mut self
    }
}
