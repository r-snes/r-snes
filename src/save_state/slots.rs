//! Save state files on disk: 10 slots per game, plus one file to undo a load.
//!
//! # Folders
//!
//! ```text
//! <home>/rsnes-save-states/
//! └── <cleaned ROM title>-<ROM CRC32>/      e.g. "SUPER GAME-1A2B3C4D/"
//!     ├── slot-01.rsnes-state
//!     ├── ...
//!     ├── slot-10.rsnes-state
//!     └── undo-load.rsnes-state
//! ```
//!
//! The game folder is created on the first save. The CRC32 in its name keeps
//! two games with the same title (or two versions of a game) apart.
//!
//! # Slots
//!
//! The slots are read from the disk once, when the game's save states are
//! opened, and again on [`GameSaveStates::refresh`] (when the slot picker
//! opens). In between, they are kept in memory: a save updates its slot
//! there, and a load that finds its file missing or damaged updates it too.
//! Files changed outside this window are only seen on the next refresh.
//!
//! A save goes to the first free slot, or overwrites the oldest one once all
//! 10 are used. Age comes from the creation time in each file, or from the
//! file's modification time when the file can't be read.
//! "Load latest" loads the most recent readable slot.
//!
//! # Undoing a load
//!
//! Every load first saves the current console to `undo-load.rsnes-state`.
//! Undoing a load loads that file the same way, which saves the current
//! console to it first: undoing twice redoes the load. A load that is refused
//! (another game, a damaged file) doesn't touch the undo file.
//!
//! # Atomic writes
//!
//! A file is written to `<name>.rsnes-state.tmp`, flushed to the disk, then
//! renamed over `<name>.rsnes-state`: a crash while saving leaves the previous
//! file intact, never a half-written one.

use super::{SaveState, SaveStateError};
use crate::rsnes::RSnesEmu;
use ppu::rendering::RawFramebuffer;
use std::cmp::Reverse;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
mod tests;

/// Number of save slots of each game.
pub const SLOT_COUNT: usize = 10;

/// Name of the folder, in the home folder, holding the save states of every game.
pub const ROOT_FOLDER_NAME: &str = "rsnes-save-states";

/// Extension of save state files.
pub const FILE_EXTENSION: &str = "rsnes-state";

/// Name of the file that undoes the last load, without its extension.
const UNDO_FILE_NAME: &str = "undo-load";

/// What a slot holds, as last read from the disk.
pub enum Slot {
    /// No file.
    Empty,
    /// A save state of this game.
    Saved {
        /// Creation time of the state.
        created_at: SystemTime,
        /// The frame on screen when the state was saved.
        screen: Box<RawFramebuffer>,
    },
    /// A file that can't be loaded: damaged, of another version or of another game.
    Unreadable {
        /// Modification time of the file.
        modified_at: SystemTime,
        /// Why the file can't be loaded.
        reason: String,
    },
}

impl Slot {
    /// When the slot was written, `None` if it is empty.
    pub fn time(&self) -> Option<SystemTime> {
        match self {
            Self::Empty => None,
            Self::Saved { created_at, .. } => Some(*created_at),
            Self::Unreadable { modified_at, .. } => Some(*modified_at),
        }
    }
}

/// The save state files of one game, and what their slots hold.
pub struct GameSaveStates {
    folder: PathBuf,
    rom_checksum: u32,
    slots: [Slot; SLOT_COUNT],
}

impl GameSaveStates {
    /// The save states of a game, in the home folder.
    pub fn for_rom(title: &str, rom_checksum: u32) -> Result<Self, SaveStateError> {
        let home = std::env::home_dir()
            .filter(|home| !home.as_os_str().is_empty())
            .ok_or(SaveStateError::NoHomeFolder)?;
        Ok(Self::in_folder(
            &home.join(ROOT_FOLDER_NAME),
            title,
            rom_checksum,
        ))
    }

    /// The save states of a game, in the game folder inside `root`.
    pub fn in_folder(root: &Path, title: &str, rom_checksum: u32) -> Self {
        let mut states = Self {
            folder: root.join(game_folder_name(title, rom_checksum)),
            rom_checksum,
            slots: [const { Slot::Empty }; SLOT_COUNT],
        };
        states.refresh();
        states
    }

    /// Reads every slot from the disk again.
    pub fn refresh(&mut self) {
        for slot in 1..=SLOT_COUNT {
            self.slots[slot - 1] = read_slot(&self.slot_path(slot), self.rom_checksum);
        }
    }

    /// Path of a slot file, `slot` going from 1 to [`SLOT_COUNT`].
    pub fn slot_path(&self, slot: usize) -> PathBuf {
        debug_assert!((1..=SLOT_COUNT).contains(&slot), "no slot {slot}");
        self.folder.join(format!("slot-{slot}.{FILE_EXTENSION}"))
    }

    /// Path of the file that undoes the last load.
    pub fn undo_path(&self) -> PathBuf {
        self.folder.join(format!("{UNDO_FILE_NAME}.{FILE_EXTENSION}"))
    }

    /// What a slot holds, `slot` going from 1 to [`SLOT_COUNT`].
    pub fn slot(&self, slot: usize) -> &Slot {
        &self.slots[slot - 1]
    }

    /// The slots that aren't empty, with their number, the most recent first.
    pub fn used_slots(&self) -> Vec<(usize, &Slot)> {
        let mut used: Vec<_> = (1..=SLOT_COUNT)
            .map(|slot| (slot, self.slot(slot)))
            .filter(|(_, slot)| slot.time().is_some())
            .collect();
        used.sort_by_key(|(_, slot)| Reverse(slot.time()));
        used
    }

