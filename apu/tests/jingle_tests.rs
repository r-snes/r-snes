//! Tests for the "no ROM loaded" jingle: golden bytes for the
//! hand-assembled SPC700 driver, and end-to-end playback through the
//! full IPL boot protocol.

use apu::jingle::{build_brr, build_driver, build_song, IdleJingle, MELODY, pitch_of};

/// Golden bytes verified by simulating the driver instruction-by-
/// instruction on a reference SPC700 interpreter (opcode semantics,
/// branch displacements, and the full DSP write sequence over two
/// song loops were all checked). If this test fails after an edit to
/// `build_driver`, the driver must be re-verified — a wrong branch
/// displacement produces garbage that no compiler will catch.
#[test]
fn test_driver_matches_verified_golden_bytes() {
    const GOLDEN_DRIVER: [u8; 122] = [
        0x8F, 0x6C, 0xF2, 0x8F, 0x20, 0xF3, 0x8F, 0x0C, 0xF2, 0x8F, 0x7F, 0xF3,
        0x8F, 0x1C, 0xF2, 0x8F, 0x7F, 0xF3, 0x8F, 0x5D, 0xF2, 0x8F, 0x03, 0xF3,
        0x8F, 0x00, 0xF2, 0x8F, 0x48, 0xF3, 0x8F, 0x01, 0xF2, 0x8F, 0x48, 0xF3,
        0x8F, 0x04, 0xF2, 0x8F, 0x00, 0xF3, 0x8F, 0x05, 0xF2, 0x8F, 0xEF, 0xF3,
        0x8F, 0x06, 0xF2, 0x8F, 0xB4, 0xF3, 0x8F, 0x7D, 0xFA, 0x8F, 0x01, 0xF1,
        0xCD, 0x00, 0xF5, 0xC0, 0x02, 0x68, 0xFF, 0xF0, 0xF7, 0x68, 0xFE, 0xF0,
        0x29, 0x8F, 0x03, 0xF2, 0xC4, 0xF3, 0x3D, 0xF5, 0xC0, 0x02, 0x8F, 0x02,
        0xF2, 0xC4, 0xF3, 0x3D, 0xF5, 0xC0, 0x02, 0x3D, 0xFD, 0x8F, 0x4C, 0xF2,
        0x8F, 0x01, 0xF3, 0xE4, 0xFD, 0xF0, 0xFC, 0xDC, 0xD0, 0xF9, 0x8F, 0x5C,
        0xF2, 0x8F, 0x01, 0xF3, 0x2F, 0xCC, 0x3D, 0xF5, 0xC0, 0x02, 0x3D, 0xFD,
        0x2F, 0xE9,
    ];
    assert_eq!(build_driver(), GOLDEN_DRIVER);
}

#[test]
fn test_song_matches_verified_golden_bytes() {
    const GOLDEN_SONG: [u8; 47] = [
        0x04, 0x30, 0x10, 0x05, 0x46, 0x10, 0x06, 0x46, 0x10, 0x07, 0x0A, 0x10,
        0x06, 0x46, 0x10, 0x05, 0x46, 0x10, 0x06, 0x46, 0x18, 0xFE, 0x08, 0x07,
        0x0A, 0x10, 0x08, 0x5F, 0x10, 0x09, 0x66, 0x10, 0x08, 0x5F, 0x10, 0x06,
        0x46, 0x18, 0x05, 0x46, 0x10, 0x04, 0x30, 0x10, 0xFE, 0x08, 0xFF,
    ];
    assert_eq!(build_song(), GOLDEN_SONG);
}

#[test]
fn test_brr_matches_verified_golden_bytes() {
    const GOLDEN_BRR: [u8; 9] = [0xB3, 0x03, 0x56, 0x54, 0x21, 0x0F, 0xEC, 0xBA, 0xBD];
    assert_eq!(build_brr(), GOLDEN_BRR);
}

/// End-to-end: boot the APU through the real IPL protocol, and the
/// jingle must actually produce sound. This exercises the whole
/// stack: IPL HLE, upload protocol, SPC700 execution, timer 0, DSP
/// pitch/KON/ADSR, BRR decode, and sample collection.
#[test]
fn test_jingle_boots_and_produces_audio() {
    let mut jingle = IdleJingle::new().expect("IPL upload must succeed");
    let samples = jingle.generate(32_000); // 1 second
    assert_eq!(samples.len(), 32_000 * 2, "stereo interleaved output");
    assert!(
        samples.iter().any(|&s| s != 0),
        "one second of jingle produced pure silence"
    );
}

/// Tempo sanity: 128 ms after boot, the first note (250 ms long) must
/// still be playing, i.e. voice 0's pitch registers hold the first
/// note's pitch. If this fails while the test above passes, the most
/// likely cause is the SPC700 core reading $FD through `read8` instead
/// of `read8_mut` — without clear-on-read, the driver's tick-wait loop
/// falls through instantly and the whole song plays in a blur.
#[test]
fn test_first_note_still_playing_at_128ms() {
    let mut jingle = IdleJingle::new().expect("IPL upload must succeed");
    jingle.generate(4_096); // 128 ms

    let pitch = u16::from_le_bytes([
        jingle.apu().memory.dsp.read_reg(0x02),
        jingle.apu().memory.dsp.read_reg(0x03),
    ]);
    let first = pitch_of(MELODY[0].0.unwrap());
    assert_eq!(
        pitch, first,
        "expected the first note ({first:#06x}) to still be sounding"
    );
}
