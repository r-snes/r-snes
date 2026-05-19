mod gui;
mod rsnes;

use crate::{
    gui::{Gui, RSnesEvent},
    rsnes::RSnes,
};
use std::time::{Duration, Instant};

fn gui_emu_loop(gui: &mut gui::Gui, emu: &mut rsnes::RSnes) {
    let mut frame_nb = 0_u64;
    let exec_start = Instant::now();

    let mut last_instant = Instant::now();
    let mut frame_accum: f64 = 0.0;
    let mut master_cycle_accum: f64 = 0.0;

    'emu_loop: loop {
        // Get new delta based on current Instant::now()
        let current_instant = Instant::now();
        let delta = current_instant.duration_since(last_instant).as_secs_f64();
        last_instant = current_instant;

        frame_accum += delta;
        master_cycle_accum += delta;

        // sleep until we are due a cycle instead of busy-waiting
        if master_cycle_accum < RSnes::MASTER_CYCLE_DURATION {
            // since the frequency of master cycles is orders
            // of magnitude greater than the framerate, we need
            // to sleep for master cycles, not for frames
            std::thread::sleep(Duration::from_secs_f64(
                RSnes::MASTER_CYCLE_DURATION - master_cycle_accum,
            ));
        }

        while master_cycle_accum >= RSnes::MASTER_CYCLE_DURATION {
            master_cycle_accum -= RSnes::MASTER_CYCLE_DURATION;
            emu.update();
        }

        // Window update if frame treshold is crossed
        if frame_accum < Gui::FRAME_DURATION {
            continue;
        }

        frame_accum -= Gui::FRAME_DURATION;

        for state_event in gui.update() {
            match state_event {
                RSnesEvent::LoadRom { path } => match rsnes::RSnes::load_rom(&path) {
                    Ok(emu_) => *emu = emu_,
                    Err(err) => eprintln!("Error loading ROM: {}", err),
                },
                RSnesEvent::Quit => break 'emu_loop,
            }
        }
        frame_nb += 1;
    }

    let time = Instant::now();
    let program_duration = time.duration_since(exec_start).as_secs_f64();
    println!("Game duration : {}", program_duration);
    println!("Frame rate : {}", frame_nb as f64 / program_duration);
}

fn gui_loop() -> Result<(), String> {
    let mut gui = gui::Gui::new()?;
    let mut emu: Option<rsnes::RSnes> = None;

    let _ = gui.update(); // todo: potentially handle events returned by this?

    loop {
        if let Some(emu) = &mut emu {
            gui_emu_loop(&mut gui, emu);

            // then reset framebuffer to logo (once game is closed)
        };
        match gui.wait_for_event() {
            RSnesEvent::LoadRom { path } => match rsnes::RSnes::load_rom(&path) {
                Ok(some_emu) => emu = Some(some_emu),
                Err(err) => println!("Error loading ROM: {}", err),
            },
            RSnesEvent::Quit => break,
        }
    }

    Ok(())
}

fn main() -> Result<(), String> {
    // TODO: CLI arg parsing: i.e. directly run with a rom passed as arg
    gui_loop()
}
