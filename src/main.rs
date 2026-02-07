mod gui;
mod rsnes;

use crate::gui::Gui;
use std::time::Instant;

fn main() -> Result<(), String> {
    let mut gui = gui::Gui::new()?;
    let mut rsnes_app: Option<rsnes::Rsnes> = None;

    // Reference variables
    let mut frame_nb = 0;
    let exec_start = Instant::now();

    // Clock utility variables
    let mut last_instant = Instant::now();
    let mut frame_accum: f64 = 0.0;

    'emulation_loop: loop {
        // rsnes_app.update_master_cycles();
        // rsnes_app.update();
        // rsnes_app.get_time_to_next_master_cycle();

        // Get new delta based on current Instant::now()
        let current_instant = Instant::now();
        let delta = current_instant.duration_since(last_instant).as_secs_f64();
        last_instant = current_instant;

        frame_accum += delta;

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

    // Print of the frame rate
    let time = Instant::now();
    let program_duration = time.duration_since(exec_start).as_secs_f64();
    println!("Program duration : {}", program_duration);
    println!("Frame rate : {}", frame_nb as f64 / program_duration);

    Ok(())
}
