//! Utility functions for the unit tests of save states: a test ROM that
//! keeps the console busy, consoles and emulators running it, and save
//! state folders in temporary folders.

// Test helpers: left out of the coverage report.
#![cfg(not(tarpaulin_include))]

use super::slots::{GameSaveStates, Slot};
use super::{SaveState, SaveStateError};
use crate::dma::DmaState;
use crate::rsnes::{RSnesCore, RSnesEmu};
use crate::test_utils::{make_rsnes_from, tick_core};
use bus::cartridge::test_rom::create_valid_lorom;
use cpu::cpu::CPUState;
use ppu::constants::{MASTER_CYCLES_PER_SCANLINE, SCANLINES_PER_FRAME};
use ppu::rendering::RawFramebuffer;
use std::thread::sleep;
use std::time::Duration;
use tempfile::TempDir;

/// Master cycles in one frame.
pub const MASTER_CYCLES_PER_FRAME: u64 =
    MASTER_CYCLES_PER_SCANLINE as u64 * SCANLINES_PER_FRAME as u64;

/// Test ROM setup, at `$00:8000` (ROM offset 0).
#[rustfmt::skip]
const SETUP: &[u8] = &[
    0x78,             // SEI
    0x18,             // CLC
    0xFB,             // XCE            native mode
    0xC2, 0x10,       // REP #$10       16-bit X/Y
    0xE2, 0x20,       // SEP #$20       8-bit A
    0xA2, 0xFF, 0x1F, // LDX #$1FFF
    0x9A,             // TXS            stack at $1FFF
    0xA9, 0x0F,       // LDA #$0F
    0x8D, 0x00, 0x21, // STA $2100      full brightness
    0xA9, 0x01,       // LDA #$01
    0x8D, 0x05, 0x21, // STA $2105      BG mode 1
    0x8D, 0x2C, 0x21, // STA $212C      BG1 on the main screen
    // HDMA channel 1: mode 0, to $2100, table at $00:8200
    0x9C, 0x10, 0x43, // STZ $4310
    0x9C, 0x11, 0x43, // STZ $4311
    0xA2, 0x00, 0x82, // LDX #$8200
    0x8E, 0x12, 0x43, // STX $4312
    0x9C, 0x14, 0x43, // STZ $4314
    0xA9, 0x02,       // LDA #$02
    0x8D, 0x0C, 0x42, // STA $420C      enable HDMA channel 1
    0xA9, 0x81,       // LDA #$81
    0x8D, 0x00, 0x42, // STA $4200      NMI + auto-joypad
];

/// Test ROM main loop, right after the setup.
#[rustfmt::skip]
const LOOP: &[u8] = &[
    0xEE, 0x00, 0x00, // INC $0000
    0xAD, 0x00, 0x00, // LDA $0000
    0x8D, 0x02, 0x42, // STA $4202      multiplier
    0x8D, 0x03, 0x42, // STA $4203
    0x8D, 0x40, 0x21, // STA $2140      APU port
    0x9C, 0x21, 0x21, // STZ $2121      CGRAM
    0x8D, 0x22, 0x21, // STA $2122
    0x8D, 0x22, 0x21, // STA $2122
    // DMA channel 0: mode 1, 2 KB from WRAM $00:0000 to $2118
    0xA9, 0x01,       // LDA #$01
    0x8D, 0x00, 0x43, // STA $4300
    0xA9, 0x18,       // LDA #$18
    0x8D, 0x01, 0x43, // STA $4301
    0xA2, 0x00, 0x00, // LDX #$0000
    0x8E, 0x02, 0x43, // STX $4302
    0x9C, 0x04, 0x43, // STZ $4304
    0xA2, 0x00, 0x08, // LDX #$0800
    0x8E, 0x05, 0x43, // STX $4305
    0xA9, 0x80,       // LDA #$80
    0x8D, 0x15, 0x21, // STA $2115      VRAM increment after the high byte
    0x9C, 0x16, 0x21, // STZ $2116
    0x9C, 0x17, 0x21, // STZ $2117
    0xA9, 0x01,       // LDA #$01
    0x8D, 0x0B, 0x42, // STA $420B      start the DMA
    0xCB,             // WAI
    // followed by BRA to the start of the loop, see `test_rom`
];

/// NMI handler, at `$00:8100`.
#[rustfmt::skip]
const NMI_HANDLER: &[u8] = &[
    0xEE, 0x10, 0x00, // INC $0010
    0xAD, 0x10, 0x42, // LDA $4210      acknowledge the NMI
    0x40,             // RTI
];

/// HDMA table, at `$00:8200`: 16 lines at brightness 15, 16 lines at 8.
const HDMA_TABLE: &[u8] = &[0x10, 0x0F, 0x10, 0x08, 0x00];

/// A LoROM that keeps the console busy: DMA, HDMA, NMI, multiplier, APU
/// port, CGRAM writes and WAI, every frame.
pub fn test_rom() -> Vec<u8> {
    let mut program = [SETUP, LOOP].concat();
    let bra_offset = SETUP.len() as isize - (program.len() as isize + 2);
    program.extend([0x80, i8::try_from(bra_offset).unwrap() as u8]); // BRA loop

    let mut rom = create_valid_lorom(0x20000);
    rom[..program.len()].copy_from_slice(&program);
    rom[0x100..0x100 + NMI_HANDLER.len()].copy_from_slice(NMI_HANDLER);
    rom[0x200..0x200 + HDMA_TABLE.len()].copy_from_slice(HDMA_TABLE);
    rom[0x7FEA..0x7FEC].copy_from_slice(&[0x00, 0x81]); // native NMI vector: $8100
    rom[0x7FFC..0x7FFE].copy_from_slice(&[0x00, 0x80]); // reset vector: $8000
    rom
}

