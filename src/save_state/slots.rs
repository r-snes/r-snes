//! Save state files on disk: 10 slots per game, plus one file to undo a load.
//!
//! # Folders
//!
//! Save states are in R-SNES's data folder, found with [`ProjectDirs`]:
//! - Linux: `$XDG_DATA_HOME/r-snes/save-states/`, by default
//!   `~/.local/share/r-snes/save-states/`;
//! - Windows: `%APPDATA%\r-snes\data\save-states\`;
//! - macOS: `~/Library/Application Support/r-snes/save-states/`.
//!
//! ```text
//! save-states/
//! └── <cleaned ROM title>-<ROM CRC32>/      e.g. "SUPER GAME-1A2B3C4D/"
//!     ├── slot-1.rsnes-state
//!     ├── ...
//!     ├── slot-10.rsnes-state
//!     ├── undo-load.rsnes-state
//!     └── slots.rsnes-settings
//! ```
//!
//! The game folder is created on the first write. The CRC32 in its name keeps
//! two games with the same title (or two versions of a game) apart.
//!
//! # Slots
//!
//! The slots are read from the disk once, when the game's save states are
//! opened, and again on [`GameSaveStates::refresh`] (when the slot picker
//! opens). In between, they are kept in memory: every action updates its
//! slot there, and a load that finds its file missing or damaged updates it
//! too. Files changed outside this window are only seen on the next refresh.
//!
//! A save goes to the first free slot, or overwrites the oldest one once all
//! 10 are used. Locked slots are skipped. Age comes from the creation time in
//! each file, or from the file's modification time when the file can't be
//! read. "Load latest" loads the most recent readable slot.
//!
//! # Slot settings
//!
//! The name and the lock of every slot are kept in `slots.rsnes-settings`,
//! not in the save state files, so changing them doesn't rewrite a state.
//! A locked slot is never saved over nor deleted. Saving over a slot clears
//! its name. A slot found empty loses its settings, so a file deleted by hand
//! doesn't leave a name or a lock behind. A settings file that can't be read
//! counts as no settings.
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
//! A file is written to `<name>.tmp`, flushed to the disk, then renamed over
//! `<name>`: a crash while saving leaves the previous file intact, never a
//! half-written one.

use super::{SaveState, SaveStateError};
use crate::rsnes::RSnesEmu;
use directories::ProjectDirs;
use ppu::rendering::RawFramebuffer;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Number of save slots of each game.
pub const SLOT_COUNT: usize = 10;

/// Name of R-SNES's folder in the platform's data folder.
pub const APP_FOLDER_NAME: &str = "r-snes";

/// Name of the folder, in R-SNES's data folder, holding the save states of every game.
pub const ROOT_FOLDER_NAME: &str = "save-states";

/// Extension of save state files.
pub const FILE_EXTENSION: &str = "rsnes-state";

/// Name of the file that undoes the last load, without its extension.
const UNDO_FILE_NAME: &str = "undo-load";

/// Name of the file holding the settings of every slot.
const SETTINGS_FILE_NAME: &str = "slots.rsnes-settings";

/// Longest slot name, in characters.
pub const MAX_NAME_LENGTH: usize = 32;

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
        /// Size of the file, in bytes.
        file_size: u64,
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

/// What the user set on a slot.
#[derive(Default, Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct SlotSettings {
    /// A short name, empty if none.
    pub name: String,
    /// A locked slot is never saved over nor deleted.
    pub locked: bool,
}

/// The save state files of one game, and what their slots hold.
pub struct GameSaveStates {
    folder: PathBuf,
    rom_checksum: u32,
    slots: [Slot; SLOT_COUNT],
    settings: [SlotSettings; SLOT_COUNT],
}

impl GameSaveStates {
    /// The save states of a game, in R-SNES's data folder.
    pub fn for_rom(title: &str, rom_checksum: u32) -> Result<Self, SaveStateError> {
        let root = save_states_root().ok_or(SaveStateError::NoDataFolder)?;
        Ok(Self::in_folder(&root, title, rom_checksum))
    }

    /// The save states of a game, in the game folder inside `root`.
    pub fn in_folder(root: &Path, title: &str, rom_checksum: u32) -> Self {
        let mut states = Self {
            folder: root.join(game_folder_name(title, rom_checksum)),
            rom_checksum,
            slots: [const { Slot::Empty }; SLOT_COUNT],
            settings: Default::default(),
        };
        states.refresh();
        states
    }

