mod gui;
mod rsnes;

use sdl2::event::Event;
use std::time::Duration;

fn main() -> Result<(), String> {
    let mut gui = gui::Gui::new()?;
    let mut rsnes_app: Option<rsnes::Rsnes> = None;

    'emulation_loop: loop {
        gui.clear(30, 30, 35);
        match gui.handle_events(&mut rsnes_app) {
            Some(Event::Quit { .. }) => break 'emulation_loop,
            _ => {}
        }
        gui.update();
        gui.present();

        // Simple frame limiter for now
        ::std::thread::sleep(Duration::new(0, 1_000_000u32)); // 1_000_000_000u32 / 60 for ~60 FPS
    }

    // TODO : Potential Cleanup or user settings save ?

    Ok(())
}
