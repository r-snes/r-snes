//! Save states: a snapshot of the whole emulated console, taken at any master
//! cycle (mid-instruction, mid-DMA, during `WAI`/`STP`...) and restored as is.
//!
//! # File layout
//!
//! ```text
//! MAGIC                     the text "R-SNES save state\n"
//! SaveStateInfo             postcard, starting with the emulator version
//!                           (first, so the slot list reads it fast)
//! core state                postcard CoreState
//! ```
//!
//! A state made by another version of R-SNES is refused.
//! A state made with another ROM (CRC32 mismatch) is refused too.
//!
//! # What is saved
//!
//! Everything the game can observe: WRAM, the I/O registers (DMA channel
//! registers and controller ports included), the cartridge S-RAM, the CPU
//! (including its next cycle function, see `cpu::instrs::cycle_registry`), the
//! PPU, both renderer framebuffers (so a mid-frame save finishes its frame
//! correctly), the APU, the DMA unit and the scheduler counters of
//! [`RSnesCore`].
//!
//! Not saved: the ROM, its header and mapping (they come from the loaded
//! cartridge), the ROM path, the host joypads (live input wins), the audio not
//! yet sent to the sound card, the renderer's per-scanline scratch buffers and
//! the plugin.

use crate::dma::Dma;
use crate::rsnes::{RSnesCore, RSnesEmu};
use apu::Apu;
use bus::Bus;
use bus::cartridge::Cartridge;
use bus::cartridge::sram::Sram;
use bus::io::Io;
use bus::wram::Wram;
use cpu::cpu::CPU;
use duplicate::duplicate;
use ppu::ppu::PPU;
use ppu::rendering::{RawFramebuffer, Renderer};
use std::fmt;
use std::io;
use std::sync::Arc;
use std::time::SystemTime;

pub mod slots;
#[cfg(test)]
mod tests;

/// Text every save state file starts with.
pub const MAGIC: &[u8] = b"R-SNES save state\n";

/// Version of R-SNES, the only one whose save states are loaded.
pub const EMULATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Header of a save state, readable without decoding the rest.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SaveStateInfo {
    /// Version of R-SNES that wrote the file.
    ///
    /// Must stay the first field: it is read alone first, so that a file of
    /// another version is refused even if the rest of its info has changed.
    pub emulator_version: String,
    /// CRC32 of the ROM the state was made with.
    pub rom_checksum: u32,
    /// Creation time.
    pub created_at: SystemTime,
}

// The saved fields of the core, written once: a borrowing struct (for saving)
// and an owning struct (for loading).
duplicate! {
    [
        DUP_name           DUP_derive            DUP_ref  DUP_doc;
        [CoreStateRef<'a>] [serde::Serialize]    [&'a]    ["Saved part of the core, borrowed from it to be serialized."];
        [CoreState]        [serde::Deserialize]  []       ["Saved part of the core, deserialized from a file."];
    ]
    #[doc = DUP_doc]
    #[derive(DUP_derive)]
    struct DUP_name {
        wram: DUP_ref Wram,
        io: DUP_ref Io,
        sram: DUP_ref Sram,
        cpu: DUP_ref CPU,
        ppu: DUP_ref PPU,
        renderer: DUP_ref Renderer,
        apu: DUP_ref Apu,
        dma: DUP_ref Dma,
        master_cycles: DUP_ref u64,
        cpu_master_cycles_to_wait: DUP_ref u32,
        apu_cycle_debt: DUP_ref u64,
        nmi_line: DUP_ref bool,
    }
}

/// A decoded save state, ready to be loaded.
pub struct SaveState {
    /// Header of the file.
    pub info: SaveStateInfo,
    core: CoreState,
}

impl SaveState {
    /// Decodes a whole save state file.
    pub fn decode(bytes: &[u8]) -> Result<Self, SaveStateError> {
        let (info, rest) = Self::read_header(bytes)?;
        let (core, rest) =
            postcard::take_from_bytes::<CoreState>(rest).map_err(SaveStateError::corrupted)?;
        if !rest.is_empty() {
            return Err(SaveStateError::Corrupted(format!(
                "{} unexpected bytes after the console state",
                rest.len()
            )));
        }

        Ok(Self { info, core })
    }

    /// Reads only the header of a save state file, without decoding the console state.
    pub fn read_info(bytes: &[u8]) -> Result<SaveStateInfo, SaveStateError> {
        Self::read_header(bytes).map(|(info, _)| info)
    }

    /// The frame that was on screen when the state was saved, without copying it.
    pub fn into_screen(self) -> Box<RawFramebuffer> {
        self.core.renderer.presented
    }

    /// Checks the magic text and the version, and reads the info.
    /// Returns the info and the rest of the file.
    fn read_header(bytes: &[u8]) -> Result<(SaveStateInfo, &[u8]), SaveStateError> {
        let rest = bytes.strip_prefix(MAGIC).ok_or(SaveStateError::NotASaveState)?;

        // The info starts with the version: postcard encodes a struct as its
        // fields one after another, so its first field reads like a lone String.
        let (found, _) =
            postcard::take_from_bytes::<String>(rest).map_err(SaveStateError::corrupted)?;
        if found != EMULATOR_VERSION {
            return Err(SaveStateError::UnsupportedVersion { found });
        }

        postcard::take_from_bytes(rest).map_err(SaveStateError::corrupted)
    }
}

impl RSnesEmu {
    /// Saves the console. The plugin isn't saved.
    pub fn save_state(&mut self) -> Result<Vec<u8>, SaveStateError> {
        self.core_mut().save_state()
    }

