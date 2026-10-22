// #![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#![cfg_attr(
    all(windows, not(debug_assertions), not(feature = "console")),
    windows_subsystem = "windows"
)]
//! This is the main entry point for the R-SNES emulator.
//! It contains the main loop, which handles GUI events,
//! emulation updates, and audio output.

mod audio;
mod dma;
mod gui;
mod rsnes;

#[cfg(test)]
mod test_utils;

use crate::{
    audio::GameAudio,
    gui::{Gui, GuiFrameData, RSnesEvent, SnesButton},
    rsnes::{RSnesCore, RSnesEmu},
};

#[cfg(feature = "cli")]
use clap::Parser;

#[cfg(feature = "plugins")]
use plugins::plugin::Plugin;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[cfg(feature = "plugins")]
type IdleExit = (RSnesEvent, Option<Plugin>);
#[cfg(not(feature = "plugins"))]
type IdleExit = RSnesEvent;

/// Update the host-side port-1 controller state for a single button. Goes through
/// `core_mut()` so it works whether the core is held directly or behind an
/// `Rc<RefCell<...>>` (plugins feature).
/// The emulated pad only sees it when the game latches the controllers.
fn set_button(emu: &mut RSnesEmu, button: SnesButton, pressed: bool) {
    emu.core_mut().joypads[0].set(button, pressed);
}

fn gui_emu_loop(
    gui: &mut gui::Gui,
    rsnes: RSnesCore,

    #[cfg(feature = "plugins")] plugin: Option<Plugin>,
) -> Option<RSnesEvent> {
    let mut frame_nb = 0_u64;
    let exec_start = Instant::now();

    // Close any overlays that might have been left open from the previous window.
    gui.close_overlays();

    // Snapshot the ROM header once — it never changes while the ROM is loaded,
    // so there's no reason to rebuild it every frame.
    let rom_info = rsnes.rom_info();
    let title = rom_info.header.title.trim();
    gui.set_rom_title(if title.is_empty() { None } else { Some(title) });

    let mut emu = cfg_select! {
        feature = "plugins" => match RSnesEmu::new_with_plugin(rsnes, plugin) {
            Ok(emu) => emu,
            Err(e) => {
                gui.pass_error(Box::new(e));
                return None;
            }
        },
        _ => RSnesEmu::new(rsnes),
    };

    let mut audio = GameAudio::new();
    // Paused by the user.
    let mut paused = false;

    let closing_ev = 'emu_loop: loop {
        let frame_start = Instant::now();

        // Emulation also stands still while a menu is open, so the game
        // doesn't carry on unattended. The last frame stays on screen, and
        // the device is paused with its queue intact, so resuming picks up
        // exactly where the sound left off.
        let running = !paused && !gui.menu_open();
        audio.set_running(gui, running);

        // run master cycles until the PPU completes a frame
        let curr_frame = emu.core_mut().ppu.frame;
        while running && emu.core_mut().ppu.frame == curr_frame {
            cfg_select! {
                feature = "plugins" => gui.unwrap_result(emu.update()),
                _ => emu.update(),
            }
        }

        if running {
            audio.queue_frame(gui, &mut emu);
        }

        #[cfg(feature = "plugins")]
        let plugin_loaded = emu.plugin_mut().is_some();
        let emu_mut = emu.core_mut();
        let events = gui.update(
            emu_mut.ppu_renderer.presented(),
            GuiFrameData {
                rom_info: Some(&rom_info),
                paused,
                #[cfg(feature = "plugins")]
                plugin_loaded,
            },
        );
        drop(emu_mut);

        for state_event in events {
            match state_event {
                RSnesEvent::Quit => break 'emu_loop Some(RSnesEvent::Quit),
                RSnesEvent::Close => break 'emu_loop None,
                RSnesEvent::LoadRom { path } => break 'emu_loop Some(RSnesEvent::LoadRom { path }),
                RSnesEvent::TogglePause => paused = !paused,
                RSnesEvent::Reset => {
                    emu.core_mut().reset();
                    audio.reset(gui);
                }
                RSnesEvent::ButtonDown(button) => {
                    set_button(&mut emu, button, true);
                }
                RSnesEvent::ButtonUp(button) => {
                    set_button(&mut emu, button, false);
                }

                #[cfg(feature = "plugins")]
                RSnesEvent::RunPluginDefault => {
                    if let Some(p) = emu.plugin_mut() {
                        gui.unwrap_result(p.run_default())
                    }
                }
            }
        }

        let sleep = audio.pacing_sleep(gui, running, frame_start);
        if !sleep.is_zero() {
            std::thread::sleep(sleep);
        }
        frame_nb += 1;
    };

    // Leave no game overlays behind.
    gui.close_overlays();

    audio.stop(gui);

    #[cfg(feature = "plugins")]
    if let Some(p) = emu.plugin_mut() {
        gui.unwrap_result(p.run_exit())
    }

    let time = Instant::now();
    let program_duration = time.duration_since(exec_start).as_secs_f64();
    println!("Game duration : {}", program_duration);
    println!("Frame rate : {}", frame_nb as f64 / program_duration);

    closing_ev
}

