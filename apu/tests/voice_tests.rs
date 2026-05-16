/// Voice and per-voice register mapping tests
///
/// Covers Voice and Brr default state, all per-voice DSP register
/// mappings (VOL, PITCH, SRCN, ADSR1, ADSR2), and independence
/// across all 8 voices.

use apu::dsp::{Brr, EnvelopePhase, Voice};
use apu::Memory;

// ============================================================
// Helpers
// ============================================================

const DSP_BASE: u16 = 0xF200;

fn dsp_vw(mem: &mut Memory, voice: u8, reg: u8, val: u8) {
    mem.write8(DSP_BASE + ((voice as u16) << 4) + reg as u16, val);
}

// ============================================================
// Voice — default state
// ============================================================

#[test]
fn test_voice_default() {
    let v = Voice::default();
    assert_eq!(v.left_vol,       0);
    assert_eq!(v.right_vol,      0);
    assert_eq!(v.pitch,          0);
    assert_eq!(v.srcn,           0);
    assert!(!v.key_on);
    assert_eq!(v.pitch_counter,  0);
    assert_eq!(v.current_sample, 0);
    assert_eq!(v.adsr.envelope_phase, EnvelopePhase::Off);
    assert_eq!(v.adsr.envelope_level, 0);
    assert_eq!(v.brr.addr,        0);
    assert_eq!(v.brr.nibble_idx,  0);
    assert_eq!(v.brr.buffer_fill, 0);
}

#[test]
fn test_brr_default_all_zero() {
    let brr = Brr::default();
    assert_eq!(brr.addr,         0, "addr must be 0");
    assert_eq!(brr.nibble_idx,   0, "nibble_idx must be 0");
    assert_eq!(brr.prev1,        0, "prev1 must be 0");
    assert_eq!(brr.prev2,        0, "prev2 must be 0");
    assert_eq!(brr.loop_addr,    0, "loop_addr must be 0");
    assert_eq!(brr.buffer_fill,  0, "buffer_fill must be 0 (no block decoded)");
    assert_eq!(brr.sample_buffer, [0i16; 16], "sample_buffer must be all-zero");
}

// ============================================================
// Dsp — per-voice register mapping (voice N at index N*0x10)
// ============================================================

#[test]
fn test_vol_left_maps_to_correct_voice() {
    let mut mem = Memory::new();
    // Voice 0: reg 0x00, Voice 3: reg 0x30, Voice 7: reg 0x70
    dsp_vw(&mut mem, 0, 0x0, 0x10);
    dsp_vw(&mut mem, 3, 0x0, 0x30);
    dsp_vw(&mut mem, 7, 0x0, 0x70);
    assert_eq!(mem.dsp.voices[0].left_vol, 0x10i8);
    assert_eq!(mem.dsp.voices[3].left_vol, 0x30i8);
    assert_eq!(mem.dsp.voices[7].left_vol, 0x70i8);
}

#[test]
fn test_vol_right_maps_to_correct_voice() {
    let mut mem = Memory::new();
    dsp_vw(&mut mem, 0, 0x1, 0x55);
    dsp_vw(&mut mem, 5, 0x1, 0x22);
    assert_eq!(mem.dsp.voices[0].right_vol, 0x55u8 as i8);
    assert_eq!(mem.dsp.voices[5].right_vol, 0x22u8 as i8);
}

#[test]
fn test_pitch_low_high_bytes_combine_correctly() {
    let mut mem = Memory::new();
    // Voice 1: PITCH low = 0xAB, PITCH high = 0x3C (only 6 bits = 0x3C & 0x3F = 0x3C)
    dsp_vw(&mut mem, 1, 0x2, 0xAB); // low
    dsp_vw(&mut mem, 1, 0x3, 0x3C); // high (14-bit → bits 13-8 = 0x3C & 0x3F)
    let expected: u16 = ((0x3C_u16 & 0x3F) << 8) | 0xAB;
    assert_eq!(mem.dsp.voices[1].pitch, expected);
}

#[test]
fn test_pitch_clamped_to_14_bits() {
    let mut mem = Memory::new();
    // Write 0xFF to high byte; only low 6 bits should survive.
    dsp_vw(&mut mem, 0, 0x3, 0xFF);
    assert_eq!(mem.dsp.voices[0].pitch & !0x3FFF, 0, "bits above 13 must be zero");
}

#[test]
fn test_srcn_register_written_correctly() {
    let mut mem = Memory::new();
    dsp_vw(&mut mem, 2, 0x4, 0x1F);
    assert_eq!(mem.dsp.voices[2].srcn, 0x1F);
}

#[test]
fn test_adsr1_register_layout() {
    // ADSR1 = EDDDAAAA
    //   bit 7   = adsr_mode
    //   bits 6-4 = decay_rate
    //   bits 3-0 = attack_rate
    let mut mem = Memory::new();
    // 0b1_011_0101 = 0xB5 → mode=1, decay=3, attack=5
    dsp_vw(&mut mem, 0, 0x5, 0xB5);
    assert!(mem.dsp.voices[0].adsr.adsr_mode);
    assert_eq!(mem.dsp.voices[0].adsr.decay_rate,  0x03);
    assert_eq!(mem.dsp.voices[0].adsr.attack_rate, 0x05);
}

#[test]
fn test_adsr1_mode_bit_zero() {
    let mut mem = Memory::new();
    dsp_vw(&mut mem, 0, 0x5, 0x35); // bit 7 = 0
    assert!(!mem.dsp.voices[0].adsr.adsr_mode);
}

#[test]
fn test_adsr2_register_layout() {
    // ADSR2 = SSSRRRRR
    //   bits 7-5 = sustain_level
    //   bits 4-0 = sustain_rate
    let mut mem = Memory::new();
    // 0b101_10110 = 0xB6 → level=5, rate=22
    dsp_vw(&mut mem, 0, 0x6, 0xB6);
    assert_eq!(mem.dsp.voices[0].adsr.sustain_level, 5);
    assert_eq!(mem.dsp.voices[0].adsr.sustain_rate,  22);
}

#[test]
fn test_all_8_voices_have_independent_registers() {
    let mut mem = Memory::new();
    for v in 0u8..8 {
        dsp_vw(&mut mem, v, 0x0, v * 10);       // left vol
        dsp_vw(&mut mem, v, 0x4, v);             // srcn
    }
    for v in 0..8usize {
        assert_eq!(mem.dsp.voices[v].left_vol, (v as u8 * 10) as i8);
        assert_eq!(mem.dsp.voices[v].srcn,      v as u8);
    }
}
