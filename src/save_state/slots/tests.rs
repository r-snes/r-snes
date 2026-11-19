use super::*;
use crate::save_state::tests::{console, emu, run, saved_part, test_rom};
use std::thread::sleep;
use std::time::Duration;
use tempfile::TempDir;

const TITLE: &str = "TEST GAME";

/// An emulator running the test ROM, and its save states in a temporary
/// folder, deleted when the returned `TempDir` is dropped.
fn setup() -> (RSnesEmu, GameSaveStates, TempDir) {
    let core = console(&test_rom());
    let root = tempfile::tempdir().unwrap();
    let states = GameSaveStates::in_folder(root.path(), TITLE, core.rom_checksum);
    (emu(core), states, root)
}

/// The save states of the test ROM in `root`, read from the disk.
fn reopen(root: &TempDir, snes: &mut RSnesEmu) -> GameSaveStates {
    GameSaveStates::in_folder(root.path(), TITLE, snes.core_mut().rom_checksum)
}

fn state_of(snes: &mut RSnesEmu) -> Vec<u8> {
    saved_part(&snes.core_mut())
}

fn screen_of(snes: &mut RSnesEmu) -> Box<RawFramebuffer> {
    snes.core_mut().ppu_renderer.presented.clone()
}

fn advance(snes: &mut RSnesEmu) {
    run(&mut snes.core_mut(), 20_000);
}

/// Leaves time between two writes, so that their times differ.
fn tick() {
    sleep(Duration::from_millis(2));
}

fn is_empty(slot: &Slot) -> bool {
    matches!(slot, Slot::Empty)
}

fn is_unreadable(slot: &Slot) -> bool {
    matches!(slot, Slot::Unreadable { .. })
}