    /// Reads every slot and their settings from the disk again.
    pub fn refresh(&mut self) {
        for slot in 1..=SLOT_COUNT {
            self.slots[slot - 1] = read_slot(&self.slot_path(slot), self.rom_checksum);
        }

        self.settings = read_if_exists(&self.settings_path())
            .ok()
            .flatten()
            .and_then(|bytes| postcard::from_bytes(&bytes).ok())
            .unwrap_or_default();
        for (slot, settings) in self.slots.iter().zip(&mut self.settings) {
            if matches!(slot, Slot::Empty) {
                *settings = SlotSettings::default();
            }
        }
    }

    /// The folder holding the game's files. It may not exist yet.
    pub fn folder(&self) -> &Path {
        &self.folder
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

    /// Path of the file holding the settings of every slot.
    pub fn settings_path(&self) -> PathBuf {
        self.folder.join(SETTINGS_FILE_NAME)
    }

    /// What a slot holds, `slot` going from 1 to [`SLOT_COUNT`].
    pub fn slot(&self, slot: usize) -> &Slot {
        &self.slots[slot - 1]
    }

    /// The name and lock of a slot, `slot` going from 1 to [`SLOT_COUNT`].
    pub fn settings(&self, slot: usize) -> &SlotSettings {
        &self.settings[slot - 1]
    }

    /// The slot the next save goes to: the first empty one, or the oldest
    /// one. Locked slots are skipped, so `None` if they are all locked.
    pub fn slot_to_write(&self) -> Option<usize> {
        (1..=SLOT_COUNT)
            .filter(|&slot| !self.settings(slot).locked)
            .min_by_key(|&slot| self.slot(slot).time())
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

    /// Saves the console to the first empty slot, or over the oldest one,
    /// skipping locked slots. Returns the slot number.
    pub fn save(&mut self, emu: &RSnesEmu) -> Result<usize, SaveStateError> {
        let slot = self.slot_to_write().ok_or(SaveStateError::AllSlotsLocked)?;
        self.save_to(emu, slot)?;
        Ok(slot)
    }

    /// Saves the console to a slot, `slot` going from 1 to [`SLOT_COUNT`],
    /// over what it holds unless it is locked. The slot loses its name.
    pub fn save_to(&mut self, emu: &RSnesEmu, slot: usize) -> Result<(), SaveStateError> {
        if self.settings(slot).locked {
            return Err(SaveStateError::SlotLocked);
        }
        let bytes = emu.save_state()?;

        fs::create_dir_all(&self.folder)?;
        write_atomic(&self.slot_path(slot), &bytes)?;

        self.slots[slot - 1] = Slot::Saved {
            created_at: SaveState::read_info(&bytes)?.created_at,
            screen: emu.core().ppu_renderer.presented.clone(),
            file_size: bytes.len() as u64,
        };
        self.change_settings(slot, |settings| settings.name.clear())
    }

    /// Deletes a slot's file, unless the slot is locked. Deleting an empty
    /// slot does nothing.
    pub fn delete(&mut self, slot: usize) -> Result<(), SaveStateError> {
        if self.settings(slot).locked {
            return Err(SaveStateError::SlotLocked);
        }
        if let Err(error) = fs::remove_file(self.slot_path(slot))
            && error.kind() != io::ErrorKind::NotFound
        {
            return Err(error.into());
        }

        self.slots[slot - 1] = Slot::Empty;
        self.change_settings(slot, |settings| *settings = SlotSettings::default())
    }

    /// Names a slot. The name is trimmed and cut to [`MAX_NAME_LENGTH`] characters;
    pub fn rename(&mut self, slot: usize, name: &str) -> Result<(), SaveStateError> {
        self.require_used(slot)?;
        let name: String = name.trim().chars().take(MAX_NAME_LENGTH).collect();
        self.change_settings(slot, |settings| settings.name = name)
    }

    /// Locks or unlocks a slot.
    pub fn set_locked(&mut self, slot: usize, locked: bool) -> Result<(), SaveStateError> {
        self.require_used(slot)?;
        self.change_settings(slot, |settings| settings.locked = locked)
    }

    /// Fails with [`SaveStateError::NoSaveState`] if the slot is empty: an
    /// empty slot has no settings.
    fn require_used(&self, slot: usize) -> Result<(), SaveStateError> {
        match self.slot(slot) {
            Slot::Empty => Err(SaveStateError::NoSaveState),
            _ => Ok(()),
        }
    }

    /// Changes the settings of a slot, and writes the settings file if they
    /// changed.
    fn change_settings(
        &mut self,
        slot: usize,
        change: impl FnOnce(&mut SlotSettings),
    ) -> Result<(), SaveStateError> {
        let settings = &mut self.settings[slot - 1];
        let before = settings.clone();
        change(settings);
        if *settings == before {
            return Ok(());
        }

        let bytes = postcard::to_stdvec(&self.settings).map_err(SaveStateError::encoding)?;
        fs::create_dir_all(&self.folder)?;
        write_atomic(&self.settings_path(), &bytes)?;
        Ok(())
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
        let state = SaveState::decode(bytes)?;
        emu.core().check_state(&state)?;

        let current = emu.save_state()?;
        write_atomic(&self.undo_path(), &current)?;
        emu.load_state(state)
    }
}

/// The folder holding the save states of every game, in R-SNES's data
/// folder. `None` if the platform's data folder can't be found.
fn save_states_root() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "", APP_FOLDER_NAME)?;
    Some(dirs.data_dir().join(ROOT_FOLDER_NAME))
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
            file_size: bytes.len() as u64,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_state::test_utils::*;
    use crate::test_utils::{make_rsnes_from, tick_core};

