pub mod command;
pub mod state;
pub mod widgets;

use std::error::Error;
use std::path::PathBuf;

use egui_sdl2::canvas::EguiCanvas;
use egui_sdl2::{egui, sdl2};
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::controller::{Button as ControllerButton, GameController};
use sdl2::event::Event as SdlEvent;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::render::Texture;

use ppu::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};

use crate::rsnes::RomInfo;
use command::Command;
use state::GuiState;

#[cfg(feature = "plugins")]
use plugins::plugin::Plugin;
#[cfg(feature = "plugins")]
use state::PendingPlugin;

pub struct Gui {
    _sdl_ctx: sdl2::Sdl,
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

    /// Whether a menu (or any other egui popup) was open last frame.
    /// Emulation pauses meanwhile, and Escape closes it rather than the game.
    menu_open: bool,
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

    /// Run the `default` action of a plugin (if a plugin is loaded, and if it defines one)
    #[cfg(feature = "plugins")]
    RunPluginDefault,
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

    pub fn new() -> Result<Self, String> {
        let sdl_ctx = sdl2::init()?;
        let video_subsystem = sdl_ctx.video()?;

        // Sizes are in screen coordinates, not pixels (see `SDL_CreateWindow`).
        let mut window = video_subsystem
            .window(
                "R-SNES",
                SCREEN_WIDTH as u32 * 2,
                SCREEN_HEIGHT as u32 * 2 + Self::MENU_BAR_HEIGHT,
            )
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())?;
        window
            .set_minimum_size(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
            .map_err(|e| e.to_string())?;

        let egui_canvas = EguiCanvas::new(window);
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
            egui_canvas,
            event_pump,
            audio_queue,
            framebuffer_texture: None,
            state: GuiState::default(),
            _controller_subsystem: controller_subsystem,
            controller,
            stick_dirs: StickDirs::default(),
            menu_open: false,
        })
    }

    /// Whether a menu is open.
    pub fn menu_open(&self) -> bool {
        self.menu_open
    }

    /// Sets the SDL2 window title. Pass `None` to reset to the default.
    pub fn set_rom_title(&mut self, title: Option<&str>) {
        let window_title = match title {
            Some(t) => format!("R-SNES - {t}"),
            None => "R-SNES".to_string(),
        };

        // Ignore the error: a failed title update is cosmetic, not worth
        // propagating up through the emu loop.
        let _ = self.egui_canvas.painter.canvas.window_mut().set_title(&window_title);
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

    /// Polls SDL, feeds everything to egui, then routes what egui didn't
    /// consume: first to GUI actions (handled here), then to emulator events
    /// (returned to the caller).
    fn handle_events(&mut self, commands: &mut Vec<Command>) -> Vec<RSnesEvent> {
        let pending: Vec<SdlEvent> = self.event_pump.poll_iter().collect();

        let mut out = Vec::new();
        for event in pending {
            if self.egui_canvas.on_event(&event).consumed {
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
                    cmd == Command::CloseRom && (self.menu_open || self.state.close_all());
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
            let texture_creator = self.egui_canvas.painter.canvas.texture_creator();
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

        let canvas = &mut self.egui_canvas.painter.canvas;
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
        // Split the borrow: `run` takes `&mut self.egui_canvas`, so `state`
        // must be borrowed separately rather than through `self`.
        let state = &mut self.state;
        let mut menu_bar_bottom = 0.0;
        self.egui_canvas.run(|ctx| {
            menu_bar_bottom = widgets::menu_bar(ctx, state, &data, &mut commands);
            // Windows may cover the game, but not the menu bar.
            let mut area = ctx.content_rect();
            area.min.y = area.min.y.max(menu_bar_bottom);
            Self::draw_overlays(state, &data, ctx, area);
        });

        let ctx = &self.egui_canvas.ctx;
        self.menu_open = egui::Popup::is_any_open(ctx);
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

        self.egui_canvas.clear([30, 30, 35, 255]);
        let _ = self.draw_framebuffer(framebuffer, menu_bar_px);
        self.egui_canvas.paint();
        self.egui_canvas.present();

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
}