    /// The slot the next save goes to: the first empty one, or the oldest one.
    pub fn slot_to_write(&self) -> usize {
        (1..=SLOT_COUNT)
            .min_by_key(|&slot| self.slot(slot).time())
            .expect("there are slots")
    }

    /// The most recent slot holding a save state of this game.
    pub fn latest_slot(&self) -> Option<usize> {
        (1..=SLOT_COUNT)
            .filter_map(|slot| match self.slot(slot) {
                Slot::Saved { created_at, .. } => Some((slot, *created_at)),
                _ => None,
            })
            .max_by_key(|&(_, created_at)| created_at)
            .map(|(slot, _)| slot)
    }

    /// Saves the console to the first empty slot, or over the oldest one.
    /// Returns the slot number.
    pub fn save(&mut self, emu: &mut RSnesEmu) -> Result<usize, SaveStateError> {
        let bytes = emu.save_state()?;
        let slot = self.slot_to_write();

        fs::create_dir_all(&self.folder)?;
        write_atomic(&self.slot_path(slot), &bytes)?;

        self.slots[slot - 1] = Slot::Saved {
            created_at: SaveState::read_info(&bytes)?.created_at,
            screen: emu.core_mut().ppu_renderer.presented.clone(),
        };
        Ok(slot)
    }

    /// Loads a slot, `slot` going from 1 to [`SLOT_COUNT`].
    ///
    /// The file is read from the disk. If it is missing or can't be loaded,
    /// the slot is updated in memory.
    pub fn load_slot(&mut self, emu: &mut RSnesEmu, slot: usize) -> Result<(), SaveStateError> {
        let path = self.slot_path(slot);
        let result = read_if_exists(&path)?
            .ok_or(SaveStateError::NoSaveState)
            .and_then(|bytes| self.load_with_undo(emu, &bytes));

        match &result {
            Err(SaveStateError::NoSaveState) => self.slots[slot - 1] = Slot::Empty,
            Err(
                error @ (SaveStateError::NotASaveState
                | SaveStateError::UnsupportedVersion { .. }
                | SaveStateError::WrongRom
                | SaveStateError::Corrupted(_)),
            ) => self.slots[slot - 1] = unreadable(&path, error),
            _ => {}
        }
        result
    }

    /// Loads the most recent slot holding a save state of this game.
    /// Returns the slot number.
    pub fn load_latest(&mut self, emu: &mut RSnesEmu) -> Result<usize, SaveStateError> {
        let slot = self.latest_slot().ok_or(SaveStateError::NoSaveState)?;
        self.load_slot(emu, slot)?;
        Ok(slot)
    }

    /// Goes back to the state before the last load (or the last undo).
    pub fn undo_load(&self, emu: &mut RSnesEmu) -> Result<(), SaveStateError> {
        let bytes = read_if_exists(&self.undo_path())?.ok_or(SaveStateError::NothingToUndo)?;
        self.load_with_undo(emu, &bytes)
    }

    /// Checks that `bytes` can be loaded, saves the current console to the
    /// undo file, then loads `bytes`. Nothing is written if the state is refused,
    /// and nothing is loaded if the undo file can't be written.
    fn load_with_undo(&self, emu: &mut RSnesEmu, bytes: &[u8]) -> Result<(), SaveStateError> {
        let rom_checksum = emu.core_mut().rom_checksum;
        let state = decode_for_rom(bytes, rom_checksum)?;

        let current = emu.save_state()?;
        write_atomic(&self.undo_path(), &current)?;
        emu.load_state(state)
    }
}

/// Name of the folder of a game: its title, made safe for file names, and its
/// ROM checksum.
fn game_folder_name(title: &str, rom_checksum: u32) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || " -_.()".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();

    let cleaned = cleaned.trim();
    let title = if cleaned.is_empty() {
        "UNTITLED"
    } else {
        cleaned
    };
    format!("{title}-{rom_checksum:08X}")
}

/// Reads what a slot file holds.
fn read_slot(path: &Path, rom_checksum: u32) -> Slot {
    let bytes = match read_if_exists(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Slot::Empty,
        Err(error) => return unreadable(path, &error.into()),
    };

    match decode_for_rom(&bytes, rom_checksum) {
        Ok(state) => Slot::Saved {
            created_at: state.info.created_at,
            screen: state.into_screen(),
        },
        Err(error) => unreadable(path, &error),
    }
}

/// Decodes a save state, refusing one made with another ROM.
fn decode_for_rom(bytes: &[u8], rom_checksum: u32) -> Result<SaveState, SaveStateError> {
    let state = SaveState::decode(bytes)?;
    if state.info.rom_checksum != rom_checksum {
        return Err(SaveStateError::WrongRom);
    }
    Ok(state)
}

/// A slot whose file can't be loaded, dated by the file's modification time
/// (the Unix epoch, the oldest possible, if there is none).
fn unreadable(path: &Path, error: &SaveStateError) -> Slot {
    Slot::Unreadable {
        modified_at: fs::metadata(path)
            .and_then(|meta| meta.modified())
            .unwrap_or(UNIX_EPOCH),
        reason: error.to_string(),
    }
}

/// Reads a file, `None` if it doesn't exist.
fn read_if_exists(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Writes `bytes` to a temporary file next to `path`, flushes it to the disk
/// and renames it to `path`, replacing the previous file.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = path.with_added_extension("tmp");

    let result = File::create(&temporary)
        .and_then(|mut file| {
            file.write_all(bytes)?;
            file.sync_all()
        })
        .and_then(|()| fs::rename(&temporary, path));

    if result.is_err() {
        // Best effort: the error that matters is the one returned.
        let _ = fs::remove_file(&temporary);
    }
    result
}
