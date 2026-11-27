pub mod command;
pub mod state;
#[cfg(test)]
mod test_utils;
pub mod widgets;

use std::error::Error;
#[cfg(not(tarpaulin_include))]
use std::io::Write;
#[cfg(not(tarpaulin_include))]
use std::path::PathBuf;

use egui_sdl2::canvas::EguiCanvas;
use egui_sdl2::{egui, sdl2};
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::controller::{Button as ControllerButton, GameController};
use sdl2::event::{Event as SdlEvent, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use ppu::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};

#[cfg(not(tarpaulin_include))]
use crate::config::{ConfigLoadError, RSnesConfig};
#[cfg(not(tarpaulin_include))]
use crate::gui::widgets::config::ConfigUiAction;
use crate::rsnes::RomInfo;
use crate::save_state::slots::{GameSaveStates, SLOT_COUNT, Slot};
use command::Command;
use state::{GuiState, Notification, PickerContent, PickerEntry, SaveStatePicker};

#[cfg(feature = "plugins")]
use plugins::plugin::Plugin;
#[cfg(feature = "plugins")]
use state::PendingPlugin;

pub struct Gui {
    _sdl_ctx: sdl2::Sdl,
    /// The window's canvas. The game and egui are both drawn on it.
    canvas: Canvas<Window>,
    egui_canvas: EguiCanvas,
    event_pump: sdl2::EventPump,

    /// Host audio output. A push-model queue fed with interleaved stereo
    /// i16 at the DSP's native 32 kHz (`Apu::drain_samples` format), so no
    /// resampling or format conversion happens anywhere in between.
    /// Opened paused; call `audio_play` to start output.
    audio_queue: AudioQueue<i16>,

    framebuffer_texture: Option<Texture>,
    /// Persistent overlay state — survives across frames.
    state: GuiState,

    _controller_subsystem: sdl2::GameControllerSubsystem,
    controller: Option<GameController>,
    /// Which stick directions are currently held past the deadzone.
    stick_dirs: StickDirs,
}

/// Everything the GUI might need to display, handed in fresh each frame.
///
/// This is the *only* channel through which `main.rs` feeds the GUI.
/// Add a field here when a new overlay needs new data; `main.rs` never
/// touches egui directly.
#[derive(Default)]
pub struct GuiFrameData<'a> {
    /// The running ROM, or `None` on the idle screen.
    pub rom_info: Option<&'a RomInfo>,

    /// Whether the user paused emulation.
    pub paused: bool,

    /// Whether a plugin is running alongside the ROM.
    #[cfg(feature = "plugins")]
    pub plugin_loaded: bool,
}

/// A button on a SNES controller.
///
/// Each button maps to a fixed bit in the JOY1 auto-read register
/// (`$4218`/`$4219`) via [`SnesButton::mask`]. The GUI produces these in
/// response to key presses; the core stores the layout, so the GUI never
/// needs to know the hardware bit order.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum SnesButton {
    R = 4,
    L,
    X,
    A,
    Right,
    Left,
    Down,
    Up,
    Start,
    Select,
    Y,
    B,
}

impl SnesButton {
    /// The button's bit within the 16-bit JOY1 register.
    ///
    /// Layout, from bit 15 down to bit 4:
    /// B, Y, Select, Start, Up, Down, Left, Right, A, X, L, R.
    /// The low 4 bits are unused on a standard controller.
    pub fn mask(self) -> u16 {
        1 << self as u16
    }
}

/// Latched state of the left analog stick's four cardinal directions,
/// so axis-motion events can be turned into press/release edges.
#[derive(Default, Clone, Copy)]
struct StickDirs {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

#[derive(PartialEq, Eq, Debug)]
pub enum RSnesEvent {
    /// Load a new ROM, showing a file picker (closes current game)
    LoadRom { path: PathBuf },

    /// Close the currently open game (or quit if no game open)
    Close,

    /// Quit the emulator program altogether
    Quit,

    /// Reset the currently loaded game
    Reset,

    /// Pause or resume emulation
    TogglePause,

    /// A key mapped to an emulated button has been pressed
    ButtonDown(SnesButton),

    /// A key mapped to an emulated button has been released
    ButtonUp(SnesButton),

    /// Save the game to the first free save slot, or over the oldest one
    QuickSaveState,

    /// Load the most recent save slot
    LoadLatestState,

    /// Open the save state window
    ChooseSaveState,

