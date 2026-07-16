mod gui;
mod rsnes;

use crate::{
    gui::{Gui, GuiFrameData, RSnesEvent},
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

#[cfg(feature = "plugins")]
type IdleExit = (RSnesEvent, Option<Plugin>);
#[cfg(not(feature = "plugins"))]
type IdleExit = RSnesEvent;

fn gui_emu_loop(
    gui: &mut gui::Gui,
    rsnes: RSnesCore,

    #[cfg(feature = "plugins")] plugin: Option<Plugin>,
) -> Option<RSnesEvent> {
    let mut frame_nb = 0_u64;
    let exec_start = Instant::now();

    let mut last_instant = Instant::now();
    let mut frame_accum: f64 = 0.0;
    let mut master_cycle_accum: f64 = 0.0;

    // Snapshot the ROM header once — it never changes while the ROM is loaded,
    // so there's no reason to rebuild it every frame.
    let rom_info = rsnes.rom_info();
    let title = rom_info.header.title.trim();
    gui.set_rom_title(if title.is_empty() { None } else { Some(title) });
    #[cfg(feature = "plugins")]
    gui.set_plugin_loading(false);

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
                feature = "plugins" => gui.unwrap_result(emu.update()),
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

        let events = gui.update(
            &emu_mut.ppu_renderer.framebuffer,
            GuiFrameData {
                rom_info: Some(&rom_info),
            },
        );
        drop(emu_mut);

        for state_event in events {
            match state_event {
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
                        gui.unwrap_result(p.run_default())
                    }
                }

                e => println!("ignored event: {e:?}"),
            }
        }
        frame_nb += 1;
    };

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

/// The no-ROM idle state: render the logo screen (plus any open overlays)
/// at the normal frame rate, and after a few seconds start the waiting
/// music — a real SPC700 driver uploaded into a standalone APU over the
/// IPL boot protocol (see `apu::jingle`).
///
/// Returns the first event the outer loop cares about. Input-only events
/// (buttons, plugin actions) are meaningless without a game and are
/// swallowed here.
fn gui_idle_loop(gui: &mut Gui, default_framebuffer: &ppu::rendering::RawFramebuffer) -> IdleExit {
    use apu::jingle::IdleJingle;

    /// Silence on the logo screen before the music starts.
    const JINGLE_DELAY: Duration = Duration::from_secs(3);
    /// Keep ~256 ms queued on the device: far more than one ~16 ms frame
    /// interval, so scheduling hiccups never underrun, while staying
    /// small enough to stop quickly when the user loads a ROM.
    const TARGET_BUFFERED_FRAMES: u32 = 8_192;
    /// Generation chunk: 32 ms of audio per `generate` call.
    const CHUNK_FRAMES: usize = 1_024;

    gui.set_rom_title(None);
    #[cfg(feature = "plugins")]
    gui.set_plugin_loading(true);

    let idle_start = Instant::now();
    let mut jingle: Option<IdleJingle> = None;
    let mut jingle_failed = false;

    let ev = loop {
        let frame_start = Instant::now();

        // Render the logo + overlays; this also polls input (routed
        // through egui first). No ROM loaded, so no rom_info to show —
        // the overlay will say so if the user opens it.
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

        // Boot the jingle APU once the delay has passed. A boot failure
        // is loudly logged but non-fatal: the emulator just stays silent.
        if jingle.is_none() && !jingle_failed && idle_start.elapsed() >= JINGLE_DELAY {
            match IdleJingle::new() {
                Ok(j) => {
                    jingle = Some(j);
                    gui.audio_play();
                }
                Err(e) => {
                    eprintln!("waiting music disabled: {e}");
                    jingle_failed = true;
                }
            }
        }

        // Keep the audio queue topped up.
        let mut feed_error = None;
        if let Some(j) = jingle.as_mut() {
            while gui.audio_buffered_frames() < TARGET_BUFFERED_FRAMES {
                let samples = j.generate(CHUNK_FRAMES);
                if let Err(e) = gui.audio_queue_samples(&samples) {
                    feed_error = Some(e);
                    break;
                }
            }
        }
        if let Some(e) = feed_error {
            eprintln!("waiting music disabled: {e}");
            gui.audio_stop();
            jingle = None;
            jingle_failed = true;
        }

        // ~256 ms of queued audio makes sleeping out the rest of the
        // frame safe for the jingle.
        let elapsed = frame_start.elapsed().as_secs_f64();
        if elapsed < Gui::FRAME_DURATION {
            std::thread::sleep(Duration::from_secs_f64(Gui::FRAME_DURATION - elapsed));
        }
    };

    // Carry any granted plugin back to `gui_loop` alongside the exit event.
    gui.audio_stop();
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
        include_bytes!("../logo_framebuffer.raw");

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
                    },
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
        _ => {{
            // args() always contains at least the program name, so only
            // warn when the user actually passed extra arguments
            if std::env::args().len() > 1 {
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
