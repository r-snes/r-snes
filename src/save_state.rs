//! Save states: a snapshot of the whole emulated console, taken at any master
//! cycle (mid-instruction, mid-DMA, during `WAI`/`STP`...) and restored as is.
//!
//! # File layout
//!
//! ```text
//! MAGIC                     the text "R-SNES save state\n"
//! SaveStateInfo             postcard, starting with the format version
//!                           (first, so it is read before anything else)
//! core state                postcard CoreState
//! ```
//!
//! Only states of the current [`FORMAT_VERSION`] are loaded, whatever the
//! version of R-SNES that made them. The format version must be bumped
//! whenever the saved data changes.
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
pub(crate) mod test_utils;

/// Text every save state file starts with.
pub const MAGIC: &[u8] = b"R-SNES save state\n";

/// Version of the save state format. Bump it whenever the saved data changes.
pub const FORMAT_VERSION: u32 = 1;

/// Version of R-SNES, written in save states for information.
pub const EMULATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Header of a save state, readable without decoding the rest.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SaveStateInfo {
    /// Version of the save state format, see [`FORMAT_VERSION`].
    ///
    /// Must stay the first field, and a `u32`, in every format: it is read
    /// alone first, so that a file of another format is refused even if the
    /// rest of its info has changed.
    pub format_version: u32,
    /// Version of R-SNES that wrote the file.
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

    /// Checks the magic text and the format version, and reads the info.
    /// Returns the info and the rest of the file.
    fn read_header(bytes: &[u8]) -> Result<(SaveStateInfo, &[u8]), SaveStateError> {
        let rest = bytes.strip_prefix(MAGIC).ok_or(SaveStateError::NotASaveState)?;

        // The info starts with the format version: postcard encodes a struct
        // as its fields one after another, so its first field reads like a
        // lone u32.
        let (found, _) =
            postcard::take_from_bytes::<u32>(rest).map_err(SaveStateError::corrupted)?;
        if found != FORMAT_VERSION {
            return Err(SaveStateError::UnsupportedFormat { found });
        }

        postcard::take_from_bytes(rest).map_err(SaveStateError::corrupted)
    }
}

