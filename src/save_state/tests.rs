use super::*;
use crate::dma::DmaState;
use bus::cartridge::test_rom::{create_temp_rom, create_valid_lorom};
use cpu::cpu::CPUState;
use ppu::constants::{MASTER_CYCLES_PER_SCANLINE, SCANLINES_PER_FRAME};

const MASTER_CYCLES_PER_FRAME: u64 = MASTER_CYCLES_PER_SCANLINE as u64 * SCANLINES_PER_FRAME as u64;

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
pub(super) fn test_rom() -> Vec<u8> {
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

pub(super) fn console(rom: &[u8]) -> RSnesCore {
    let (path, _dir) = create_temp_rom(rom);
    RSnesCore::load_rom(&path).unwrap()
}

pub(super) fn run(core: &mut RSnesCore, master_cycles: u64) {
    for _ in 0..master_cycles {
        core.update();
    }
}

/// Runs the console until `condition` holds, at most two frames.
fn run_until(core: &mut RSnesCore, what: &str, condition: fn(&RSnesCore) -> bool) {
    for _ in 0..2 * MASTER_CYCLES_PER_FRAME {
        if condition(core) {
            return;
        }
        core.update();
    }
    panic!("the console never got {what} within two frames");
}

/// Saves `from` and loads the state into `to`.
fn copy_state(from: &RSnesCore, to: &mut RSnesCore) {
    let bytes = from.save_state().unwrap();
    to.load_state(SaveState::decode(&bytes).unwrap()).unwrap();
}

/// The saved part of a console, as written in a file.
pub(super) fn saved_part(core: &RSnesCore) -> Vec<u8> {
    core.encode_core(Vec::new()).unwrap()
}

/// Checks that the saved parts of two consoles are identical, naming the
/// first component that differs. Uses `assert!`, not `assert_eq!`: printing
/// these large byte vectors would be useless.
fn assert_same_state(a: &RSnesCore, b: &RSnesCore, when: &str) {
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

fn mid_instruction(core: &RSnesCore) -> bool {
    core.cpu.state() == CPUState::Running
        && core.dma.state == DmaState::Idle
        && !core.cpu.is_instr_start()
}

/// An emulator running `core` without a plugin.
#[cfg(feature = "plugins")]
pub(super) fn emu(core: RSnesCore) -> RSnesEmu {
    RSnesEmu::new_with_plugin(core, None).unwrap()
}

/// An emulator running `core`.
#[cfg(not(feature = "plugins"))]
pub(super) fn emu(core: RSnesCore) -> RSnesEmu {
    RSnesEmu::new(core)
}

fn decode_error(bytes: &[u8]) -> SaveStateError {
    SaveState::decode(bytes).err().expect("decoding must fail")
}

#[test]
fn the_test_rom_runs() {
    let mut core = console(&test_rom());
    run(&mut core, 3 * MASTER_CYCLES_PER_FRAME);

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
    let mut original = console(&rom);
    let mut copy = console(&rom);
    run(&mut copy, 12_345);

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

        run(&mut original, run_after);
        run(&mut copy, run_after);
        assert_same_state(&original, &copy, &format!("{when}, then running"));

        // Move the copy away, so the next load starts from another state,
        // even if the next save point is already reached.
        run(&mut copy, 777);
    }
}

#[test]
fn info_records_the_rom_and_the_version() {
    let core = console(&test_rom());

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
    let mut emu = emu(console(&test_rom()));
    run(&mut emu.core_mut(), 50_000);
    let bytes = emu.save_state().unwrap();
    let saved = saved_part(&emu.core_mut());

    run(&mut emu.core_mut(), 50_000);
    emu.load_state(SaveState::decode(&bytes).unwrap()).unwrap();

    assert!(
        saved_part(&emu.core_mut()) == saved,
        "the state wasn't restored"
    );
}

#[cfg(feature = "plugins")]
#[test]
fn loading_restarts_the_plugin_interval() {
    let plugin = plugins::plugin::Plugin::load_from_raw(b"return { permissions = {} }", None);
    let mut emu = RSnesEmu::new_with_plugin(console(&test_rom()), Some(plugin.unwrap())).unwrap();
    let bytes = emu.save_state().unwrap();

    emu.plugin_mut().unwrap().next_interval_master_cycle = Some(123_456);
    emu.load_state(SaveState::decode(&bytes).unwrap()).unwrap();

    assert_eq!(emu.plugin_mut().unwrap().next_interval_master_cycle, None);
}

#[test]
fn the_screen_is_the_presented_frame() {
    let mut core = console(&test_rom());
    run(&mut core, 2 * MASTER_CYCLES_PER_FRAME + 5_000);

    let state = SaveState::decode(&core.save_state().unwrap()).unwrap();

    assert!(state.screen() == core.ppu_renderer.presented());
}

#[test]
fn a_state_of_another_rom_is_refused_and_changes_nothing() {
    let rom = test_rom();
    let mut other_rom = rom.clone();
    other_rom[0x1000] ^= 0xFF; // unused byte

    let mut other = console(&other_rom);
    run(&mut other, 10_000);
    let mut core = console(&rom);
    run(&mut core, 20_000);
    let before = saved_part(&core);

    let state = SaveState::decode(&other.save_state().unwrap()).unwrap();
    let result = core.load_state(state);

    assert!(matches!(result, Err(SaveStateError::WrongRom)));
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
fn a_state_of_another_emulator_version_is_refused() {
    let core = console(&test_rom());
    let info = SaveState::read_info(&core.save_state().unwrap()).unwrap();
    let older = SaveStateInfo {
        emulator_version: "0.0.1".into(),
        ..info
    };
    let file = postcard::to_extend(&older, MAGIC.to_vec()).unwrap();
    let bytes = core.encode_core(file).unwrap();

    assert!(matches!(
        decode_error(&bytes),
        SaveStateError::UnsupportedVersion { found } if found == "0.0.1"
    ));
    assert!(matches!(
        SaveState::read_info(&bytes),
        Err(SaveStateError::UnsupportedVersion { .. })
    ));
}

#[test]
fn a_truncated_file_is_corrupted() {
    let bytes = console(&test_rom()).save_state().unwrap();

    // Without the version, inside it, inside the rest of the info, then
    // inside the console state.
    for length in [
        MAGIC.len(),
        MAGIC.len() + 2,
        MAGIC.len() + 1 + EMULATOR_VERSION.len() + 1,
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
    let mut bytes = console(&test_rom()).save_state().unwrap();
    bytes.extend_from_slice(b"extra");

    assert!(matches!(decode_error(&bytes), SaveStateError::Corrupted(_)));
}

#[test]
fn every_error_has_a_message() {
    let errors = [
        SaveStateError::NotASaveState,
        SaveStateError::UnsupportedVersion {
            found: "0.0.1".into(),
        },
        SaveStateError::WrongRom,
        SaveStateError::Corrupted("detail".into()),
        SaveStateError::Encoding("detail".into()),
        SaveStateError::from(io::Error::other("disk on fire")),
        SaveStateError::NoSaveState,
        SaveStateError::NothingToUndo,
        SaveStateError::NoHomeFolder,
    ];

    for error in errors {
        assert!(error.to_string().ends_with('.'), "{error}");
    }
    let io_error = SaveStateError::from(io::Error::other("disk on fire"));
    assert!(std::error::Error::source(&io_error).is_some());
}