    #[test]
    fn game_folder_names_are_cleaned() {
        assert_eq!(
            game_folder_name("SUPER GAME", 0x1A2B_3C4D),
            "SUPER GAME-1A2B3C4D"
        );
        assert_eq!(
            game_folder_name("Game (v1.1) - part_2", 0xAB),
            "Game (v1.1) - part_2-000000AB"
        );
        assert_eq!(
            game_folder_name("  Pokémon: Red/Blue\0 ", 1),
            "Pok_mon_ Red_Blue_-00000001"
        );
        assert_eq!(game_folder_name("", 1), "UNTITLED-00000001");
        assert_eq!(game_folder_name("    ", 1), "UNTITLED-00000001");
    }

    #[test]
    fn slot_files_are_named_after_their_number() {
        let root = Path::new("root");
        let states = GameSaveStates::in_folder(root, "GAME", 0x1234_5678);
        let folder = root.join("GAME-12345678");

        assert_eq!(states.slot_path(1), folder.join("slot-1.rsnes-state"));
        assert_eq!(states.slot_path(10), folder.join("slot-10.rsnes-state"));
        assert_eq!(states.undo_path(), folder.join("undo-load.rsnes-state"));
    }

    #[test]
    fn save_states_are_in_the_data_folder() {
        // Only reads: nothing is created.
        let Some(dirs) = ProjectDirs::from("", "", "r-snes") else {
            return;
        };
        let states = GameSaveStates::for_rom("GAME", 0x1234_5678).unwrap();

        assert_eq!(
            states.slot_path(1),
            dirs.data_dir()
                .join("save-states")
                .join("GAME-12345678")
                .join("slot-1.rsnes-state")
        );
    }