    /// An action chosen in the save state window
    SlotAction(SlotAction),

    /// Go back to the state before the last load
    UndoLoadState,

    /// Run the `default` action of a plugin (if a plugin is loaded, and if it defines one)
    #[cfg(feature = "plugins")]
    RunPluginDefault,
}

/// An action chosen in the save state window. Slots are numbered from 1.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum SlotAction {
    /// Load a slot.
    Load(usize),
    /// Save the console to a slot, over what it holds.
    Save(usize),
    /// Delete a slot.
    Delete(usize),
    /// Name a slot (an empty name removes it).
    Rename(usize, String),
    /// Lock or unlock a slot.
    SetLocked(usize, bool),
    /// Open the game's save state folder in the file manager.
    OpenFolder,
}

/// One slot of `states`, as the save state window shows it. The saved
/// screen is turned into a texture here.
fn picker_entry(ctx: &egui::Context, states: &GameSaveStates, slot: usize) -> PickerEntry {
    let content = match states.slot(slot) {
        Slot::Empty => PickerContent::Empty,
        Slot::Saved {
            created_at,
            screen,
            file_size,
        } => PickerContent::Saved {
            created_at: *created_at,
            file_size: *file_size,
            screen: ctx.load_texture(
                format!("save state slot {slot}"),
                egui::ColorImage::from_rgb([SCREEN_WIDTH, SCREEN_HEIGHT], &screen[..]),
                // Sharp pixels when the thumbnail is larger than the SNES
                // screen, smooth ones when it is smaller.
                egui::TextureOptions {
                    magnification: egui::TextureFilter::Nearest,
                    ..egui::TextureOptions::LINEAR
                },
            ),
        },
        Slot::Unreadable {
            modified_at,
            reason,
        } => PickerContent::Unreadable {
            modified_at: *modified_at,
            reason: reason.clone(),
        },
    };
    let settings = states.settings(slot);
    PickerEntry {
        slot,
        content,
        name: settings.name.clone(),
        locked: settings.locked,
        latest: states.latest_slot() == Some(slot),
    }
}

/// Where to draw the game in a `width` x `height` drawable whose top
/// `menu_bar` pixel rows belong to the menu bar
///
/// The result never overlaps the bar. `None` when no space is left
/// (e.g. a minimised window).
fn game_rect(width: u32, height: u32, menu_bar: u32) -> Option<Rect> {
    let avail_height = height.checked_sub(menu_bar)?;
    if width == 0 || avail_height == 0 {
        return None;
    }

    let (snes_w, snes_h) = (SCREEN_WIDTH as f64, SCREEN_HEIGHT as f64);
    let scale = f64::min(width as f64 / snes_w, avail_height as f64 / snes_h);
    let w = ((snes_w * scale).round() as u32).clamp(1, width);
    let h = ((snes_h * scale).round() as u32).clamp(1, avail_height);

    let x = (width - w) / 2;
    let y = menu_bar + (avail_height - h) / 2;
    Some(Rect::new(x as i32, y as i32, w, h))
}

#[cfg(not(tarpaulin_include))]
impl Gui {
    pub const FRAME_RATE: u16 = 60;
    pub const FRAME_DURATION: f64 = 1.0 / Self::FRAME_RATE as f64;
    const STICK_DEADZONE: i16 = 13_000; // Deadzone for the analog stick, ~40% of the i16 range
    const MENU_BAR_HEIGHT: u32 = 23; // Height of the menu bar with egui's default style (22), plus 1 for its anti-aliased bottom edge, in window coordinates.
    /// Size of the game area when R-SNES opens: the SNES screen at twice
    /// its size. In window coordinates, which egui uses as points.
    pub const START_GAME_AREA: egui::Vec2 =
        egui::vec2(SCREEN_WIDTH as f32 * 2.0, SCREEN_HEIGHT as f32 * 2.0);
    /// Size of the game area in the smallest window: the SNES screen.
    pub const MIN_GAME_AREA: egui::Vec2 = egui::vec2(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);