    /// Loads a save state. The plugin keeps running, but its emulated-time interval
    /// is rescheduled from the loaded master cycle count.
    pub fn load_state(&mut self, state: SaveState) -> Result<(), SaveStateError> {
        self.core_mut().load_state(state)?;

        #[cfg(feature = "plugins")]
        if let Some(plugin) = self.plugin_mut() {
            plugin.next_interval_master_cycle = None;
        }
        Ok(())
    }
}

impl RSnesCore {
    /// Saves the whole console into a save state file.
    pub fn save_state(&self) -> Result<Vec<u8>, SaveStateError> {
        let info = SaveStateInfo {
            emulator_version: EMULATOR_VERSION.to_owned(),
            rom_checksum: self.rom_checksum,
            created_at: SystemTime::now(),
        };

        let out = postcard::to_extend(&info, MAGIC.to_vec()).map_err(SaveStateError::encoding)?;
        self.encode_core(out)
    }

    /// Appends the core state to `out`.
    fn encode_core(&self, out: Vec<u8>) -> Result<Vec<u8>, SaveStateError> {
        // No `..`: a new field anywhere in these structs fails to compile here
        // until it is saved or explicitly ignored.
        let RSnesCore {
            rom_path: _,
            bus:
                Bus {
                    wram,
                    cart:
                        Cartridge {
                            rom: _,
                            map: _,
                            header: _,
                            sram,
                        },
                    io,
                },
            cpu,
            ppu,
            ppu_renderer,
            apu,
            master_cycles,
            cpu_master_cycles_to_wait,
            apu_cycle_debt,
            dma,
            nmi_line,
            joypads: _,
            rom_checksum: _,
        } = self;

        let core = CoreStateRef {
            wram,
            io,
            sram,
            cpu,
            ppu,
            renderer: ppu_renderer,
            apu,
            dma,
            master_cycles,
            cpu_master_cycles_to_wait,
            apu_cycle_debt,
            nmi_line,
        };
        postcard::to_extend(&core, out).map_err(SaveStateError::encoding)
    }

    /// Loads a save state into this console.
    pub fn load_state(&mut self, state: SaveState) -> Result<(), SaveStateError> {
        if state.info.rom_checksum != self.rom_checksum {
            return Err(SaveStateError::WrongRom);
        }

        let CoreState {
            wram,
            io,
            sram,
            cpu,
            ppu,
            renderer,
            apu,
            dma,
            master_cycles,
            cpu_master_cycles_to_wait,
            apu_cycle_debt,
            nmi_line,
        } = state.core;

        self.bus.wram = wram;
        self.bus.io = io;
        self.bus.cart.sram = sram;
        self.cpu = cpu;
        self.ppu = ppu;
        self.ppu_renderer = renderer;
        self.apu = apu;
        self.dma = dma;
        self.master_cycles = master_cycles;
        self.cpu_master_cycles_to_wait = cpu_master_cycles_to_wait;
        self.apu_cycle_debt = apu_cycle_debt;
        self.nmi_line = nmi_line;
        Ok(())
    }
}

/// Why a save state couldn't be saved, found or loaded.
#[derive(Debug, Clone)]
pub enum SaveStateError {
    /// The file doesn't start with [`MAGIC`].
    NotASaveState,
    /// The file was made by another version of R-SNES.
    UnsupportedVersion {
        /// Version of R-SNES that made the file.
        found: String,
    },
    /// The state was made with another ROM.
    WrongRom,
    /// The file is damaged.
    Corrupted(String),
    /// The console couldn't be serialized.
    Encoding(String),
    /// A file couldn't be read or written. Shared, since `io::Error` can't
    /// be cloned.
    Io(Arc<io::Error>),
    /// There is no save state to load.
    NoSaveState,
    /// There is no load to undo.
    NothingToUndo,
    /// The slot is locked: it can't be saved over or deleted.
    SlotLocked,
    /// Every slot is locked, so a save has nowhere to go.
    AllSlotsLocked,
    /// The home folder, where save states are stored, can't be found.
    NoHomeFolder,
}

impl SaveStateError {
    fn corrupted(error: postcard::Error) -> Self {
        Self::Corrupted(error.to_string())
    }

    fn encoding(error: impl fmt::Display) -> Self {
        Self::Encoding(error.to_string())
    }
}

impl fmt::Display for SaveStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotASaveState => write!(f, "This file is not an R-SNES save state."),
            Self::UnsupportedVersion { found } => write!(
                f,
                "This save state was made by R-SNES {found} \
                 and can only be loaded by that version (this is R-SNES {EMULATOR_VERSION})."
            ),
            Self::WrongRom => write!(f, "This save state was made with another game."),
            Self::Corrupted(detail) => {
                write!(
                    f,
                    "This save state is damaged and can't be loaded ({detail})."
                )
            }
            Self::Encoding(detail) => write!(f, "The console state couldn't be saved ({detail})."),
            Self::Io(error) => write!(f, "Save state file error: {error}."),
            Self::NoSaveState => write!(f, "No save state for this game yet."),
            Self::NothingToUndo => write!(f, "There is no load to undo."),
            Self::SlotLocked => write!(f, "This slot is locked. Unlock it first."),
            Self::AllSlotsLocked => write!(f, "Every slot is locked. Unlock one to save."),
            Self::NoHomeFolder => write!(
                f,
                "Save states are stored in the home folder, which can't be found."
            ),
        }
    }
}

impl std::error::Error for SaveStateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

impl From<io::Error> for SaveStateError {
    fn from(error: io::Error) -> Self {
        Self::Io(Arc::new(error))
    }
}
