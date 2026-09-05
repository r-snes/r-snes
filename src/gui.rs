pub mod state;
pub mod widgets;

use std::error::Error;
use std::path::PathBuf;

use egui_sdl2::canvas::EguiCanvas;
use egui_sdl2::{egui, sdl2};
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::event::Event as SdlEvent;
use sdl2::keyboard::Keycode;
use sdl2::render::Texture;

use ppu::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};

use crate::rsnes::RomInfo;
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
    /// Whether Ctrl+P is allowed to open the plugin picker. Only the idle
    /// loop enables it, since injection needs a running emu (handled by the
    /// emu loop, not here).
    #[cfg(feature = "plugins")]
    plugin_loading_enabled: bool,
}

/// Everything the GUI might need to display, handed in fresh each frame.
///
/// This is the *only* channel through which `main.rs` feeds the GUI.
/// Add a field here when a new overlay needs new data; `main.rs` never
/// touches egui directly.
#[derive(Default)]
pub struct GuiFrameData<'a> {
    pub rom_info: Option<&'a RomInfo>,
}

/// A button on a SNES controller.
///
/// Each button maps to a fixed bit in the JOY1 auto-read register
/// (`$4218`/`$4219`) via [`SnesButton::mask`]. The GUI produces these in
/// response to key presses; the core stores the layout, so the GUI never
/// needs to know the hardware bit order.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum SnesButton {
    X,
    Y,
    A,
    B,
    Start,
    Select,
    Up,
    Down,
    Left,
    Right,
    R,
    L,
}