    #[test]
    fn an_atomic_write_leaves_no_temporary_file() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("file.rsnes-state");

        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"second");
        let names: Vec<String> = fs::read_dir(folder.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["file.rsnes-state"]);
    }

    #[test]
    fn a_failed_atomic_write_leaves_no_temporary_file() {
        let folder = tempfile::tempdir().unwrap();
        // A folder can't be replaced by a file: the rename fails.
        let path = folder.path().join("file.rsnes-state");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("inside"), b"kept").unwrap();

        assert!(write_atomic(&path, b"new").is_err());

        let names: Vec<String> = fs::read_dir(folder.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["file.rsnes-state"]);
        assert_eq!(fs::read(path.join("inside")).unwrap(), b"kept");
    }

    #[test]
    fn the_folder_is_named_after_the_game() {
        let root = Path::new("root");
        let states = GameSaveStates::in_folder(root, "GAME", 0x1234_5678);

        assert_eq!(states.folder(), root.join("GAME-12345678"));
        assert_eq!(states.slot_path(1).parent(), Some(states.folder()));
        assert_eq!(states.settings_path().parent(), Some(states.folder()));
    }

    #[test]
    fn a_slot_that_cannot_be_read_or_deleted_reports_it() {
        let (mut snes, mut states, _root) = setup();
        states.save(&snes).unwrap();
        // A folder in place of the file: reading and deleting it fail.
        fs::create_dir(states.slot_path(4)).unwrap();
        states.refresh();
        assert!(is_unreadable(states.slot(4)));

        let before = state_of(&snes);
        assert!(matches!(
            states.load_slot(&mut snes, 4),
            Err(SaveStateError::Io(_))
        ));
        assert!(matches!(states.delete(4), Err(SaveStateError::Io(_))));

        assert!(state_of(&snes) == before, "the console changed");
        assert!(is_unreadable(states.slot(4)), "the slot was forgotten");
        assert!(states.slot_path(4).is_dir());
    }

    #[test]
    fn a_new_folder_has_only_empty_slots() {
        let (_snes, states, root) = setup();

        assert!((1..=SLOT_COUNT).all(|slot| is_empty(states.slot(slot))));
        assert_eq!(states.latest_slot(), None);
        assert_eq!(states.slot_to_write(), Some(1));
        assert!(
            fs::read_dir(root.path()).unwrap().next().is_none(),
            "a folder was created"
        );
    }

    #[test]
    fn a_save_updates_its_slot_in_memory() {
        let (mut snes, mut states, _root) = setup();
        tick_core(&mut snes.core_mut(), 400_000); // more than a frame, so a frame is presented

        let slot = states.save(&snes).unwrap();

        let Slot::Saved {
            created_at,
            screen,
            file_size,
        } = states.slot(slot)
        else {
            panic!("slot {slot} holds no save state");
        };
        let file = fs::read(states.slot_path(slot)).unwrap();
        assert_eq!(*created_at, SaveState::read_info(&file).unwrap().created_at);
        assert!(*screen == screen_of(&snes));
        assert_eq!(*file_size, file.len() as u64);
    }

    #[test]
    fn opening_the_folder_reads_the_slots_on_disk() {
        let (snes, mut states, root) = setup();
        states.save(&snes).unwrap();
        tick();
        states.save(&snes).unwrap();
        fs::write(states.slot_path(5), b"damaged").unwrap();

        let reopened = reopen(&root, &snes);

        assert!(*saved_screen(reopened.slot(1)) == *saved_screen(states.slot(1)));
        assert_eq!(reopened.slot(2).time(), states.slot(2).time());
        assert!(is_unreadable(reopened.slot(5)));
        assert!(
            [3, 4, 6, 7, 8, 9, 10]
                .into_iter()
                .all(|slot| is_empty(reopened.slot(slot)))
        );
        assert_eq!(reopened.latest_slot(), Some(2));
    }

    #[test]
    fn saves_fill_the_slots_then_overwrite_the_oldest() {
        let (snes, mut states, _root) = setup();

        for expected in 1..=SLOT_COUNT {
            assert_eq!(states.save(&snes).unwrap(), expected);
            tick();
        }
        assert_eq!(states.save(&snes).unwrap(), 1);
        assert_eq!(
            states.latest_slot(),
            Some(1),
            "the latest isn't the highest"
        );
        tick();
        assert_eq!(states.save(&snes).unwrap(), 2);

        assert!((1..=SLOT_COUNT).all(|slot| !is_empty(states.slot(slot))));
    }

    #[test]
    fn a_slot_deleted_by_hand_is_reused_after_a_refresh() {
        let (snes, mut states, _root) = setup();
        for _ in 1..=3 {
            states.save(&snes).unwrap();
            tick();
        }

        fs::remove_file(states.slot_path(2)).unwrap();
        states.refresh();

        assert!(is_empty(states.slot(2)));
        assert_eq!(states.save(&snes).unwrap(), 2);
        assert_eq!(states.save(&snes).unwrap(), 4);
    }

    #[test]
    fn a_slot_deleted_by_hand_is_noticed_on_load() {
        let (mut snes, mut states, _root) = setup();
        states.save(&snes).unwrap();

        fs::remove_file(states.slot_path(1)).unwrap();

        assert!(matches!(
            states.load_latest(&mut snes),
            Err(SaveStateError::NoSaveState)
        ));
        assert!(is_empty(states.slot(1)));
        assert_eq!(states.latest_slot(), None);
    }

    #[test]
    fn load_latest_restores_the_latest_save() {
        let (mut snes, mut states, _root) = setup();
        advance(&mut snes);
        states.save(&snes).unwrap();
        tick();
        advance(&mut snes);
        states.save(&snes).unwrap();
        let saved = state_of(&snes);

        advance(&mut snes);
        assert_eq!(states.load_latest(&mut snes).unwrap(), 2);

        assert!(state_of(&snes) == saved, "the save wasn't restored");
    }

    #[test]
    fn load_latest_skips_unreadable_slots() {
        let (mut snes, mut states, _root) = setup();
        states.save(&snes).unwrap();
        let saved = state_of(&snes);
        tick();
        fs::write(states.slot_path(2), b"damaged").unwrap();
        states.refresh();

        advance(&mut snes);
        assert_eq!(states.load_latest(&mut snes).unwrap(), 1);

        assert!(state_of(&snes) == saved, "the save wasn't restored");
    }

    #[test]
    fn undo_restores_the_state_before_the_load_and_undoing_again_redoes_it() {
        let (mut snes, mut states, _root) = setup();
        advance(&mut snes);
        states.save(&snes).unwrap();
        let saved = state_of(&snes);
        advance(&mut snes);
        let before_load = state_of(&snes);

        states.load_latest(&mut snes).unwrap();
        assert!(state_of(&snes) == saved, "the save wasn't loaded");

        states.undo_load(&mut snes).unwrap();
        assert!(state_of(&snes) == before_load, "the load wasn't undone");

        states.undo_load(&mut snes).unwrap();
        assert!(state_of(&snes) == saved, "the load wasn't redone");
    }

    #[test]
    fn nothing_to_load_or_undo_is_reported() {
        let (mut snes, mut states, _root) = setup();

        assert!(matches!(
            states.load_latest(&mut snes),
            Err(SaveStateError::NoSaveState)
        ));
        assert!(matches!(
            states.load_slot(&mut snes, 3),
            Err(SaveStateError::NoSaveState)
        ));
        assert!(matches!(
            states.undo_load(&mut snes),
            Err(SaveStateError::NothingToUndo)
        ));
    }

    #[test]
    fn a_state_of_another_rom_is_not_loaded_and_writes_no_undo_file() {
        let (mut snes, mut states, root) = setup();
        let mut other_rom = test_rom();
        other_rom[0x1000] ^= 0xFF; // unused byte
        let other = emu(make_rsnes_from(&other_rom));
        states.save(&other).unwrap();

        let before = state_of(&snes);
        let result = states.load_slot(&mut snes, 1);

        assert!(matches!(result, Err(SaveStateError::WrongRom)));
        assert!(state_of(&snes) == before, "the console changed");
        assert!(!states.undo_path().exists(), "an undo file was written");
        assert!(is_unreadable(states.slot(1)));
        assert!(is_unreadable(reopen(&root, &snes).slot(1)));
    }

    #[test]
    fn a_state_with_a_damaged_sram_is_not_loaded_and_writes_no_undo_file() {
        let mut damaged = make_rsnes_from(&test_rom_with_sram());
        damaged.bus.cart.sram = damaged_sram();
        let root = tempfile::tempdir().unwrap();
        let mut states = GameSaveStates::in_folder(root.path(), TITLE, damaged.rom_checksum);
        states.save(&emu(damaged)).unwrap();
        let mut snes = emu(make_rsnes_from(&test_rom_with_sram()));

        let before = state_of(&snes);
        let result = states.load_slot(&mut snes, 1);

        assert!(matches!(result, Err(SaveStateError::Corrupted(_))));
        assert!(state_of(&snes) == before, "the console changed");
        assert!(!states.undo_path().exists(), "an undo file was written");
        assert!(is_unreadable(states.slot(1)));
    }

    #[test]
    fn reopened_slots_have_their_screen_and_size_or_their_error() {
        let (mut snes, mut states, root) = setup();
        // More than a frame, so that a frame is presented.
        tick_core(&mut snes.core_mut(), 400_000);
        states.save(&snes).unwrap();
        fs::write(states.slot_path(5), b"damaged").unwrap();

        let reopened = reopen(&root, &snes);

        let Slot::Saved {
            screen, file_size, ..
        } = reopened.slot(1)
        else {
            panic!("slot 1 holds no save state");
        };
        assert!(*screen == screen_of(&snes));
        assert_eq!(*file_size, fs::metadata(states.slot_path(1)).unwrap().len());
        let Slot::Unreadable { reason, .. } = reopened.slot(5) else {
            panic!("the damaged slot is readable");
        };
        assert_eq!(*reason, SaveStateError::NotASaveState.to_string());
    }

    #[test]
    fn saving_to_a_chosen_slot_overwrites_it_and_clears_its_name() {
        let (snes, mut states, root) = setup();
        states.save(&snes).unwrap();
        states.rename(1, "before the boss").unwrap();
        let first = states.slot(1).time();
        tick();

        states.save_to(&snes, 1).unwrap();
        states.save_to(&snes, 7).unwrap();

        assert!(states.slot(1).time() > first, "slot 1 wasn't saved over");
        assert_eq!(states.settings(1).name, "");
        assert_eq!(reopen(&root, &snes).settings(1).name, "");
        assert!(matches!(states.slot(7), Slot::Saved { .. }));
        assert!(is_empty(states.slot(2)));
    }

    #[test]
    fn locked_slots_are_skipped_by_saves() {
        let (snes, mut states, _root) = setup();
        for _ in 1..=SLOT_COUNT {
            states.save(&snes).unwrap();
            tick();
        }
        states.set_locked(1, true).unwrap();
        states.set_locked(2, true).unwrap();

        assert_eq!(states.save(&snes).unwrap(), 3, "the oldest unlocked slot");

        for slot in 3..=SLOT_COUNT {
            states.set_locked(slot, true).unwrap();
        }
        assert!(matches!(
            states.save(&snes),
            Err(SaveStateError::AllSlotsLocked)
        ));
    }

    #[test]
    fn a_locked_slot_is_neither_saved_over_nor_deleted() {
        let (snes, mut states, _root) = setup();
        states.save(&snes).unwrap();
        states.set_locked(1, true).unwrap();
        let file = fs::read(states.slot_path(1)).unwrap();

        assert!(matches!(
            states.save_to(&snes, 1),
            Err(SaveStateError::SlotLocked)
        ));
        assert!(matches!(states.delete(1), Err(SaveStateError::SlotLocked)));
        assert_eq!(fs::read(states.slot_path(1)).unwrap(), file);

        states.set_locked(1, false).unwrap();
        states.delete(1).unwrap();
        assert!(is_empty(states.slot(1)));
    }

    #[test]
    fn delete_removes_the_file_and_the_settings() {
        let (snes, mut states, root) = setup();
        states.save(&snes).unwrap();
        states.rename(1, "old").unwrap();

        states.delete(1).unwrap();
        states.delete(2).unwrap(); // already empty

        assert!(!states.slot_path(1).exists());
        assert!(is_empty(states.slot(1)));
        assert_eq!(*states.settings(1), SlotSettings::default());
        assert_eq!(*reopen(&root, &snes).settings(1), SlotSettings::default());
    }

    #[test]
    fn names_and_locks_are_kept_on_disk() {
        let (snes, mut states, root) = setup();
        states.save(&snes).unwrap();
        states.save(&snes).unwrap();
        states
            .rename(1, "  a name that is much too long to be kept whole  ")
            .unwrap();
        states.set_locked(2, true).unwrap();

        let reopened = reopen(&root, &snes);

        assert_eq!(
            reopened.settings(1).name,
            "a name that is much too long to "
        );
        assert_eq!(reopened.settings(1).name.chars().count(), MAX_NAME_LENGTH);
        assert!(!reopened.settings(1).locked);
        assert!(reopened.settings(2).locked);
        assert_eq!(reopened.settings(2).name, "");
    }

    #[test]
    fn a_slot_deleted_by_hand_loses_its_settings() {
        let (snes, mut states, _root) = setup();
        states.save(&snes).unwrap();
        states.rename(1, "gone").unwrap();
        states.set_locked(1, true).unwrap();

        fs::remove_file(states.slot_path(1)).unwrap();
        states.refresh();

        assert_eq!(*states.settings(1), SlotSettings::default());
        assert_eq!(states.save(&snes).unwrap(), 1);
    }

    #[test]
    fn an_empty_slot_has_no_settings() {
        let (_snes, mut states, _root) = setup();

        assert!(matches!(
            states.rename(1, "name"),
            Err(SaveStateError::NoSaveState)
        ));
        assert!(matches!(
            states.set_locked(1, true),
            Err(SaveStateError::NoSaveState)
        ));
        assert!(!states.settings_path().exists());
    }

    #[test]
    fn the_settings_file_is_only_written_when_settings_change() {
        let (snes, mut states, _root) = setup();

        states.save(&snes).unwrap();
        states.rename(1, "").unwrap();
        states.set_locked(1, false).unwrap();
        assert!(!states.settings_path().exists());

        states.set_locked(1, true).unwrap();
        assert!(states.settings_path().exists());
    }

    #[test]
    fn a_damaged_settings_file_counts_as_no_settings() {
        let (snes, mut states, root) = setup();
        states.save(&snes).unwrap();
        states.set_locked(1, true).unwrap();
        fs::write(states.settings_path(), b"damaged").unwrap();

        let reopened = reopen(&root, &snes);

        assert_eq!(*reopened.settings(1), SlotSettings::default());
        assert!(matches!(reopened.slot(1), Slot::Saved { .. }));
    }
}
