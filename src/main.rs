mod gui;
mod rsnes;

use crate::{
    gui::{Gui, RSnesEvent},
    rsnes::{RSnesCore, RSnesEmu},
};
#[cfg(feature = "cli")]
use clap::Parser;
#[cfg(feature = "plugins")]
use plugins::plugin::Plugin;
use ppu::constants::SCREEN_HEIGHT;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

fn gui_emu_loop(
    gui: &mut gui::Gui,
    rsnes: RSnesCore,

    #[cfg(feature = "plugins")]
    plugin: Option<Plugin>,
) -> Option<RSnesEvent> {
    let mut frame_nb = 0_u64;
    let exec_start = Instant::now();

    let mut last_instant = Instant::now();
    let mut frame_accum: f64 = 0.0;
    let mut master_cycle_accum: f64 = 0.0;

    let mut emu = cfg_select! {
        feature = "plugins" => RSnesEmu::new_with_plugin(rsnes, plugin).unwrap(),
        _ => RSnesEmu::new(rsnes),
    };

    let closing_ev = 'emu_loop: loop {
        // Get new delta based on current Instant::now()
        let current_instant = Instant::now();
        let delta = current_instant.duration_since(last_instant).as_secs_f64();
        last_instant = current_instant;

        frame_accum += delta;
        master_cycle_accum += delta;

        // sleep until we are due a cycle instead of busy-waiting
        if master_cycle_accum < RSnesCore::MASTER_CYCLE_DURATION {
            // since the frequency of master cycles is orders
            // of magnitude greater than the framerate, we need
            // to sleep for master cycles, not for frames
            std::thread::sleep(Duration::from_secs_f64(
                RSnesCore::MASTER_CYCLE_DURATION - master_cycle_accum,
            ));
        }

        while master_cycle_accum >= RSnesCore::MASTER_CYCLE_DURATION {
            master_cycle_accum -= RSnesCore::MASTER_CYCLE_DURATION;

            cfg_select! {
                feature = "plugins" => emu.update().unwrap(),
                _ => emu.update(),
            }
        }

        // Window update if frame treshold is crossed
        if frame_accum < Gui::FRAME_DURATION {
            continue;
        }
        frame_accum -= Gui::FRAME_DURATION;

        let mut emu_mut = emu.core_mut();

        // temporary: render full PPU frame for each GUI frame
        for y in 0..SCREEN_HEIGHT {
            let RSnesCore {
                ppu, ppu_renderer, ..
            } = &mut *emu_mut;
            ppu_renderer.render_scanline(ppu, y);
            emu_mut.ppu.step_scanline();
        }
        // temporary: toggle VBLANK each rendered frame
        emu_mut.bus.io.rdnmi = !emu_mut.bus.io.rdnmi;

        let events = gui.update(&emu_mut.ppu_renderer.framebuffer);
        drop(emu_mut); // release the RefCell so that we're able to call plugins
        for state_event in events {
            match state_event {
                // RSnesEvent::LoadRom { path } => match rsnes::RSnes::load_rom(&path) {
                //     Ok(emu_) => emu = emu_,
                //     Err(err) => eprintln!("Error loading ROM: {}", err),
                // },
                RSnesEvent::Quit => break 'emu_loop Some(RSnesEvent::Quit),
                RSnesEvent::Close => break 'emu_loop None,
                RSnesEvent::ButtonDown => {
                    let mut emu_mut = emu.core_mut();
                    emu_mut.bus.io.hvbjoy = 0;
                    emu_mut.bus.io.joy1 = !0;
                }
                RSnesEvent::ButtonUp => {
                    let mut emu_mut = emu.core_mut();
                    emu_mut.bus.io.hvbjoy = 0;
                    emu_mut.bus.io.joy1 = 0;
                }

                #[cfg(feature = "plugins")]
                RSnesEvent::RunPluginDefault => {
                    if let Some(p) = emu.plugin_mut() {
                        p.run_default().unwrap();
                    }
                }

                e => println!("ignored event: {e:?}"),
            }
        }
        frame_nb += 1;
    };

    #[cfg(feature = "plugins")]
    if let Some(p) = emu.plugin_mut() {
        p.run_exit().unwrap();
    }

    let time = Instant::now();
    let program_duration = time.duration_since(exec_start).as_secs_f64();
    println!("Game duration : {}", program_duration);
    println!("Frame rate : {}", frame_nb as f64 / program_duration);

    closing_ev
}

fn gui_loop(
    mut rsnes_core: Option<RSnesCore>,

    #[cfg(feature = "plugins")]
    mut plugin: Option<Plugin>,
) -> Result<(), String> {
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
        let ev = match rsnes_core.take() {
            None => Some(gui.wait_for_event()),

            Some(emu) => {
                let ret_ev = cfg_select! {
                    feature = "plugins" => gui_emu_loop(&mut gui, emu, plugin.take()),
                    _ => gui_emu_loop(&mut gui, emu),
                };

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
            RSnesEvent::LoadRom { path } => match rsnes::RSnesCore::load_rom(&path) {
                Ok(some_emu) => rsnes_core = Some(some_emu),
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
    /// A SNES ROM to load at startup
    pub rom: Option<PathBuf>,

    /// A plugin to load at startup, **without any confirmation
    /// for requested permissions**
    #[cfg(feature = "plugins")]
    #[arg(long, value_name = "PLUGIN.lua")]
    pub load_plugin_noconfirm: Option<PathBuf>,
}

fn main() -> Result<(), String> {
    let cli = cfg_select! {
        feature = "cli" => Cli::parse(),
        _ => {{
            if std::env::args().len() != 0 {
                eprintln!("CLI feature disabled at compile time, CLI arguments are ignored");
            }
            Cli::default()
        }}
    };

    let emu = match cli.rom {
        None => None,
        Some(rom_path) => Some(RSnesCore::load_rom(&rom_path).map_err(|e| e.to_string())?),
    };

    cfg_select! {
        feature = "plugins" => gui_loop(
            emu,
            cli.load_plugin_noconfirm.map(|p| Plugin::load_from_file(&p).unwrap())
        ),
        _ => gui_loop(emu)
    }
}
