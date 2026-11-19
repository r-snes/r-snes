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
mod save_state;
#[cfg(test)]
mod test_utils;

use crate::{
    audio::GameAudio,
    gui::{Gui, GuiFrameData, RSnesEvent, SlotAction, SnesButton},
    rsnes::{RSnesCore, RSnesEmu},
    save_state::{SaveStateError, slots::GameSaveStates},
};

#[cfg(feature = "cli")]
use clap::Parser;

#[cfg(feature = "plugins")]
use plugins::plugin::Plugin;
use std::{
    fs, io,
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

/// Runs `action` on the game's save states, or gives the error that made them
/// unavailable when the game started.
fn with_save_states<T>(
    save_states: &mut Result<GameSaveStates, SaveStateError>,
    action: impl FnOnce(&mut GameSaveStates) -> Result<T, SaveStateError>,
) -> Result<T, SaveStateError> {
    save_states.as_mut().map_err(|error| error.clone()).and_then(action)
}

/// Shows how a save state action went: a notification for a success or when
/// there is nothing to do, the error popup for a real failure.
fn report(gui: &mut Gui, result: Result<String, SaveStateError>) {
    match result {
        Ok(text) => gui.notify(text),
        Err(
            error @ (SaveStateError::NoSaveState
            | SaveStateError::NothingToUndo
            | SaveStateError::SlotLocked
            | SaveStateError::AllSlotsLocked),
        ) => {
            gui.notify(error.to_string());
        }
        Err(error) => gui.pass_error(Box::new(error)),
    }
}

/// Runs an action chosen in the save state window. Returns the message to
/// show.
fn run_slot_action(
    states: &mut GameSaveStates,
    emu: &mut RSnesEmu,
    action: &SlotAction,
) -> Result<String, SaveStateError> {
    Ok(match *action {
        SlotAction::Load(slot) => {
            states.load_slot(emu, slot)?;
            format!("Loaded slot {slot}")
        }
        SlotAction::Save(slot) => {
            states.save_to(emu, slot)?;
            format!("Saved to slot {slot}")
        }
        SlotAction::Delete(slot) => {
            states.delete(slot)?;
            format!("Deleted slot {slot}")
        }
        SlotAction::Rename(slot, ref name) => {
            states.rename(slot, name)?;
            format!("Renamed slot {slot}")
        }
        SlotAction::SetLocked(slot, locked) => {
            states.set_locked(slot, locked)?;
            let done = if locked { "Locked" } else { "Unlocked" };
            format!("{done} slot {slot}")
        }
        SlotAction::OpenFolder => {
            // The folder only exists after the first save.
            fs::create_dir_all(states.folder())?;
            opener::open(states.folder()).map_err(io::Error::other)?;
            "Opened the save state folder".to_owned()
        }
    })
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

    let mut save_states = GameSaveStates::for_rom(&rom_info.header.title, rsnes.rom_checksum);

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

        // Whether the save state window must show the slots again.
        let mut slots_changed = false;
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

                // Loads keep the paused/running state; the queued sound
                // belongs to the old state, so it is dropped.
                RSnesEvent::SaveState => {
                    let result = with_save_states(&mut save_states, |states| states.save(&mut emu));
                    report(gui, result.map(|slot| format!("Saved to slot {slot}")));
                    slots_changed = true;
                }
                RSnesEvent::LoadLatestState => {
                    let result =
                        with_save_states(&mut save_states, |states| states.load_latest(&mut emu));
                    if result.is_ok() {
                        audio.reset(gui);
                    }
                    report(gui, result.map(|slot| format!("Loaded slot {slot}")));
                    slots_changed = true;
                }
                RSnesEvent::SlotAction(action) => {
                    let result = with_save_states(&mut save_states, |states| {
                        run_slot_action(states, &mut emu, &action)
                    });
                    if result.is_ok() && matches!(action, SlotAction::Load(_)) {
                        audio.reset(gui);
                    }
                    report(gui, result);
                    slots_changed = true;
                }
                RSnesEvent::UndoLoadState => {
                    let result =
                        with_save_states(&mut save_states, |states| states.undo_load(&mut emu));
                    if result.is_ok() {
                        audio.reset(gui);
                    }
                    report(gui, result.map(|()| "Load undone".to_owned()));
                }
                RSnesEvent::ChooseSaveState => {
                    let result = with_save_states(&mut save_states, |states| {
                        // The picker shows the folder as it is now.
                        states.refresh();
                        gui.open_slot_picker(states);
                        Ok(())
                    });
                    if let Err(error) = result {
                        report(gui, Err(error));
                    }
                }

                #[cfg(feature = "plugins")]
                RSnesEvent::RunPluginDefault => {
                    if let Some(p) = emu.plugin_mut() {
                        gui.unwrap_result(p.run_default())
                    }
                }
            }
        }

        if slots_changed && let Ok(states) = &save_states {
            gui.refresh_slot_picker(states);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_state::test_utils::*;

    #[test]
    fn with_save_states_runs_the_action_on_the_states() {
        let (mut snes, states, _root) = setup();
        let mut save_states = Ok(states);

        let slot = with_save_states(&mut save_states, |states| states.save(&mut snes));

        assert_eq!(slot.unwrap(), 1);
        assert_eq!(save_states.unwrap().latest_slot(), Some(1));
    }

    #[test]
    fn with_save_states_reports_why_there_are_none() {
        let mut save_states = Err(SaveStateError::NoHomeFolder);

        let result = with_save_states(&mut save_states, |_| -> Result<(), _> {
            panic!("the action must not run without save states")
        });

        assert!(matches!(result, Err(SaveStateError::NoHomeFolder)));
        // The error stays, for the next action.
        assert!(matches!(save_states, Err(SaveStateError::NoHomeFolder)));
    }

    #[test]
    fn slot_actions_change_the_slot_and_say_what_they_did() {
        let (mut snes, mut states, _root) = setup();
        let mut run = |action| run_slot_action(&mut states, &mut snes, &action).unwrap();

        assert_eq!(run(SlotAction::Save(3)), "Saved to slot 3");
        assert_eq!(run(SlotAction::Load(3)), "Loaded slot 3");
        assert_eq!(run(SlotAction::Rename(3, "boss".into())), "Renamed slot 3");
        assert_eq!(run(SlotAction::SetLocked(3, true)), "Locked slot 3");
        assert_eq!(run(SlotAction::SetLocked(3, false)), "Unlocked slot 3");
        assert_eq!(run(SlotAction::Delete(3)), "Deleted slot 3");

        assert!(is_empty(states.slot(3)));
        assert_eq!(states.settings(3).name, "");
    }

    #[test]
    fn a_refused_slot_action_returns_its_error() {
        let (mut snes, mut states, _root) = setup();
        states.save_to(&mut snes, 2).unwrap();
        states.set_locked(2, true).unwrap();

        let result = run_slot_action(&mut states, &mut snes, &SlotAction::Delete(2));

        assert!(matches!(result, Err(SaveStateError::SlotLocked)));
        assert!(matches!(
            run_slot_action(&mut states, &mut snes, &SlotAction::Load(5)),
            Err(SaveStateError::NoSaveState)
        ));
    }
}