    pub fn new(
        config_load: Result<RSnesConfig, ConfigLoadError>,
        config_path: Option<PathBuf>,
    ) -> Result<Self, String> {
        let sdl_ctx = sdl2::init()?;
        let video_subsystem = sdl_ctx.video()?;

        // Sizes are in screen coordinates, not pixels (see `SDL_CreateWindow`).
        let mut window = video_subsystem
            .window(
                "R-SNES",
                Self::START_GAME_AREA.x as u32,
                Self::START_GAME_AREA.y as u32 + Self::MENU_BAR_HEIGHT,
            )
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())?;
        window
            .set_minimum_size(
                Self::MIN_GAME_AREA.x as u32,
                Self::MIN_GAME_AREA.y as u32 + Self::MENU_BAR_HEIGHT,
            )
            .map_err(|e| e.to_string())?;

        let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
        let egui_canvas = EguiCanvas::new(&canvas);
        let event_pump = sdl_ctx.event_pump()?;

        let audio_subsystem = sdl_ctx.audio()?;
        let desired = AudioSpecDesired {
            freq: Some(32_000), // native S-DSP output rate
            channels: Some(2),  // stereo, interleaved L R
            samples: Some(512), // device buffer: 16 ms — latency vs. underrun
        };
        // `None` = default output device. SDL may hand back a different
        // spec than desired; AudioQueue<i16> converts as needed, so the
        // queue always accepts our 32 kHz stereo i16 regardless.
        let audio_queue = audio_subsystem.open_queue::<i16, _>(None, &desired)?;

        // Controller subsystem. `set_event_state(true)` is required for SDL  to actually push button/axis events
        // into the pump, without it the pad opens fine but never emits anything.
        let controller_subsystem = sdl_ctx.game_controller()?;
        controller_subsystem.set_event_state(true);
        let controller = (0..controller_subsystem.num_joysticks().unwrap_or(0))
            .find_map(|i| controller_subsystem.open(i).ok());
        println!(
            "Controller: {} joystick(s) found, opened: {}",
            controller_subsystem.num_joysticks().unwrap_or(0),
            controller.as_ref().map_or("none", |c| c.name().leak())
        );