/// The screen of a slot holding a save state.
fn saved_screen(slot: &Slot) -> &RawFramebuffer {
    match slot {
        Slot::Saved { screen, .. } => screen,
        _ => panic!("the slot holds no save state"),
    }
}

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
fn save_states_are_in_the_home_folder() {
    // Only reads: nothing is created.
    let Some(home) = std::env::home_dir() else {
        return;
    };
    let states = GameSaveStates::for_rom("GAME", 0x1234_5678).unwrap();

    assert_eq!(
        states.slot_path(1),
        home.join(ROOT_FOLDER_NAME)
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
    run(&mut snes.core_mut(), 400_000); // more than a frame, so a frame is presented

    let slot = states.save(&mut snes).unwrap();

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
    assert!(*screen == screen_of(&mut snes));
    assert_eq!(*file_size, file.len() as u64);
}

#[test]
fn opening_the_folder_reads_the_slots_on_disk() {
    let (mut snes, mut states, root) = setup();
    states.save(&mut snes).unwrap();
    tick();
    states.save(&mut snes).unwrap();
    fs::write(states.slot_path(5), b"damaged").unwrap();

    let reopened = reopen(&root, &mut snes);

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
    let (mut snes, mut states, _root) = setup();

    for expected in 1..=SLOT_COUNT {
        assert_eq!(states.save(&mut snes).unwrap(), expected);
        tick();
    }
    assert_eq!(states.save(&mut snes).unwrap(), 1);
    assert_eq!(
        states.latest_slot(),
        Some(1),
        "the latest isn't the highest"
    );
    tick();
    assert_eq!(states.save(&mut snes).unwrap(), 2);

    assert!((1..=SLOT_COUNT).all(|slot| !is_empty(states.slot(slot))));
}

#[test]
fn a_slot_deleted_by_hand_is_reused_after_a_refresh() {
    let (mut snes, mut states, _root) = setup();
    for _ in 1..=3 {
        states.save(&mut snes).unwrap();
        tick();
    }

    fs::remove_file(states.slot_path(2)).unwrap();
    states.refresh();

    assert!(is_empty(states.slot(2)));
    assert_eq!(states.save(&mut snes).unwrap(), 2);
    assert_eq!(states.save(&mut snes).unwrap(), 4);
}

#[test]
fn a_slot_deleted_by_hand_is_noticed_on_load() {
    let (mut snes, mut states, _root) = setup();
    states.save(&mut snes).unwrap();

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
    states.save(&mut snes).unwrap();
    tick();
    advance(&mut snes);
    states.save(&mut snes).unwrap();
    let saved = state_of(&mut snes);

    advance(&mut snes);
    assert_eq!(states.load_latest(&mut snes).unwrap(), 2);

    assert!(state_of(&mut snes) == saved, "the save wasn't restored");
}

#[test]
fn load_latest_skips_unreadable_slots() {
    let (mut snes, mut states, _root) = setup();
    states.save(&mut snes).unwrap();
    let saved = state_of(&mut snes);
    tick();
    fs::write(states.slot_path(2), b"damaged").unwrap();
    states.refresh();

    advance(&mut snes);
    assert_eq!(states.load_latest(&mut snes).unwrap(), 1);

    assert!(state_of(&mut snes) == saved, "the save wasn't restored");
}

#[test]
fn undo_restores_the_state_before_the_load_and_undoing_again_redoes_it() {
    let (mut snes, mut states, _root) = setup();
    advance(&mut snes);
    states.save(&mut snes).unwrap();
    let saved = state_of(&mut snes);
    advance(&mut snes);
    let before_load = state_of(&mut snes);

    states.load_latest(&mut snes).unwrap();
    assert!(state_of(&mut snes) == saved, "the save wasn't loaded");

    states.undo_load(&mut snes).unwrap();
    assert!(state_of(&mut snes) == before_load, "the load wasn't undone");

    states.undo_load(&mut snes).unwrap();
    assert!(state_of(&mut snes) == saved, "the load wasn't redone");
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
    let mut other = emu(console(&other_rom));
    states.save(&mut other).unwrap();

    let before = state_of(&mut snes);
    let result = states.load_slot(&mut snes, 1);

    assert!(matches!(result, Err(SaveStateError::WrongRom)));
    assert!(state_of(&mut snes) == before, "the console changed");
    assert!(!states.undo_path().exists(), "an undo file was written");
    assert!(is_unreadable(states.slot(1)));
    assert!(is_unreadable(reopen(&root, &mut snes).slot(1)));
}

#[test]
fn reopened_slots_have_their_screen_and_size_or_their_error() {
    let (mut snes, mut states, root) = setup();
    // More than a frame, so that a frame is presented.
    run(&mut snes.core_mut(), 400_000);
    states.save(&mut snes).unwrap();
    fs::write(states.slot_path(5), b"damaged").unwrap();

    let reopened = reopen(&root, &mut snes);

    let Slot::Saved {
        screen, file_size, ..
    } = reopened.slot(1)
    else {
        panic!("slot 1 holds no save state");
    };
    assert!(*screen == screen_of(&mut snes));
    assert_eq!(*file_size, fs::metadata(states.slot_path(1)).unwrap().len());
    let Slot::Unreadable { reason, .. } = reopened.slot(5) else {
        panic!("the damaged slot is readable");
    };
    assert_eq!(*reason, SaveStateError::NotASaveState.to_string());
}

#[test]
fn saving_to_a_chosen_slot_overwrites_it_and_clears_its_name() {
    let (mut snes, mut states, root) = setup();
    states.save(&mut snes).unwrap();
    states.rename(1, "before the boss").unwrap();
    let first = states.slot(1).time();
    tick();

    states.save_to(&mut snes, 1).unwrap();
    states.save_to(&mut snes, 7).unwrap();

    assert!(states.slot(1).time() > first, "slot 1 wasn't saved over");
    assert_eq!(states.settings(1).name, "");
    assert_eq!(reopen(&root, &mut snes).settings(1).name, "");
    assert!(matches!(states.slot(7), Slot::Saved { .. }));
    assert!(is_empty(states.slot(2)));
}

#[test]
fn locked_slots_are_skipped_by_saves() {
    let (mut snes, mut states, _root) = setup();
    for _ in 1..=SLOT_COUNT {
        states.save(&mut snes).unwrap();
        tick();
    }
    states.set_locked(1, true).unwrap();
    states.set_locked(2, true).unwrap();

    assert_eq!(
        states.save(&mut snes).unwrap(),
        3,
        "the oldest unlocked slot"
    );

    for slot in 3..=SLOT_COUNT {
        states.set_locked(slot, true).unwrap();
    }
    assert!(matches!(
        states.save(&mut snes),
        Err(SaveStateError::AllSlotsLocked)
    ));
}

#[test]
fn a_locked_slot_is_neither_saved_over_nor_deleted() {
    let (mut snes, mut states, _root) = setup();
    states.save(&mut snes).unwrap();
    states.set_locked(1, true).unwrap();
    let file = fs::read(states.slot_path(1)).unwrap();

    assert!(matches!(
        states.save_to(&mut snes, 1),
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
    let (mut snes, mut states, root) = setup();
    states.save(&mut snes).unwrap();
    states.rename(1, "old").unwrap();

    states.delete(1).unwrap();
    states.delete(2).unwrap(); // already empty

    assert!(!states.slot_path(1).exists());
    assert!(is_empty(states.slot(1)));
    assert_eq!(*states.settings(1), SlotSettings::default());
    assert_eq!(
        *reopen(&root, &mut snes).settings(1),
        SlotSettings::default()
    );
}

#[test]
fn names_and_locks_are_kept_on_disk() {
    let (mut snes, mut states, root) = setup();
    states.save(&mut snes).unwrap();
    states.save(&mut snes).unwrap();
    states
        .rename(1, "  a name that is much too long to be kept whole  ")
        .unwrap();
    states.set_locked(2, true).unwrap();

    let reopened = reopen(&root, &mut snes);

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
    let (mut snes, mut states, _root) = setup();
    states.save(&mut snes).unwrap();
    states.rename(1, "gone").unwrap();
    states.set_locked(1, true).unwrap();

    fs::remove_file(states.slot_path(1)).unwrap();
    states.refresh();

    assert_eq!(*states.settings(1), SlotSettings::default());
    assert_eq!(states.save(&mut snes).unwrap(), 1);
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
    let (mut snes, mut states, _root) = setup();

    states.save(&mut snes).unwrap();
    states.rename(1, "").unwrap();
    states.set_locked(1, false).unwrap();
    assert!(!states.settings_path().exists());

    states.set_locked(1, true).unwrap();
    assert!(states.settings_path().exists());
}

#[test]
fn a_damaged_settings_file_counts_as_no_settings() {
    let (mut snes, mut states, root) = setup();
    states.save(&mut snes).unwrap();
    states.set_locked(1, true).unwrap();
    fs::write(states.settings_path(), b"damaged").unwrap();

    let reopened = reopen(&root, &mut snes);

    assert_eq!(*reopened.settings(1), SlotSettings::default());
    assert!(matches!(reopened.slot(1), Slot::Saved { .. }));
}
