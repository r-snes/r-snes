use std::path::PathBuf;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;

type RawFrameBuffer = [u8; Gui::SNES_WIDTH * Gui::SNES_HEIGHT * 3];

pub struct Gui {
    _sdl_ctx: sdl2::Sdl,
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    event_pump: sdl2::EventPump,
    pub(crate) framebuffer: Box<RawFrameBuffer>,
}

pub enum RSnesEvent {
    LoadRom { path: PathBuf },
    ButtonDown,
    ButtonUp,
    Quit,
}

impl Gui {
    pub const SNES_WIDTH: usize = 256; // TODO : Remove when GUI linked with PPU
    pub const SNES_HEIGHT: usize = 224; // TODO : Remove when GUI linked with PPU

    pub const FRAME_RATE: u16 = 60;
    pub const FRAME_DURATION: f64 = 1.0 / Self::FRAME_RATE as f64;

    pub fn new() -> Result<Self, String> {
        let sdl_ctx = sdl2::init()?;
        let video_subsystem = sdl_ctx.video()?;

        let window = video_subsystem
            .window("R-SNES", Self::SNES_WIDTH as u32 * 2, Self::SNES_HEIGHT as u32 * 2)
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
            framebuffer: Box::new(*include_bytes!("../logo_framebuffer.raw")),
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

    fn handle_events(&mut self) -> impl Iterator<Item = RSnesEvent> {
        self.event_pump
            .poll_iter()
            .filter_map(|event: Event| match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => Some(RSnesEvent::Quit),
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    ..
                } => {
                    Some(RSnesEvent::ButtonDown)
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Space),
                    ..
                } => {
                    Some(RSnesEvent::ButtonUp)
                }
                Event::KeyDown {
                    keycode: Some(Keycode::L),
                    ..
                } => match Some("./cputest-basic.sfc".into()) {
                    Some(path) => Some(RSnesEvent::LoadRom { path }),
                    None => None,
                },
                _ => None,
            })
    }

    fn draw_framebuffer(&mut self, framebuffer: Option<&RawFrameBuffer>) -> Result<(), String> {
        use sdl2::pixels::PixelFormatEnum;

        let framebuffer = framebuffer.unwrap_or(&self.framebuffer);

        let texture_creator = self.canvas.texture_creator();

        let mut texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::RGB24,
                Self::SNES_WIDTH as u32,
                Self::SNES_HEIGHT as u32,
            )
            .map_err(|e| e.to_string())?;

        texture
            .update(None, framebuffer, Self::SNES_WIDTH * 3)
            .map_err(|e| e.to_string())?;

        self.canvas.copy(&texture, None, None)?;

        Ok(())
    }

    pub fn update(&mut self, framebuffer: Option<&RawFrameBuffer>) -> impl Iterator<Item = RSnesEvent> + use<'_> {
        self.clear(30, 30, 35);

        match self.draw_framebuffer(framebuffer) {
            Ok(()) => {},
            Err(s) => eprintln!("draw_framebuffer: {s}"),
        }

        self.present();

        self.handle_events() // Handle events after presenting window because it's borrowing mut self
    }
}