impl RSnesEmu {
    /// Saves the console. The plugin isn't saved.
    pub fn save_state(&self) -> Result<Vec<u8>, SaveStateError> {
        self.core().save_state()
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
            format_version: FORMAT_VERSION,
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

    /// Checks that a save state can be loaded into this console: made with
    /// the same ROM, and with an S-RAM of the cartridge's size.
    pub fn check_state(&self, state: &SaveState) -> Result<(), SaveStateError> {
        if state.info.rom_checksum != self.rom_checksum {
            return Err(SaveStateError::WrongRom);
        }
        if !self.bus.cart.sram.same_layout(&state.core.sram) {
            return Err(SaveStateError::Corrupted(
                "its S-RAM doesn't match the cartridge's".to_owned(),
            ));
        }
        Ok(())
    }

    /// Loads a save state into this console. A state refused by
    /// [`RSnesCore::check_state`] do not load the save state.
    pub fn load_state(&mut self, state: SaveState) -> Result<(), SaveStateError> {
        self.check_state(&state)?;

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
    /// The file uses another save state format than [`FORMAT_VERSION`].
    UnsupportedFormat {
        /// Format version of the file.
        found: u32,
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
    /// R-SNES's data folder, where save states are stored, can't be found.
    NoDataFolder,
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
            Self::UnsupportedFormat { found } => {
                let made_by = if *found > FORMAT_VERSION {
                    "a newer"
                } else {
                    "an older"
                };
                write!(
                    f,
                    "This save state was made by {made_by} version of R-SNES and can't be \
                     loaded by this one (format {found}, this version reads format {FORMAT_VERSION})."
                )
            }
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
            Self::NoDataFolder => write!(
                f,
                "Save states are stored in R-SNES's data folder, which can't be found."
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

#[cfg(test)]
mod tests {
    use super::test_utils::*;
    use super::*;
    use crate::dma::DmaState;
    use crate::test_utils::{make_rsnes_from, tick_core};
    use cpu::cpu::CPUState;

    #[test]
    fn the_test_rom_runs() {
        let mut core = make_rsnes_from(&test_rom());
        tick_core(&mut core, 3 * MASTER_CYCLES_PER_FRAME);

        let loops = core.bus.wram.data[0x0000];
        let nmis = core.bus.wram.data[0x0010];
        assert!(loops >= 2, "the main loop ran {loops} times");
        assert!(nmis >= 2, "the NMI handler ran {nmis} times");
        assert!(
            core.ppu.vram.memory.iter().any(|&word| word != 0),
            "the DMA never wrote to VRAM"
        );
    }

    #[test]
    fn a_loaded_state_runs_exactly_like_the_original() {
        let rom = test_rom();
        let mut original = make_rsnes_from(&rom);
        let mut copy = make_rsnes_from(&rom);
        tick_core(&mut copy, 12_345);

        type Condition = fn(&RSnesCore) -> bool;
        let save_points: [(&str, Condition, u64); 6] = [
            ("during a DMA", |c| c.dma.state == DmaState::Dma, 30_000),
            ("during HDMA", |c| c.dma.state == DmaState::Hdma, 30_000),
            ("mid-instruction", mid_instruction, 30_000),
            // Run a whole frame after this one, so the copy wakes up on an NMI.
            (
                "in WAI",
                |c| c.cpu.state() == CPUState::WaitForInterrupt,
                MASTER_CYCLES_PER_FRAME + 1_000,
            ),
            ("mid-instruction again", mid_instruction, 30_000),
            (
                "at an instruction start",
                |c| c.cpu.is_instr_start() && c.cpu_master_cycles_to_wait == 0,
                30_000,
            ),
        ];

        for (when, condition, run_after) in save_points {
            run_until(&mut original, when, condition);
            assert!(
                saved_part(&original) != saved_part(&copy),
                "{when}: the copy must start from another state"
            );

            copy_state(&original, &mut copy);
            assert_same_state(&original, &copy, when);

            tick_core(&mut original, run_after);
            tick_core(&mut copy, run_after);
            assert_same_state(&original, &copy, &format!("{when}, then running"));

            // Move the copy away, so the next load starts from another state,
            // even if the next save point is already reached.
            tick_core(&mut copy, 777);
        }
    }

    #[test]
    fn info_records_the_rom_and_the_version() {
        let core = make_rsnes_from(&test_rom());

        let before = SystemTime::now();
        let bytes = core.save_state().unwrap();
        let after = SystemTime::now();

        let info = SaveState::read_info(&bytes).unwrap();
        assert_eq!(info.rom_checksum, core.rom_checksum);
        assert_eq!(info.emulator_version, env!("CARGO_PKG_VERSION"));
        assert!((before..=after).contains(&info.created_at));
        assert_eq!(SaveState::decode(&bytes).unwrap().info, info);
    }

    #[test]
    fn the_emulator_loads_its_own_state_back() {
        let mut emu = emu(make_rsnes_from(&test_rom()));
        tick_core(&mut emu.core_mut(), 50_000);
        let bytes = emu.save_state().unwrap();
        let saved = saved_part(&emu.core());

        tick_core(&mut emu.core_mut(), 50_000);
        emu.load_state(SaveState::decode(&bytes).unwrap()).unwrap();

        assert!(
            saved_part(&emu.core()) == saved,
            "the state wasn't restored"
        );
    }

    #[cfg(feature = "plugins")]
    #[test]
    fn loading_restarts_the_plugin_interval() {
        let plugin = plugins::plugin::Plugin::load_from_raw(b"return { permissions = {} }", None);
        let mut emu =
            RSnesEmu::new_with_plugin(make_rsnes_from(&test_rom()), Some(plugin.unwrap())).unwrap();
        let bytes = emu.save_state().unwrap();

        emu.plugin_mut().unwrap().next_interval_master_cycle = Some(123_456);
        emu.load_state(SaveState::decode(&bytes).unwrap()).unwrap();

        assert_eq!(emu.plugin_mut().unwrap().next_interval_master_cycle, None);
    }

    #[test]
    fn the_screen_is_the_presented_frame() {
        let mut core = make_rsnes_from(&test_rom());
        tick_core(&mut core, 2 * MASTER_CYCLES_PER_FRAME + 5_000);

        let state = SaveState::decode(&core.save_state().unwrap()).unwrap();

        assert!(*state.into_screen() == *core.ppu_renderer.presented());
    }

    #[test]
    fn a_state_of_another_rom_is_refused_and_changes_nothing() {
        let rom = test_rom();
        let mut other_rom = rom.clone();
        other_rom[0x1000] ^= 0xFF; // unused byte

        let mut other = make_rsnes_from(&other_rom);
        tick_core(&mut other, 10_000);
        let mut core = make_rsnes_from(&rom);
        tick_core(&mut core, 20_000);
        let before = saved_part(&core);

        let state = SaveState::decode(&other.save_state().unwrap()).unwrap();
        let result = core.load_state(state);

        assert!(matches!(result, Err(SaveStateError::WrongRom)));
        assert!(saved_part(&core) == before, "the console changed");
    }

    #[test]
    fn a_state_whose_sram_does_not_match_the_cartridge_is_refused_and_changes_nothing() {
        let mut damaged = make_rsnes_from(&test_rom_with_sram());
        damaged.bus.cart.sram = damaged_sram();
        let mut core = make_rsnes_from(&test_rom_with_sram());
        tick_core(&mut core, 20_000);
        let before = saved_part(&core);

        let state = SaveState::decode(&damaged.save_state().unwrap()).unwrap();
        let result = core.load_state(state);

        assert!(matches!(result, Err(SaveStateError::Corrupted(_))));
        assert!(saved_part(&core) == before, "the console changed");
    }

    #[test]
    fn garbage_is_not_a_save_state() {
        let garbage = b"definitely not a save state";

        assert!(matches!(
            decode_error(garbage),
            SaveStateError::NotASaveState
        ));
        assert!(matches!(
            SaveState::read_info(garbage),
            Err(SaveStateError::NotASaveState)
        ));
    }

    #[test]
    fn a_state_of_another_format_is_refused() {
        let core = make_rsnes_from(&test_rom());
        let info = SaveState::read_info(&core.save_state().unwrap()).unwrap();

        for other in [FORMAT_VERSION - 1, FORMAT_VERSION + 1] {
            let info = SaveStateInfo {
                format_version: other,
                ..info.clone()
            };
            let file = postcard::to_extend(&info, MAGIC.to_vec()).unwrap();
            let bytes = core.encode_core(file).unwrap();

            assert!(matches!(
                decode_error(&bytes),
                SaveStateError::UnsupportedFormat { found } if found == other
            ));
            assert!(matches!(
                SaveState::read_info(&bytes),
                Err(SaveStateError::UnsupportedFormat { .. })
            ));
        }
    }

    #[test]
    fn a_state_of_another_emulator_version_is_loaded() {
        let mut core = make_rsnes_from(&test_rom());
        let info = SaveState::read_info(&core.save_state().unwrap()).unwrap();
        let older = SaveStateInfo {
            emulator_version: "0.0.1".into(),
            ..info
        };
        let file = postcard::to_extend(&older, MAGIC.to_vec()).unwrap();
        let bytes = core.encode_core(file).unwrap();

        let state = SaveState::decode(&bytes).unwrap();
        assert_eq!(state.info.emulator_version, "0.0.1");
        core.load_state(state).unwrap();
    }

    #[test]
    fn the_unsupported_format_message_says_which_version_made_the_state() {
        let older = SaveStateError::UnsupportedFormat {
            found: FORMAT_VERSION - 1,
        };
        let newer = SaveStateError::UnsupportedFormat {
            found: FORMAT_VERSION + 1,
        };

        assert!(older.to_string().contains("an older version"), "{older}");
        assert!(newer.to_string().contains("a newer version"), "{newer}");
    }

    #[test]
    fn a_truncated_file_is_corrupted() {
        let bytes = make_rsnes_from(&test_rom()).save_state().unwrap();

        // Without the format version, inside the emulator version, inside the
        // rest of the info, then inside the console state. The format version
        // takes 1 byte, and the emulator version 1 byte of length plus its text.
        for length in [
            MAGIC.len(),
            MAGIC.len() + 1 + 2,
            MAGIC.len() + 1 + 1 + EMULATOR_VERSION.len() + 1,
            bytes.len() / 2,
            bytes.len() - 1,
        ] {
            let result = SaveState::decode(&bytes[..length]);
            assert!(
                matches!(result, Err(SaveStateError::Corrupted(_))),
                "truncated to {length} of {} bytes",
                bytes.len()
            );
        }
    }

    #[test]
    fn bytes_after_the_state_are_corrupted() {
        let mut bytes = make_rsnes_from(&test_rom()).save_state().unwrap();
        bytes.extend_from_slice(b"extra");

        assert!(matches!(decode_error(&bytes), SaveStateError::Corrupted(_)));
    }

    #[test]
    fn every_error_has_a_message() {
        let errors = [
            SaveStateError::NotASaveState,
            SaveStateError::UnsupportedFormat { found: 0 },
            SaveStateError::WrongRom,
            SaveStateError::Corrupted("detail".into()),
            SaveStateError::Encoding("detail".into()),
            SaveStateError::from(io::Error::other("disk on fire")),
            SaveStateError::NoSaveState,
            SaveStateError::NothingToUndo,
            SaveStateError::SlotLocked,
            SaveStateError::AllSlotsLocked,
            SaveStateError::NoDataFolder,
        ];

        for error in errors {
            assert!(error.to_string().ends_with('.'), "{error}");
        }
        let io_error = SaveStateError::from(io::Error::other("disk on fire"));
        assert!(std::error::Error::source(&io_error).is_some());
    }

    #[test]
    fn only_io_errors_have_a_source() {
        let io_error = SaveStateError::from(io::Error::other("disk on fire"));
        let source = std::error::Error::source(&io_error).expect("an I/O error has a source");
        assert_eq!(source.to_string(), "disk on fire");

        for error in [
            SaveStateError::NotASaveState,
            SaveStateError::WrongRom,
            SaveStateError::Corrupted("detail".into()),
            SaveStateError::NoSaveState,
            SaveStateError::SlotLocked,
        ] {
            assert!(std::error::Error::source(&error).is_none(), "{error}");
        }
    }
}
