mod gui;
mod rsnes;

use crate::{
    gui::{Gui, RSnesEvent},
    rsnes::RSnes,
};
use std::time::Instant;

fn main() -> Result<(), String> {
    let mut gui = gui::Gui::new()?;
    let mut rsnes_app: Option<Box<rsnes::RSnes>> = None;

    // Reference variables
    let mut frame_nb = 0;
    let exec_start = Instant::now();

    // Clock utility variables
    let mut last_instant = Instant::now();
    let mut frame_accum: f64 = 0.0;
    let mut master_cycle_accum: f64 = 0.0;

    'emulation_loop: loop {
        // Get new delta based on current Instant::now()
        let current_instant = Instant::now();
        let delta = current_instant.duration_since(last_instant).as_secs_f64();
        last_instant = current_instant;

        frame_accum += delta;

        // Emulation update if emulator exists and if master_cycle duration treshold is crossed
        match rsnes_app {
            Some(ref mut app) => {
                master_cycle_accum += delta;

                while master_cycle_accum >= RSnes::MASTER_CYCLE_DURATION {
                    master_cycle_accum -= RSnes::MASTER_CYCLE_DURATION;
                    app.update();
                }
            }
            None => {}
        }

        // Window update if frame treshold is crossed
        if frame_accum >= Gui::FRAME_DURATION {
            frame_accum -= Gui::FRAME_DURATION;

            for state_event in gui.update() {
                match state_event {
                    RSnesEvent::LoadRom { path } => match rsnes::RSnes::load_rom(&path) {
                        Ok(emu) => rsnes_app = Some(Box::new(emu)),
                        Err(err) => println!("Error loading ROM: {}", err),
                    },
                    RSnesEvent::Quit => break 'emulation_loop,
                }
            }
            frame_nb += 1;
        }
    }

    // TODO : Potential Cleanup or user settings save ?

    // Print of the window frame rate and program duration
    let time = Instant::now();
    let program_duration = time.duration_since(exec_start).as_secs_f64();
    println!("Program duration : {}", program_duration);
    println!("Frame rate : {}", frame_nb as f64 / program_duration);

    Ok(())
}
