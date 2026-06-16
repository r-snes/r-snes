mod gui;
mod rsnes;

use crate::{
    gui::{Gui, RSnesEvent},
    rsnes::RSnes,
};
#[cfg(feature = "cli")]
use clap::Parser;
#[cfg(feature = "plugins")]
use plugins::plugin::Plugin;
use ppu::constants::SCREEN_HEIGHT;
#[cfg(feature = "plugins")]
use std::{cell::RefCell, rc::Rc};
use std::{
    ops::DerefMut, path::{Path, PathBuf}, time::{Duration, Instant}
};

fn gui_emu_loop(gui: &mut gui::Gui, mut emu: rsnes::RSnes) -> Option<RSnesEvent> {
    let mut frame_nb = 0_u64;
    let exec_start = Instant::now();

    let mut last_instant = Instant::now();
    let mut frame_accum: f64 = 0.0;
    let mut master_cycle_accum: f64 = 0.0;

    #[cfg(feature = "plugins")]
    let mut plugin = Plugin::load_from_file(Path::new("./plugin.lua")).unwrap();

    for _ in 0..100 {
        emu.update();
    }

    #[cfg(feature = "plugins")]
    let emu_rc = Rc::new(RefCell::new(emu));

    #[cfg(feature = "plugins")]
    {
        RSnes::inject_into_lua(emu_rc.clone(), &mut plugin);
        plugin.run_init().unwrap();
    }

    #[cfg(feature = "plugins")]
    let mut emu = emu_rc.borrow_mut();
    #[cfg(not(feature = "plugins"))]
    let mut emu = &mut emu;

    println!("actual addr is {:?}", emu.cpu.addr_bus());

    let closing_ev = 'emu_loop: loop {
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

        // temporary: render full PPU frame for each GUI frame
        for y in 0..SCREEN_HEIGHT {
            let RSnes { ppu, ppu_renderer, .. } = &mut *emu;
            ppu_renderer.render_scanline(ppu, y);
            emu.ppu.step_scanline();
        }
        // temporary: toggle VBLANK each rendered frame
        emu.bus.io.rdnmi = !emu.bus.io.rdnmi;

        for state_event in gui.update(&emu.ppu_renderer.framebuffer) {
            match state_event {
                // RSnesEvent::LoadRom { path } => match rsnes::RSnes::load_rom(&path) {
                //     Ok(emu_) => emu = emu_,
                //     Err(err) => eprintln!("Error loading ROM: {}", err),
                // },
                RSnesEvent::Quit => break 'emu_loop Some(RSnesEvent::Quit),
                RSnesEvent::Close => break 'emu_loop None,
                RSnesEvent::ButtonDown => {
                    emu.bus.io.hvbjoy = 0;
                    emu.bus.io.joy1 = !0;
                }
                RSnesEvent::ButtonUp => {
                    emu.bus.io.hvbjoy = 0;
                    emu.bus.io.joy1 = 0;
                }
                e => println!("ignored event: {e:?}"),
            }
        }
        frame_nb += 1;
    };

    let time = Instant::now();
    let program_duration = time.duration_since(exec_start).as_secs_f64();
    println!("Game duration : {}", program_duration);
    println!("Frame rate : {}", frame_nb as f64 / program_duration);

    closing_ev
}

fn gui_loop(mut emu: Option<RSnes>) -> Result<(), String> {
    let mut gui = gui::Gui::new()?;
    const DEFAULT_FRAMEBUFFER: &ppu::rendering::RawFramebuffer =
        include_bytes!("../logo_framebuffer.raw");

    gui.draw_framebuffer(DEFAULT_FRAMEBUFFER)?;
    gui.present();

    loop {
        // move out of the `Option` in case it's `Some`
        // so that we can pass by value in the emu loop,
        // guaranteeing that the `RSnes` is destructed when
        // we leave the loop
        let ev = match emu.take() {
            None => Some(gui.wait_for_event()),

            Some(emu) => {
                let ret_ev = gui_emu_loop(&mut gui, emu);

                if ret_ev != Some(RSnesEvent::Quit) {
                    // re-render default framebuffer after game has exited
                    gui.draw_framebuffer(DEFAULT_FRAMEBUFFER)?;
                    gui.present();
                }

                ret_ev
            }
        };
        let Some(ev) = ev else {
            continue;
        };
        match ev {
            RSnesEvent::LoadRom { path } => match rsnes::RSnes::load_rom(&path) {
                Ok(some_emu) => emu = Some(some_emu),
                Err(err) => println!("Error loading ROM: {}", err),
            },
            RSnesEvent::Quit | RSnesEvent::Close => break,
            _ => {}
        }
    }

    Ok(())
}

#[cfg_attr(feature = "cli", derive(Parser))]
#[cfg_attr(feature = "cli", command(about, long_about = None))]
#[derive(Default)]
struct Cli {
    pub rom: Option<PathBuf>,
}

fn main() -> Result<(), String> {
    let cli;
    #[cfg(feature = "cli")]
    {
        cli = Cli::parse();
    }

    #[cfg(not(feature = "cli"))]
    {
        cli = Cli::default();

        if std::env::args().len() != 0 {
            eprintln!("CLI feature disabled at compile time, CLI arguments are ignored");
        }
    }

    let emu = match cli.rom {
        None => None,
        Some(rom_path) => Some(RSnes::load_rom(&rom_path).map_err(|e| e.to_string())?),
    };

    gui_loop(emu)
}
