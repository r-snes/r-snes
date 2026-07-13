pub mod state;
pub mod widgets;

use std::path::PathBuf;

use egui_sdl2::canvas::EguiCanvas;
use egui_sdl2::{egui, sdl2};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Texture;

use ppu::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};

use crate::rsnes::RomInfo;
use state::GuiState;

pub struct Gui {
    _sdl_ctx: sdl2::Sdl,
    egui_canvas: EguiCanvas,
    event_pump: sdl2::EventPump,
    framebuffer_texture: Option<Texture>,
    /// Persistent overlay state — survives across frames.
    state: GuiState,
}

/// Everything the GUI might need to display, handed in fresh each frame.
///
/// This is the *only* channel through which `main.rs` feeds the GUI.
/// Add a field here when a new overlay needs new data; `main.rs` never
/// touches egui directly.
#[derive(Default)]
pub struct GuiFrameData {
    pub rom_info: Option<RomInfo>,
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
    ButtonDown,

    /// A key mapped to an emulated button has been released
    ButtonUp,

    /// Run the `default` action of a plugin (if a plugin is loaded, and if it defines one)
    RunPluginDefault,
}

/// Keyboard actions that only affect the GUI, never the emulator.
/// Handled entirely inside `Gui`; `main.rs` never sees these.
enum GuiAction {
    ToggleRomInfo,
    CloseOverlays,
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

        Ok(Gui {
            _sdl_ctx: sdl_ctx,
            egui_canvas,
            event_pump,
            framebuffer_texture: None,
            state: GuiState::default(),
        })
    }

    /// Maps an SDL event to a GUI-only action (toggling overlays).
    fn map_gui_action(event: &Event) -> Option<GuiAction> {
        match event {
            Event::KeyDown {
                keycode: Some(Keycode::F1),
                repeat: false,
                ..
            } => Some(GuiAction::ToggleRomInfo),

            Event::KeyDown {
                keycode: Some(Keycode::Escape),
                repeat: false,
                ..
            } => Some(GuiAction::CloseOverlays),

            _ => None,
        }
    }

    fn map_event(event: &Event) -> Option<RSnesEvent> {
        use sdl2::keyboard::Mod;

        match event {
            Event::Quit { .. } => Some(RSnesEvent::Quit),
            Event::KeyDown {
                keycode: Some(Keycode::Q),
                keymod,
                ..
            } if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => Some(RSnesEvent::Quit),

            Event::KeyDown {
                keycode: Some(Keycode::Escape),
                repeat: false,
                ..
            } => Some(RSnesEvent::Close),

            Event::KeyDown {
                keycode: Some(Keycode::L),
                keymod,
                ..
            } if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => rfd::FileDialog::new()
                .pick_file()
                .map(|path| RSnesEvent::LoadRom { path }),

            Event::KeyDown {
                keycode: Some(Keycode::Space),
                repeat: false,
                keymod,
                ..
            } if !keymod
                .intersects(Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LALTMOD | Mod::RALTMOD) =>
            {
                Some(RSnesEvent::ButtonDown)
            }

            Event::KeyUp {
                keycode: Some(Keycode::Space),
                ..
            } => Some(RSnesEvent::ButtonUp),

            Event::KeyDown {
                keycode: Some(Keycode::R),
                keymod,
                ..
            } if !keymod
                .intersects(Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LALTMOD | Mod::RALTMOD) =>
            {
                Some(RSnesEvent::RunPluginDefault)
            }

            _ => None,
        }
    }

    /// Polls SDL, feeds everything to egui, then routes what egui didn't
    /// consume: first to GUI actions (handled here), then to emulator events
    /// (returned to the caller).
    fn handle_events(&mut self) -> Vec<RSnesEvent> {
        let pending: Vec<Event> = self.event_pump.poll_iter().collect();

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
        widgets::rom_info(ctx, &mut state.show_rom_info, data.rom_info.clone());
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
}
