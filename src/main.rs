use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::time::Duration;

fn main() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;

    let window = video
        .window("Rust Emulator", 800, 600)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .build()
        .map_err(|e| e.to_string())?;

    let mut event_pump = sdl.event_pump()?;

    'running: loop {
        // ---- Event handling (non-blocking) ----
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        // ---- Emulator work would go here ----
        // cpu.step();
        // ppu.step();

        // ---- Clear screen (example draw) ----
        canvas.set_draw_color(Color::RGB(30, 30, 35));
        canvas.clear();
        canvas.present();

        // Prevent 100% CPU spin (optional)
        std::thread::sleep(Duration::from_millis(1));
    }

    Ok(())
}