impl SnesButton {
    /// The button's bit within the 16-bit JOY1 register.
    ///
    /// Layout, from bit 15 down to bit 4:
    /// B, Y, Select, Start, Up, Down, Left, Right, A, X, L, R.
    /// The low 4 bits are unused on a standard controller.
    pub fn mask(self) -> u16 {
        let bit = match self {
            SnesButton::B => 15,
            SnesButton::Y => 14,
            SnesButton::Select => 13,
            SnesButton::Start => 12,
            SnesButton::Up => 11,
            SnesButton::Down => 10,
            SnesButton::Left => 9,
            SnesButton::Right => 8,
            SnesButton::A => 7,
            SnesButton::X => 6,
            SnesButton::L => 5,
            SnesButton::R => 4,
        };
        1 << bit
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum RSnesEvent {
    /// Load a new ROM, showing a file picker (closes current game)
    LoadRom { path: PathBuf },

    /// Close the currently open game (or quit if no game open)
    Close,

    /// Quit the emulator program altogether
    Quit,

    /// A key mapped to an emulated button has been pressed
    ButtonDown(SnesButton),

    /// A key mapped to an emulated button has been released
    ButtonUp(SnesButton),

    /// Run the `default` action of a plugin (if a plugin is loaded, and if it defines one)
    RunPluginDefault,
}

/// Keyboard actions that only affect the GUI, never the emulator.
/// Handled entirely inside `Gui`; `main.rs` never sees these.
enum GuiAction {
    ToggleRomInfo,
    CloseOverlays,
    #[cfg(feature = "plugins")]
    LoadPlugin,
}

#[cfg(not(tarpaulin_include))]
impl Gui {
    pub const FRAME_RATE: u16 = 60;
    pub const FRAME_DURATION: f64 = 1.0 / Self::FRAME_RATE as f64;

    pub fn new() -> Result<Self, String> {
        let sdl_ctx = sdl2::init()?;
        let video_subsystem = sdl_ctx.video()?;

        let window = video_subsystem
            .window("R-SNES", SCREEN_WIDTH as u32 * 2, SCREEN_HEIGHT as u32 * 2)
            .position_centered()
            .build()
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

        Ok(Gui {
            _sdl_ctx: sdl_ctx,
            egui_canvas,
            event_pump,
            audio_queue,
            framebuffer_texture: None,
            state: GuiState::default(),
            #[cfg(feature = "plugins")]
            plugin_loading_enabled: false,
        })
    }

    /// Sets the SDL2 window title. Pass `None` to reset to the default.
    pub fn set_rom_title(&mut self, title: Option<&str>) {
        let window_title = match title {
            Some(t) => format!("R-SNES - {t}"),
            None => "R-SNES".to_string(),
        };

        // Ignore the error: a failed title update is cosmetic, not worth
        // propagating up through the emu loop.
        let _ = self
            .egui_canvas
            .painter
            .canvas
            .window_mut()
            .set_title(&window_title);
    }

    /// Enables/disables Ctrl+P plugin loading. Idle loop enables, emu loop disables.
    #[cfg(feature = "plugins")]
    pub fn set_plugin_loading(&mut self, enabled: bool) {
        self.plugin_loading_enabled = enabled;
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
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Lua plugin", &["lua"])
            .pick_file()
        else {
            return;
        };

        match Plugin::load_from_file(&path) {
            Ok(plugin) => self.state.pending_plugin = Some(PendingPlugin::new(plugin)),
            Err(e) => self.pass_error(Box::new(e)),
        }
    }

    /// Maps an SDL event to a GUI-only action (toggling overlays).
    fn map_gui_action(event: &SdlEvent) -> Option<GuiAction> {
        match event {
            SdlEvent::KeyDown {
                keycode: Some(Keycode::F1),
                repeat: false,
                ..
            } => Some(GuiAction::ToggleRomInfo),

            #[cfg(feature = "plugins")]
            SdlEvent::KeyDown {
                keycode: Some(Keycode::P),
                keymod,
                repeat: false,
                ..
            } if keymod
                .intersects(sdl2::keyboard::Mod::LCTRLMOD | sdl2::keyboard::Mod::RCTRLMOD) =>
            {
                Some(GuiAction::LoadPlugin)
            }

            SdlEvent::KeyDown {
                keycode: Some(Keycode::Escape),
                repeat: false,
                ..
            } => Some(GuiAction::CloseOverlays),

            _ => None,
        }
    }

    /// Maps a keyboard key to a SNES controller button, or `None` if the key
    /// isn't bound. This is the single place to change the key layout.
    ///
    /// Current layout:
    ///   Arrows = D-pad, Z = X, Q = Y, E = A, S = B
    ///   A = L, R = R, Return = Start, Right Shift = Select.
    fn map_button(keycode: Keycode) -> Option<SnesButton> {
        Some(match keycode {
            Keycode::Z => SnesButton::X,
            Keycode::Q => SnesButton::Y,
            Keycode::E => SnesButton::A,
            Keycode::S => SnesButton::B,
            Keycode::Return => SnesButton::Start,
            Keycode::RShift => SnesButton::Select,
            Keycode::Up => SnesButton::Up,
            Keycode::Down => SnesButton::Down,
            Keycode::Left => SnesButton::Left,
            Keycode::Right => SnesButton::Right,
            Keycode::A => SnesButton::L,
            Keycode::R => SnesButton::R,
            _ => return None,
        })
    }

    fn map_event(event: &SdlEvent) -> Option<RSnesEvent> {
        use sdl2::keyboard::Mod;

        // System / GUI shortcuts first - these take precedence over buttons.
        match event {
            SdlEvent::Quit { .. } => return Some(RSnesEvent::Quit),

            SdlEvent::KeyDown {
                keycode: Some(Keycode::Q),
                keymod,
                ..
            } if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => {
                return Some(RSnesEvent::Quit);
            }

            SdlEvent::KeyDown {
                keycode: Some(Keycode::Escape),
                repeat: false,
                ..
            } => return Some(RSnesEvent::Close),

            SdlEvent::KeyDown {
                keycode: Some(Keycode::L),
                keymod,
                ..
            } if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => {
                return rfd::FileDialog::new()
                    .pick_file()
                    .map(|path| RSnesEvent::LoadRom { path });
            }

            SdlEvent::KeyDown {
                keycode: Some(Keycode::R),
                keymod,
                ..
            } if !keymod
                .intersects(Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LALTMOD | Mod::RALTMOD) =>
            {
                return Some(RSnesEvent::RunPluginDefault);
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
    fn handle_events(&mut self) -> Vec<RSnesEvent> {
        let pending: Vec<SdlEvent> = self.event_pump.poll_iter().collect();

        let mut out = Vec::new();
        for event in pending {
            if self.egui_canvas.on_event(&event).consumed {
                continue;
            }

            // GUI actions take priority over emulator events. This is why
            // Escape closes overlays first and only closes the ROM once
            // nothing is open — `close_all` reports whether it did anything.
            if let Some(action) = Self::map_gui_action(&event) {
                match action {
                    GuiAction::ToggleRomInfo => {
                        self.state.show_rom_info = !self.state.show_rom_info;
                        continue;
                    }
                    #[cfg(feature = "plugins")]
                    GuiAction::LoadPlugin => {
                        // Idle-loop only: injection needs a running emu.
                        if self.plugin_loading_enabled {
                            self.load_plugin_dialog();
                        }
                        continue;
                    }
                    GuiAction::CloseOverlays => {
                        if self.state.close_all() {
                            continue; // an overlay was open; Escape consumed by GUI
                        }
                        // nothing was open — fall through so Escape reaches
                        // the emulator as RSnesEvent::Close
                    }
                }
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

        self.egui_canvas.painter.canvas.copy(texture, None, None)?;

        Ok(())
    }

    /// Draws every currently-open overlay. Adding a new window means adding
    /// one line here plus one field in `GuiState` — nothing in `main.rs`.
    fn draw_overlays(state: &mut GuiState, data: &GuiFrameData, ctx: &egui::Context) {
        widgets::rom_info(ctx, &mut state.show_rom_info, data.rom_info);
        widgets::error_box(ctx, &mut state.error_popup);
        #[cfg(feature = "plugins")]
        widgets::plugin_perm_request(ctx, &mut state.pending_plugin, &mut state.granted_plugin);
    }

    /// One frame: poll input, blit the framebuffer, draw overlays, present.
    pub fn update(
        &mut self,
        framebuffer: &ppu::rendering::RawFramebuffer,
        data: GuiFrameData,
    ) -> Vec<RSnesEvent> {
        let events = self.handle_events();

        self.egui_canvas.clear([30, 30, 35, 255]);
        let _ = self.draw_framebuffer(framebuffer);

        // Split the borrow: `run` takes `&mut self.egui_canvas`, so `state`
        // must be borrowed separately rather than through `self`.
        let state = &mut self.state;
        self.egui_canvas
            .run(|ctx| Self::draw_overlays(state, &data, ctx));

        self.egui_canvas.paint();
        self.egui_canvas.present();

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
