use crate::rsnes;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

pub struct Gui {
    sdl_ctx: sdl2::Sdl,
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    event_pump: sdl2::EventPump,
    framebuffer: Vec<u8>,
}

impl Gui {
    pub const SNES_WIDTH: usize = 256; // TODO : Remove when GUI linked with PPU
    pub const SNES_HEIGHT: usize = 224; // TODO : Remove when GUI linked with PPU

    pub const FRAME_RATE: u16 = 60;
    pub const FRAME_DURATION: f64 = 1.0 / Self::FRAME_RATE as f64;

    pub fn new() -> Result<Self, String> {
        let sdl_ctx = sdl2::init().unwrap();
        let video_subsystem = sdl_ctx.video().unwrap();

        let window = video_subsystem
            .window("R-SNES", 1920 / 2, 1080 / 2)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        let canvas = window
            .into_canvas()
            .accelerated()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        let event_pump = sdl_ctx.event_pump().unwrap();

        Ok(Gui {
            sdl_ctx,
            canvas,
            event_pump,
            framebuffer: Self::temporary_framebuffer(),
        })
    }

    pub fn temporary_framebuffer() -> Vec<u8> {
        let mut framebuffer = vec![0u8; Self::SNES_WIDTH * Self::SNES_HEIGHT * 4];

        for y in 0..Self::SNES_HEIGHT {
            for x in 0..Self::SNES_WIDTH {
                let pixel_index = y * Self::SNES_WIDTH + x;
                let byte_index = pixel_index * 4;

                let shade = ((x + y) & 0xFF) as u8;

                framebuffer[byte_index + 0] = shade; // B
                framebuffer[byte_index + 1] = shade; // G
                framebuffer[byte_index + 2] = shade; // R
                framebuffer[byte_index + 3] = 255; // A
            }
        }

        framebuffer
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        self.canvas
            .set_draw_color(sdl2::pixels::Color::RGB(r, g, b));
        self.canvas.clear();
    }

    pub fn present(&mut self) {
        self.canvas.present();
    }

    pub fn load_rom() -> Result<rsnes::Rsnes, String> {
        match rfd::FileDialog::new().pick_file() {
            Some(path) => match rsnes::Rsnes::load_rom(&path) {
                Ok(emu) => Ok(emu),
                Err(err) => Err(format!("Error loading ROM: {}", err)),
            },
            None => Err("No file selected".to_string()),
        }
    }

    pub fn handle_events(
        &mut self,
        rsnes_app: &mut Option<rsnes::Rsnes>,
    ) -> Option<sdl2::event::Event> {
        for event in self.event_pump.poll_iter() {
            return match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => Some(Event::Quit { timestamp: 0 }),
                Event::KeyDown {
                    keycode: Some(Keycode::L),
                    ..
                } => {
                    match Self::load_rom() {
                        Ok(emu) => *rsnes_app = Some(emu),
                        Err(err) => println!("{}", err),
                    };
                    return None;
                }
                _ => return None,
            };
        }
        None
    }

    pub fn draw_framebuffer(&mut self) -> Result<(), String> {
        use sdl2::pixels::PixelFormatEnum;

        let texture_creator = self.canvas.texture_creator();

        let mut texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::ARGB8888,
                Self::SNES_WIDTH as u32,
                Self::SNES_HEIGHT as u32,
            )
            .map_err(|e| e.to_string())?;

        texture
            .update(None, &self.framebuffer, Self::SNES_WIDTH * 4)
            .map_err(|e| e.to_string())?;

        self.canvas.copy(&texture, None, None)?;

        Ok(())
    }

    pub fn update(&mut self, rsnes_app: &mut Option<rsnes::Rsnes>) -> Result<(), String> {
        self.clear(30, 30, 35);
        match self.handle_events(rsnes_app) {
            Some(Event::Quit { .. }) => return Err("User quit window".to_string()),
            _ => {}
        }
        let _ = self.draw_framebuffer(); // TODO: Handle error properly
        self.present();

        Ok(())
    }
}