/// The no-ROM idle state: render the logo screen (plus any open overlays).
///
/// Returns the first event the outer loop cares about. Input-only events
/// (buttons, plugin actions) are meaningless without a game and are
/// swallowed here.
fn gui_idle_loop(gui: &mut Gui, default_framebuffer: &ppu::rendering::RawFramebuffer) -> IdleExit {
    gui.set_rom_title(None);

    let ev = loop {
        let frame_start = Instant::now();

        // Render the logo + overlays; this also polls input (routed through egui first).
        let events = gui.update(default_framebuffer, GuiFrameData::default());

        let mut terminal = None;
        for event in events {
            match event {
                RSnesEvent::Quit => terminal = Some(RSnesEvent::Quit),
                RSnesEvent::Close => terminal = Some(RSnesEvent::Close),
                RSnesEvent::LoadRom { path } => terminal = Some(RSnesEvent::LoadRom { path }),
                _ => {}
            }
        }
        if let Some(ev) = terminal {
            break ev;
        }

        // Sleep out the rest of the frame.
        let elapsed = frame_start.elapsed().as_secs_f64();
        if elapsed < Gui::FRAME_DURATION {
            std::thread::sleep(Duration::from_secs_f64(Gui::FRAME_DURATION - elapsed));
        }
    };

    // Carry any granted plugin back to `gui_loop` alongside the exit event.
    cfg_select! {
        feature = "plugins" => (ev, gui.take_granted_plugin()),
        _ => ev,
    }
}

fn gui_loop(
    mut rsnes_core: Option<RSnesCore>,

    #[cfg(feature = "plugins")] mut plugin: Option<Plugin>,
) -> Result<(), String> {
    let mut gui = gui::Gui::new()?;
    const DEFAULT_FRAMEBUFFER: &ppu::rendering::RawFramebuffer =
        include_bytes!("../assets/logo_framebuffer.raw");

    loop {
        // move out of the `Option` in case it's `Some`
        // so that we can pass by value in the emu loop,
        // guaranteeing that the `RSnes` is destructed when
        // we leave the loop
        let ev = match rsnes_core.take() {
            None => {
                let idle_exit = gui_idle_loop(&mut gui, DEFAULT_FRAMEBUFFER);
                cfg_select! {
                    feature = "plugins" => {
                        let (ev, new_plugin) = idle_exit;
                        // A freshly granted plugin replaces whatever we were holding;
                        // neither has been injected/init'd yet, so nothing to run_exit.
                        if new_plugin.is_some() {
                            plugin = new_plugin;
                        }
                        ev
                    }
                    _ => idle_exit,
                }
            }

            Some(emu) => {
                let ret_ev = cfg_select! {
                    feature = "plugins" => gui_emu_loop(&mut gui, emu, plugin.take()),
                    _ => gui_emu_loop(&mut gui, emu),
                };

                match ret_ev {
                    Some(ev) => ev,
                    None => continue,
                }
            }
        };

        match ev {
            RSnesEvent::LoadRom { path } => match rsnes::RSnesCore::load_rom(&path) {
                Ok(some_emu) => rsnes_core = Some(some_emu),
                Err(err) => gui.pass_error(err),
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
        _ => {
            {
                // args() always contains at least the program name, so only
                // warn when the user actually passed extra arguments
                if std::env::args().len() > 1 {
                    eprintln!("CLI feature disabled at compile time, CLI arguments are ignored");
                }
                Cli::default()
            }
        }
    };

    let emu = match cli.rom {
        None => None,
        Some(rom_path) => Some(RSnesCore::load_rom(&rom_path).map_err(|e| e.to_string())?),
    };

    cfg_select! {
        feature = "plugins" => gui_loop(
            emu,
            cli.load_plugin_noconfirm
                .map(|p| Plugin::load_from_file(&p).unwrap()),
        ),
        _ => gui_loop(emu),
    }
}