        Ok(Gui {
            _sdl_ctx: sdl_ctx,
            canvas,
            egui_canvas,
            event_pump,
            audio_queue,
            framebuffer_texture: None,
            state: GuiState::new(config_load, config_path),
            _controller_subsystem: controller_subsystem,
            controller,
            stick_dirs: StickDirs::default(),
        })
    }

    pub fn config(&self) -> &RSnesConfig {
        &self.state.config
    }

    /// Whether a menu (or any other egui popup) is open.
    fn popup_open(&self) -> bool {
        egui::Popup::is_any_open(&self.egui_canvas.ctx)
    }

    /// Whether a menu or a window is open.
    pub fn menu_open(&self) -> bool {
        self.popup_open() || self.state.any_open()
    }

    /// Opens the save state window.
    pub fn open_save_state_picker(&mut self, states: &GameSaveStates) {
        let ctx = &self.egui_canvas.ctx;
        let entries = (1..=SLOT_COUNT).map(|slot| picker_entry(ctx, states, slot)).collect();
        let selected = match &self.state.save_state_picker {
            Some(picker) => picker.selected,
            None => states.latest_slot().map_or(0, |slot| slot - 1),
        };
        self.state.save_state_picker = Some(SaveStatePicker::new(entries, selected));
    }

    /// Shows the slots of `states` again in the save state window, if it is
    /// open.
    pub fn refresh_save_state_picker(&mut self, states: &GameSaveStates) {
        if self.state.save_state_picker.is_some() {
            self.open_save_state_picker(states);
        }
    }

    /// Shows a short message over the game for a moment.
    pub fn notify(&mut self, text: impl Into<String>) {
        self.state.notification = Some(Notification::new(text.into()));
    }

    /// Returns whether theres is a perm request open
    #[cfg(feature = "plugins")]
    pub fn pending_perm_request(&self) -> bool {
        self.state.pending_plugin.is_some()
    }

    /// Returns whether theres is an error popup open
    pub fn pending_error_popup(&self) -> bool {
        self.state.error_popup.is_some()
    }

    /// Returns whether the save state picker is open
    pub fn save_state_picker_open(&self) -> bool {
        self.state.save_state_picker.is_some()
    }

    /// Returns whether the config info window is open
    pub fn config_info_open(&self) -> bool {
        self.state.config_ui.is_some()
    }

    /// Whether the ROM info window is open
    pub fn rom_info_open(&self) -> bool {
        self.state.show_rom_info
    }

    /// Sets the SDL2 window title. Pass `None` to reset to the default.
    pub fn set_rom_title(&mut self, title: Option<&str>) {
        let window_title = match title {
            Some(t) => format!("R-SNES - {t}"),
            None => "R-SNES".to_string(),
        };

        // Ignore the error: a failed title update is cosmetic, not worth
        // propagating up through the emu loop.
        let _ = self.canvas.window_mut().set_title(&window_title);
    }

    /// Closes every open overlay. Called when a game starts or stops, so no
    /// window outlives the state it belongs to (e.g. ROM information with no
    /// ROM loaded, where its toggle is disabled and couldn't close it).
    pub fn close_overlays(&mut self) {
        self.state.close_all();
    }

    /// Takes the plugin the user granted this idle session, if any, and
    /// abandons any still-undecided prompt. Called when the idle loop exits.
    #[cfg(feature = "plugins")]
    pub fn take_granted_plugin(&mut self) -> Option<Plugin> {
        self.state.pending_plugin = None;
        self.state.granted_plugin.take()
    }

    /// Opens a .lua file picker; on success the plugin becomes pending (its
    /// permission prompt opens next frame), on failure it surfaces as an error.
    #[cfg(feature = "plugins")]
    fn load_plugin_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Lua plugin", &["lua"]).pick_file()
        else {
            return;
        };

        match Plugin::load_from_file(&path) {
            Ok(plugin) => self.state.pending_plugin = Some(PendingPlugin::new(plugin)),
            Err(e) => self.pass_error(Box::new(e)),
        }
    }

    /// Runs a command picked from the menu bar or triggered by its shortcut.
    /// Called outside the egui pass, since some commands open blocking file
    /// dialogs.
    fn execute(&mut self, cmd: Command, data: &GuiFrameData, out: &mut Vec<RSnesEvent>) {
        if !cmd.is_enabled(data) && cmd != Command::CloseRom {
            return;
        }

        match cmd {
            Command::OpenRom => {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    out.push(RSnesEvent::LoadRom { path });
                }
            }
            Command::CloseRom => out.push(RSnesEvent::Close),
            Command::Quit => out.push(RSnesEvent::Quit),
            Command::Reset => out.push(RSnesEvent::Reset),
            Command::TogglePause => out.push(RSnesEvent::TogglePause),
            Command::ToggleRomInfo => self.state.show_rom_info = !self.state.show_rom_info,
            Command::QuickSaveState => out.push(RSnesEvent::QuickSaveState),
            Command::LoadLatestState => out.push(RSnesEvent::LoadLatestState),
            Command::ChooseSaveState if self.state.save_state_picker.is_some() => {
                self.state.save_state_picker = None;
            }
            Command::ChooseSaveState => out.push(RSnesEvent::ChooseSaveState),
            Command::UndoLoadState => out.push(RSnesEvent::UndoLoadState),
            Command::ReloadConfig => self.state.reload_config(),
            Command::ToggleConfigErrors => {
                self.state.config_ui = match self.state.config_ui {
                    None => Some(Default::default()),
                    Some(_) => None,
                };
            }
            #[cfg(feature = "plugins")]
            Command::LoadPlugin => self.load_plugin_dialog(),
            #[cfg(feature = "plugins")]
            Command::RunPluginDefault => out.push(RSnesEvent::RunPluginDefault),
        }
    }

    /// Maps a keyboard key to a SNES controller button, or `None` if the key
    /// isn't bound. This is the single place to change the key layout.
    ///
    /// Current layout:
    ///   Arrows = D-pad, Z = X, Q = Y, D = A, S = B
    ///   A = L, E = R, Return = Start, Right Shift = Select.
    fn map_button(keycode: Keycode) -> Option<SnesButton> {
        Some(match keycode {
            Keycode::Z => SnesButton::X,
            Keycode::Q => SnesButton::Y,
            Keycode::D => SnesButton::A,
            Keycode::S => SnesButton::B,
            Keycode::Return => SnesButton::Start,
            Keycode::RShift => SnesButton::Select,
            Keycode::Up => SnesButton::Up,
            Keycode::Down => SnesButton::Down,
            Keycode::Left => SnesButton::Left,
            Keycode::Right => SnesButton::Right,
            Keycode::A => SnesButton::L,
            Keycode::E => SnesButton::R,
            _ => return None,
        })
    }

    /// Maps an Xbox-style controller button to a SNES button. Positional
    /// mapping: Xbox A = SNES B, Xbox B = SNES A, Xbox X = SNES Y, Xbox Y = SNES X,
    /// and the rest are identicals.
    fn map_controller_button(button: ControllerButton) -> Option<SnesButton> {
        Some(match button {
            ControllerButton::A => SnesButton::B,
            ControllerButton::B => SnesButton::A,
            ControllerButton::X => SnesButton::Y,
            ControllerButton::Y => SnesButton::X,
            ControllerButton::LeftShoulder => SnesButton::L,
            ControllerButton::RightShoulder => SnesButton::R,
            ControllerButton::Start => SnesButton::Start,
            ControllerButton::Back => SnesButton::Select,
            ControllerButton::DPadUp => SnesButton::Up,
            ControllerButton::DPadDown => SnesButton::Down,
            ControllerButton::DPadLeft => SnesButton::Left,
            ControllerButton::DPadRight => SnesButton::Right,
            _ => return None,
        })
    }

    /// Turns a left-stick axis motion into button press/release edges.
    /// The stick acts as a digital D-pad substitute: the X axis drives Left/Right,
    /// the Y axis Up/Down (SDL's Y is positive-down). Axis motion arrives continuously,
    /// so this compares the new position against the latched `stick_dirs`
    /// and emits an event only when a direction crosses the deadzone, never on every motion sample.
    fn map_stick_motion(&mut self, axis: sdl2::controller::Axis, value: i16) -> Vec<RSnesEvent> {
        use sdl2::controller::Axis;

        let dz = Self::STICK_DEADZONE;
        let mut out = Vec::new();

        match axis {
            Axis::LeftX => {
                Self::edge(
                    &mut out,
                    &mut self.stick_dirs.left,
                    value < -dz,
                    SnesButton::Left,
                );
                Self::edge(
                    &mut out,
                    &mut self.stick_dirs.right,
                    value > dz,
                    SnesButton::Right,
                );
            }
            Axis::LeftY => {
                Self::edge(
                    &mut out,
                    &mut self.stick_dirs.up,
                    value < -dz,
                    SnesButton::Up,
                );
                Self::edge(
                    &mut out,
                    &mut self.stick_dirs.down,
                    value > dz,
                    SnesButton::Down,
                );
            }
            _ => {}
        }
        out
    }

    /// Emits a press or release edge when `now` differs from the latched `held` state, then updates `held`.
    fn edge(out: &mut Vec<RSnesEvent>, held: &mut bool, now: bool, button: SnesButton) {
        if now && !*held {
            out.push(RSnesEvent::ButtonDown(button));
        } else if !now && *held {
            out.push(RSnesEvent::ButtonUp(button));
        }
        *held = now;
    }

    fn map_event(event: &SdlEvent) -> Option<RSnesEvent> {
        use sdl2::keyboard::Mod;

        match event {
            SdlEvent::Quit { .. } => return Some(RSnesEvent::Quit),

            SdlEvent::ControllerButtonDown { button, .. } => {
                return Self::map_controller_button(*button).map(RSnesEvent::ButtonDown);
            }
            SdlEvent::ControllerButtonUp { button, .. } => {
                return Self::map_controller_button(*button).map(RSnesEvent::ButtonUp);
            }

            _ => {}
        }

        // Controller buttons: a mapped key with no ctrl/alt held. Key releases
        // always clear, even if a modifier is now held, so a held button can't
        // get stuck.
        match event {
            SdlEvent::KeyDown {
                keycode: Some(kc),
                keymod,
                ..
            } if !keymod
                .intersects(Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LALTMOD | Mod::RALTMOD) =>
            {
                Self::map_button(*kc).map(RSnesEvent::ButtonDown)
            }

            SdlEvent::KeyUp {
                keycode: Some(kc), ..
            } => Self::map_button(*kc).map(RSnesEvent::ButtonUp),

            _ => None,
        }
    }

    /// Start (or resume) audio playback. Queued samples begin draining
    /// to the device.
    pub fn audio_play(&self) {
        self.audio_queue.resume();
    }

    /// Pause audio playback, keeping whatever is queued so `audio_play`
    /// picks up exactly where it left off.
    pub fn audio_pause(&self) {
        self.audio_queue.pause();
    }

    /// Stop audio playback and discard anything still queued, so the next
    /// `audio_play` starts silent instead of replaying stale sound.
    pub fn audio_stop(&self) {
        self.audio_queue.pause();
        self.audio_queue.clear();
    }

    /// Queue interleaved stereo i16 samples (L, R, L, R, ...) at 32 kHz —
    /// exactly what `Apu::drain_samples` produces.
    pub fn audio_queue_samples(&self, samples: &[i16]) -> Result<(), String> {
        self.audio_queue.queue_audio(samples)
    }

    /// Number of stereo frames currently queued and not yet played.
    /// (`size` is in bytes; 1 frame = 2 channels × 2 bytes.)
    pub fn audio_buffered_frames(&self) -> u32 {
        self.audio_queue.size() / 4
    }

    /// Moves the picker's selection with the arrows and load with Enter.
    /// Returns whether the key was used.
    fn drive_save_state_picker(picker: &mut SaveStatePicker, key: Keycode) -> bool {
        let row = SaveStatePicker::COLUMNS as isize;
        match key {
            Keycode::Left => picker.move_selection(-1),
            Keycode::Right => picker.move_selection(1),
            Keycode::Up => picker.move_selection(-row),
            Keycode::Down => picker.move_selection(row),
            Keycode::Return | Keycode::KpEnter => picker.load_selected(),
            _ => return false,
        }
        true
    }

    /// Polls SDL, feeds everything to egui, then routes what egui didn't
    /// consume: first to GUI actions (handled here), then to emulator events
    /// (returned to the caller).
    fn handle_events(&mut self, commands: &mut Vec<Command>) -> Vec<RSnesEvent> {
        let pending: Vec<SdlEvent> = self.event_pump.poll_iter().collect();

        let mut out = Vec::new();
        for event in pending {
            // Consuming `WindowEvent::Minimized` to avoid egui-sdl2
            // recomputing its scale to 0x0, which breaks the UI.
            if let SdlEvent::Window {
                win_event: WindowEvent::Minimized,
                ..
            } = event
            {
                continue;
            }

            // While the save state picker is open, arrows and Enter drive it,
            // unless a name is being typed.
            if let Some(picker) = &mut self.state.save_state_picker
                && !self.egui_canvas.ctx.egui_wants_keyboard_input()
                && let SdlEvent::KeyDown {
                    keycode: Some(key), ..
                } = &event
                && Self::drive_save_state_picker(picker, *key)
            {
                continue;
            }

            if self.egui_canvas.on_event(&self.canvas, &event).consumed {
                continue;
            }

            match &event {
                SdlEvent::ControllerDeviceAdded { which, .. } => {
                    println!("Controller added: index {which}");
                    if self.controller.is_none() {
                        self.controller = self._controller_subsystem.open(*which).ok();
                    }
                    continue;
                }
                SdlEvent::ControllerDeviceRemoved { which, .. } => {
                    println!("Controller removed: index {which}");
                    if self.controller.as_ref().is_some_and(|c| c.instance_id() == *which) {
                        self.controller = None;
                    }
                    continue;
                }
                _ => {}
            }

            // Analog stick needs &mut self (latched edge state) and can emit several events at once,
            // so it's handled here rather than in the stateless `map_event`.
            if let SdlEvent::ControllerAxisMotion { axis, value, .. } = &event {
                out.extend(self.map_stick_motion(*axis, *value));
                continue;
            }

            // Shortcuts take priority over emulator buttons.
            if let SdlEvent::KeyDown {
                keycode: Some(key),
                keymod,
                repeat: false,
                ..
            } = &event
                && let Some(cmd) = Command::from_key(*key, *keymod)
            {
                // Escape close opened overlays first before closing the ROM or quitting.
                let closed_menu =
                    cmd == Command::CloseRom && (self.popup_open() || self.state.close_all());
                if !closed_menu {
                    commands.push(cmd);
                }
                continue;
            }

            if let Some(rsnes_event) = Self::map_event(&event) {
                out.push(rsnes_event);
            }
        }
        out
    }

    fn draw_framebuffer(
        &mut self,
        framebuffer: &ppu::rendering::RawFramebuffer,
        menu_bar: u32,
    ) -> Result<(), String> {
        use sdl2::pixels::PixelFormatEnum;

        if self.framebuffer_texture.is_none() {
            let texture_creator = self.canvas.texture_creator();
            let texture = texture_creator
                .create_texture_streaming(
                    PixelFormatEnum::RGB24,
                    SCREEN_WIDTH as u32,
                    SCREEN_HEIGHT as u32,
                )
                .map_err(|e| e.to_string())?;
            self.framebuffer_texture = Some(texture);
        }

        let texture = self.framebuffer_texture.as_mut().unwrap();
        texture
            .update(None, framebuffer, SCREEN_WIDTH * 3)
            .map_err(|e| e.to_string())?;

        let canvas = &mut self.canvas;
        let (width, height) = canvas.output_size()?;
        if let Some(dst) = game_rect(width, height, menu_bar) {
            canvas.copy(texture, None, dst)?;
        }

        Ok(())
    }

    /// Draws every currently-open overlay, kept inside `area` (the space
    /// below the menu bar). Adding a new window means adding one line here
    /// plus one field in `GuiState` — nothing in `main.rs`.
    fn draw_overlays(
        state: &mut GuiState,
        data: &GuiFrameData,
        ctx: &egui::Context,
        area: egui::Rect,
    ) {
        widgets::rom_info(ctx, area, &mut state.show_rom_info, data.rom_info);
        widgets::error_box(ctx, area, &mut state.error_popup);
        widgets::save_state_picker(ctx, area, &mut state.save_state_picker);
        widgets::notification(ctx, area, &mut state.notification);
        widgets::config_error_box(ctx, area, state.config_ui.as_mut(), &state.config_error);
        #[cfg(feature = "plugins")]
        widgets::plugin_perm_request(
            ctx,
            area,
            &mut state.pending_plugin,
            &mut state.granted_plugin,
        );
    }

    /// One frame: poll input, lay out the UI, blit the framebuffer below the
    /// menu bar, paint the UI, present, then run the frame's commands.
    pub fn update(
        &mut self,
        framebuffer: &ppu::rendering::RawFramebuffer,
        data: GuiFrameData,
    ) -> Vec<RSnesEvent> {
        let mut commands = Vec::new();
        let mut events = self.handle_events(&mut commands);

        // Lay out the UI before drawing the game: the game goes in the space
        // the menu bar leaves, so the bar's height must be known first.
        let mut menu_bar_bottom = 0.0;
        self.egui_canvas.run_ui(|ui| {
            menu_bar_bottom = widgets::menu_bar(ui, &self.state, &data, &mut commands);
            // Windows may cover the game, but not the menu bar.
            let mut area = ui.ctx().content_rect();
            area.min.y = area.min.y.max(menu_bar_bottom);
            Self::draw_overlays(&mut self.state, &data, ui.ctx(), area);
        });

        // An action chosen in the save state window. A load closes it.
        if let Some(picker) = &mut self.state.save_state_picker
            && let Some(action) = picker.action.take()
        {
            if matches!(action, SlotAction::Load(_)) {
                self.state.save_state_picker = None;
            }
            events.push(RSnesEvent::SlotAction(action));
        }
        if let Some(config_info) = &mut self.state.config_ui
            && let Some(action) = config_info.action.take()
        {
            self.run_config_ui_action(action);
        }

        let ctx = &self.egui_canvas.ctx;
        // How far down the bar is actually painted, in pixels. egui
        // anti-aliases shape edges by feathering half of `feathering_size`
        // outside them, so the bar's fill bleeds that far past its rect.
        // Rounded up so the game never shares a pixel row with the bar.
        let ppp = ctx.pixels_per_point();
        let feathering_size = ctx.tessellation_options(|o| {
            if o.feathering {
                o.feathering_size_in_pixels / 2.0
            } else {
                0.0
            }
        });
        let menu_bar_px = (menu_bar_bottom * ppp + feathering_size).ceil() as u32;

        self.canvas.set_draw_color(Color::RGB(30, 30, 35));
        self.canvas.clear();
        let _ = self.draw_framebuffer(framebuffer, menu_bar_px);
        self.egui_canvas.paint(&mut self.canvas);
        self.canvas.present();

        // Run commands outside the egui pass: some open blocking file dialogs.
        for cmd in commands {
            self.execute(cmd, &data, &mut events);
        }

        events
    }

    pub fn pass_error(&mut self, err: Box<dyn Error>) {
        self.state.error_popup = Some(err);
    }

    pub fn unwrap_result<T: Default, E: Error + 'static>(&mut self, res: Result<T, E>) -> T {
        match res {
            Ok(x) => x,
            Err(e) => {
                self.pass_error(Box::new(e));
                Default::default()
            }
        }
    }

    fn run_config_ui_action(&mut self, action: ConfigUiAction) {
        let create_new_config = |gui: &mut Self| -> Option<std::fs::File> {
            let path = gui.state.config_path.as_ref()?;
            if let Some(parent) = path.parent() {
                match std::fs::create_dir_all(parent) {
                    Ok(()) => (),
                    Err(e) => {
                        gui.pass_error(Box::new(e));
                        return None;
                    }
                }
            }
            match std::fs::OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(file) => Some(file),
                Err(e) => {
                    gui.pass_error(Box::new(e));
                    None
                }
            }
        };

        match action {
            ConfigUiAction::Close => self.state.config_ui = None,
            ConfigUiAction::Reload => self.state.reload_config(),
            ConfigUiAction::CreateEmptyConfig => {
                if create_new_config(self).is_some() {
                    self.state.reload_config();
                }
            }
            ConfigUiAction::CreateDefaultConfig => {
                if let Some(mut file) = create_new_config(self) {
                    self.unwrap_result(
                        file.write_all(crate::assets::DEFAULT_CONFIG_FILE.as_bytes()),
                    );
                    drop(file);
                    self.state.reload_config();
                }
            }
            ConfigUiAction::OpenConfigFile => {
                if let Some(path) = &self.state.config_path {
                    self.unwrap_result(opener::open(path))
                } else {
                    // only fails when we don't have a path for the config file,
                    // trying to reload will open the config error box or show
                    // a notification that the config file is still invalid if
                    // it was already open
                    self.state.reload_config();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_state::SaveStateError;
    use crate::save_state::test_utils::{setup, tick};
    use test_utils::test_picker;

    #[test]
    fn arrows_move_the_selection_and_enter_loads() {
        let mut picker = test_picker(&egui::Context::default());

        for (key, selected) in [
            (Keycode::Right, 1),
            (Keycode::Down, 6),
            (Keycode::Left, 5),
            (Keycode::Up, 0),
        ] {
            assert!(Gui::drive_save_state_picker(&mut picker, key), "{key}");
            assert_eq!(picker.selected, selected, "after {key}");
        }
        assert!(Gui::drive_save_state_picker(&mut picker, Keycode::Return));
        assert_eq!(picker.action, Some(SlotAction::Load(1)));

        picker.action = None;
        assert!(Gui::drive_save_state_picker(&mut picker, Keycode::KpEnter));
        assert_eq!(picker.action, Some(SlotAction::Load(1)));
    }

    #[test]
    fn other_keys_are_left_to_the_game() {
        let mut picker = test_picker(&egui::Context::default());

        for key in [Keycode::Z, Keycode::Space, Keycode::Escape, Keycode::F4] {
            assert!(!Gui::drive_save_state_picker(&mut picker, key), "{key}");
        }
        assert_eq!(picker.selected, 0);
        assert_eq!(picker.action, None);
    }

    #[test]
    fn picker_entries_show_the_slots_and_their_settings() {
        let (snes, mut states, _root) = setup();
        states.save(&snes).unwrap();
        tick();
        states.save(&snes).unwrap();
        states.rename(2, "boss").unwrap();
        states.set_locked(2, true).unwrap();
        std::fs::write(states.slot_path(5), b"damaged").unwrap();
        states.refresh();
        let ctx = egui::Context::default();

        let first = picker_entry(&ctx, &states, 1);
        let PickerContent::Saved {
            created_at,
            file_size,
            screen,
        } = &first.content
        else {
            panic!("slot 1 holds a save state");
        };
        assert_eq!(Some(*created_at), states.slot(1).time());
        assert_eq!(
            *file_size,
            std::fs::metadata(states.slot_path(1)).unwrap().len()
        );
        assert_eq!(screen.size(), [SCREEN_WIDTH, SCREEN_HEIGHT]);
        assert_eq!((first.slot, first.name.as_str()), (1, ""));
        assert!(!first.locked && !first.latest);

        let second = picker_entry(&ctx, &states, 2);
        assert_eq!(second.name, "boss");
        assert!(second.locked && second.latest);

        let PickerContent::Unreadable { reason, .. } = picker_entry(&ctx, &states, 5).content
        else {
            panic!("slot 5 is damaged");
        };
        assert_eq!(reason, SaveStateError::NotASaveState.to_string());
        assert!(matches!(
            picker_entry(&ctx, &states, 9).content,
            PickerContent::Empty
        ));
    }
}
