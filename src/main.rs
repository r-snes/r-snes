mod gui;
mod rsnes;

use crate::{gui::Gui, rsnes::Rsnes};
use std::time::Instant;

fn main() -> Result<(), String> {
    let gui = gui::Gui::new()?;
    let mut gui = Box::new(gui);
    let mut rsnes_app: Option<Box<rsnes::Rsnes>> = None;

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

        // Emulation update if master_cycle duration treshold is crossed
        match rsnes_app {
            Some(ref mut app) => {
                master_cycle_accum += delta;

                // In the future, check if this 'while' doesn't restrain the program execution too much
                while master_cycle_accum >= Rsnes::MASTER_CYCLE_DURATION {
                    master_cycle_accum -= Rsnes::MASTER_CYCLE_DURATION
                }
                app.update();
            }
            None => {}
        }

        // Widnow update if frame treshold is crossed
        if frame_accum >= Gui::FRAME_DURATION {
            frame_accum -= Gui::FRAME_DURATION;

            match gui.update(&mut rsnes_app) {
                Err(_) => break 'emulation_loop,
                Ok(_) => {}
            }
            frame_nb += 1;
        }

        // Deactivated sleep for now
        // ::std::thread::sleep(Duration::new(0, 1_000_000u32));
    }

    // TODO : Potential Cleanup or user settings save ?

    // Print of the window frame rate
    let time = Instant::now();
    let program_duration = time.duration_since(exec_start).as_secs_f64();
    println!("Program duration : {}", program_duration);
    println!("Frame rate : {}", frame_nb as f64 / program_duration);

    Ok(())
}