/// Runs the console until `condition` holds, at most two frames.
pub fn run_until(core: &mut RSnesCore, what: &str, condition: fn(&RSnesCore) -> bool) {
    for _ in 0..2 * MASTER_CYCLES_PER_FRAME {
        if condition(core) {
            return;
        }
        core.update();
    }
    panic!("the console never got {what} within two frames");
}

/// Saves `from` and loads the state into `to`.
pub fn copy_state(from: &RSnesCore, to: &mut RSnesCore) {
    let bytes = from.save_state().unwrap();
    to.load_state(SaveState::decode(&bytes).unwrap()).unwrap();
}

/// The saved part of a console, as written in a file.
pub fn saved_part(core: &RSnesCore) -> Vec<u8> {
    core.encode_core(Vec::new()).unwrap()
}

/// Checks that the saved parts of two consoles are identical, naming the
/// first component that differs. Uses `assert!`, not `assert_eq!`: printing
/// these large byte vectors would be useless.
pub fn assert_same_state(a: &RSnesCore, b: &RSnesCore, when: &str) {
    fn components(core: &RSnesCore) -> [(&'static str, Vec<u8>); 9] {
        let encode = |value: &dyn erased::Encode| value.encode();
        [
            ("WRAM", encode(&core.bus.wram)),
            ("I/O registers", encode(&core.bus.io)),
            ("S-RAM", encode(&core.bus.cart.sram)),
            ("CPU", encode(&core.cpu)),
            ("PPU", encode(&core.ppu)),
            ("renderer", encode(&core.ppu_renderer)),
            ("APU", encode(&core.apu)),
            ("DMA", encode(&core.dma)),
            (
                "scheduler counters",
                encode(&(
                    core.master_cycles,
                    core.cpu_master_cycles_to_wait,
                    core.apu_cycle_debt,
                    core.nmi_line,
                )),
            ),
        ]
    }

    for ((name, a), (_, b)) in components(a).into_iter().zip(components(b)) {
        assert!(a == b, "{when}: the {name} differ");
    }
    assert!(
        saved_part(a) == saved_part(b),
        "{when}: the saved states differ"
    );
}

/// Lets `assert_same_state` list components of different types.
mod erased {
    pub trait Encode {
        fn encode(&self) -> Vec<u8>;
    }

    impl<T: serde::Serialize> Encode for T {
        fn encode(&self) -> Vec<u8> {
            postcard::to_stdvec(self).unwrap()
        }
    }
}

/// Whether the CPU is in the middle of an instruction, outside DMA.
pub fn mid_instruction(core: &RSnesCore) -> bool {
    core.cpu.state() == CPUState::Running
        && core.dma.state == DmaState::Idle
        && !core.cpu.is_instr_start()
}

/// An emulator running `core`.
pub fn emu(core: RSnesCore) -> RSnesEmu {
    RSnesEmu::new(core)
}

/// The error of decoding `bytes`, which must fail.
pub fn decode_error(bytes: &[u8]) -> SaveStateError {
    SaveState::decode(bytes).err().expect("decoding must fail")
}

// Save state folders.

/// Title of the game whose save states the tests write.
pub const TITLE: &str = "TEST GAME";

/// An emulator running the test ROM, and its save states in a temporary
/// folder, deleted when the returned `TempDir` is dropped.
pub fn setup() -> (RSnesEmu, GameSaveStates, TempDir) {
    let core = make_rsnes_from(&test_rom());
    let root = tempfile::tempdir().unwrap();
    let states = GameSaveStates::in_folder(root.path(), TITLE, core.rom_checksum);
    (emu(core), states, root)
}

/// The save states of the test ROM in `root`, read from the disk.
pub fn reopen(root: &TempDir, snes: &mut RSnesEmu) -> GameSaveStates {
    GameSaveStates::in_folder(root.path(), TITLE, snes.core_mut().rom_checksum)
}

/// The saved part of the emulator's console.
pub fn state_of(snes: &mut RSnesEmu) -> Vec<u8> {
    saved_part(&snes.core_mut())
}

/// The last frame the emulator presented.
pub fn screen_of(snes: &mut RSnesEmu) -> Box<RawFramebuffer> {
    snes.core_mut().ppu_renderer.presented.clone()
}

/// Runs the emulator for a moment, so that its state changes.
pub fn advance(snes: &mut RSnesEmu) {
    tick_core(&mut snes.core_mut(), 20_000);
}

/// Leaves time between two writes, so that their times differ.
pub fn tick() {
    sleep(Duration::from_millis(2));
}

/// Whether the slot holds no file.
pub fn is_empty(slot: &Slot) -> bool {
    matches!(slot, Slot::Empty)
}

/// Whether the slot's file can't be read.
pub fn is_unreadable(slot: &Slot) -> bool {
    matches!(slot, Slot::Unreadable { .. })
}

/// The screen of a slot holding a save state.
pub fn saved_screen(slot: &Slot) -> &RawFramebuffer {
    match slot {
        Slot::Saved { screen, .. } => screen,
        _ => panic!("the slot holds no save state"),
    }
}
