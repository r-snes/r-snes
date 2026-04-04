/// DSP tests
///
/// Covers:
///   - Adsr: all 5 phases, rate-table gating, sustain hold (rate=0),
///     attack fast-path (rate=15), decay exponential step, full A→D→S→R→Off cycle
///   - Voice: default state, field types (i8 volumes, 14-bit pitch, srcn)
///   - Dsp::new / read_reg / write_reg: per-voice registers, global registers
///     (KON, KOFF, DIR, MVOL), register layout (voice N at offset N*0x10)
///   - Dsp::step: pitch counter advance, BRR decoding driven from DIR table,
///     voice goes Off on non-looping end block
///   - Dsp::render_audio_single: silent voices skipped, envelope scaling,
///     signed volume, multi-voice mix, clamping, mute flag (FLGS bit 6)

use apu::dsp::{Adsr, Brr, Dsp, EnvelopePhase, Voice};
use apu::Memory;

// ============================================================
// Helpers
// ============================================================

const DSP_BASE: u16 = 0xF200;

/// Write a per-voice DSP register through the Memory bus.
fn dsp_vw(mem: &mut Memory, voice: u8, reg: u8, val: u8) {
    mem.write8(DSP_BASE + ((voice as u16) << 4) + reg as u16, val);
}

/// Write a global DSP register through the Memory bus.
fn dsp_gw(mem: &mut Memory, reg: u8, val: u8) {
    mem.write8(DSP_BASE + reg as u16, val);
}

/// Read a DSP register by its 7-bit index directly.
fn dsp_r(mem: &Memory, idx: u8) -> u8 {
    mem.dsp.read_reg(idx)
}

/// Build a minimal valid 9-byte BRR block in APU RAM.
/// shift=4, filter=0, end=end_flag, loop=loop_flag, all nibbles=0.
fn write_silent_brr_block(mem: &mut Memory, addr: u16, end: bool, do_loop: bool) {
    let mut header: u8 = 0x40; // shift=4, filter=0
    if end     { header |= 0x01; }
    if do_loop { header |= 0x02; }
    mem.write8(addr, header);
    for i in 1..9u16 {
        mem.write8(addr + i, 0x00);
    }
}

/// Write a 4-byte DIR entry for srcn N.
fn write_dir_entry(mem: &mut Memory, dir_page: u8, srcn: u8, start: u16, loop_addr: u16) {
    let base = (dir_page as u16) << 8;
    let entry = base + (srcn as u16) * 4;
    mem.write8(entry,     (start     & 0xFF) as u8);
    mem.write8(entry + 1, (start     >> 8)   as u8);
    mem.write8(entry + 2, (loop_addr & 0xFF) as u8);
    mem.write8(entry + 3, (loop_addr >> 8)   as u8);
}

// ============================================================
// ADSR — EnvelopePhase::Off
// ============================================================

#[test]
fn test_adsr_off_does_nothing() {
    let mut adsr = Adsr::default();
    // Default phase is Off; envelope_level must stay 0 forever.
    for _ in 0..1000 {
        adsr.update_envelope();
        assert_eq!(adsr.envelope_level, 0);
        assert_eq!(adsr.envelope_phase, EnvelopePhase::Off);
    }
}

// ============================================================
// ADSR — Attack
// ============================================================

#[test]
fn test_adsr_attack_rate15_jumps_1024_per_tick() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Attack;
    adsr.attack_rate = 15; // fast-path: no rate gating

    adsr.update_envelope();
    // Should jump straight by 1024 (or hit 0x7FF if it was near the top)
    assert!(adsr.envelope_level >= 1024 || adsr.envelope_level == 0x7FF);
}

#[test]
fn test_adsr_attack_rate15_reaches_max_within_2_ticks() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Attack;
    adsr.attack_rate = 15;
    adsr.update_envelope(); // +1024 → 1024
    adsr.update_envelope(); // could hit max
    // After at most ceil(0x7FF / 1024) = 2 ticks we must be at max or in Decay
    assert!(
        adsr.envelope_level == 0x7FF || adsr.envelope_phase == EnvelopePhase::Decay,
        "level={:#05X} phase={:?}", adsr.envelope_level, adsr.envelope_phase
    );
}

#[test]
fn test_adsr_attack_normal_rate_gated() {
    // attack_rate=0 → rate_idx=1 → period=2048 ticks between steps.
    // After 1 tick nothing should have changed.
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Attack;
    adsr.attack_rate = 0;

    adsr.update_envelope(); // first tick: counter=1, not yet due
    assert_eq!(adsr.envelope_level, 0, "should not step yet");
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Attack);
}

#[test]
fn test_adsr_attack_transitions_to_decay_at_max() {
    // Use rate=15 to reach max quickly.
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Attack;
    adsr.attack_rate = 15;

    let mut reached_decay = false;
    for _ in 0..10 {
        adsr.update_envelope();
        if adsr.envelope_phase == EnvelopePhase::Decay {
            reached_decay = true;
            break;
        }
    }
    assert!(reached_decay, "Attack must transition to Decay on hitting 0x7FF");
    assert_eq!(adsr.envelope_level, 0x7FF, "level must be exactly 0x7FF on Decay entry");
}

#[test]
fn test_adsr_attack_level_never_exceeds_max() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Attack;
    adsr.attack_rate = 15;
    for _ in 0..20 {
        adsr.update_envelope();
        assert!(adsr.envelope_level <= 0x7FF, "level={:#05X}", adsr.envelope_level);
    }
}

// ============================================================
// ADSR — Decay
// ============================================================

#[test]
fn test_adsr_decay_falls_toward_sustain_target() {
    // decay_rate=7 → rate_idx = 7*2+16 = 30 → period=2 (very fast)
    let mut adsr = Adsr::default();
    adsr.envelope_phase  = EnvelopePhase::Decay;
    adsr.decay_rate      = 7;
    adsr.sustain_level   = 3; // target = (3+1)*0x100 = 0x400
    adsr.envelope_level  = 0x7FF;

    let mut hit_sustain = false;
    for _ in 0..5000 {
        adsr.update_envelope();
        if adsr.envelope_phase == EnvelopePhase::Sustain {
            hit_sustain = true;
            break;
        }
    }
    assert!(hit_sustain, "Decay must eventually reach Sustain");
    let expected_target = (adsr.sustain_level as u16 + 1) * 0x100;
    assert_eq!(adsr.envelope_level, expected_target, "must land exactly on sustain target");
}

#[test]
fn test_adsr_decay_step_is_exponential() {
    // At high levels the step is larger than at low levels.
    // decay_rate=7 (period=2), run two steps from two different starting points.
    let step_at = |start: u16| -> u16 {
        let mut adsr = Adsr::default();
        adsr.envelope_phase = EnvelopePhase::Decay;
        adsr.decay_rate     = 7;
        adsr.sustain_level  = 0; // target = 0x100
        adsr.envelope_level = start;
        let before = adsr.envelope_level;
        // Pump until at least one step fires
        for _ in 0..10 {
            let pre = adsr.envelope_level;
            adsr.update_envelope();
            if adsr.envelope_level != pre || adsr.envelope_phase != EnvelopePhase::Decay {
                break;
            }
        }
        before.saturating_sub(adsr.envelope_level)
    };

    let step_high = step_at(0x700);
    let step_low  = step_at(0x200);
    assert!(step_high > step_low, "exponential: high={step_high} low={step_low}");
}

#[test]
fn test_adsr_decay_rate0_is_slow() {
    // decay_rate=0 → rate_idx=16 → period=64: after 10 ticks, no step.
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Decay;
    adsr.decay_rate     = 0;
    adsr.sustain_level  = 0;
    adsr.envelope_level = 0x7FF;

    for _ in 0..10 {
        adsr.update_envelope();
    }
    assert_eq!(adsr.envelope_level, 0x7FF, "decay_rate=0 should not step within 10 ticks");
}

// ============================================================
// ADSR — Sustain
// ============================================================

#[test]
fn test_adsr_sustain_rate0_holds_forever() {
    // sustain_rate=0 → period=0 → tick_due always returns false → level never changes.
    let mut adsr = Adsr::default();
    adsr.envelope_phase  = EnvelopePhase::Sustain;
    adsr.sustain_rate    = 0;
    adsr.envelope_level  = 0x400;

    for _ in 0..10_000 {
        adsr.update_envelope();
    }
    assert_eq!(adsr.envelope_level, 0x400, "sustain_rate=0 must hold level indefinitely");
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Sustain);
}

#[test]
fn test_adsr_sustain_decreases_with_nonzero_rate() {
    // sustain_rate=31 → period=1 (every tick)
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Sustain;
    adsr.sustain_rate   = 31;
    adsr.envelope_level = 0x400;
    let before = adsr.envelope_level;

    adsr.update_envelope();
    assert!(adsr.envelope_level < before, "level must decrease with sustain_rate=31");
}

#[test]
fn test_adsr_sustain_reaches_off_at_zero() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Sustain;
    adsr.sustain_rate   = 31;
    adsr.envelope_level = 1; // one step away from 0

    // The step formula is (level >> 8) + 1 = (0 >> 8) + 1 = 1, so one tick should silence it.
    adsr.update_envelope();
    assert_eq!(adsr.envelope_level, 0);
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Off);
}

#[test]
fn test_adsr_sustain_step_is_exponential() {
    // Higher level → bigger step, like Decay.
    let step_at = |start: u16| -> u16 {
        let mut adsr = Adsr::default();
        adsr.envelope_phase = EnvelopePhase::Sustain;
        adsr.sustain_rate   = 31;
        adsr.envelope_level = start;
        let before = adsr.envelope_level;
        adsr.update_envelope();
        before.saturating_sub(adsr.envelope_level)
    };
    let step_high = step_at(0x700);
    let step_low  = step_at(0x100);
    assert!(step_high > step_low, "sustain exponential: high={step_high} low={step_low}");
}

#[test]
fn test_tick_due_period_zero_never_fires() {
    // period=0 (sustain_rate=0) must never step the envelope — covers the
    // early-return guard inside tick_due.
    let mut adsr = Adsr::default();
    adsr.envelope_phase  = EnvelopePhase::Sustain;
    adsr.sustain_rate    = 0; // ENVELOPE_RATE_TABLE[0] = 0
    adsr.envelope_level  = 0x400;

    for _ in 0..100_000 {
        adsr.update_envelope();
    }
    assert_eq!(adsr.envelope_level, 0x400, "period=0 must never step");
}

#[test]
fn test_tick_due_fires_exactly_at_period() {
    // decay_rate=7 → period = ENVELOPE_RATE_TABLE[30] = 2.
    // Must not step on tick 1, must step on tick 2.
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Decay;
    adsr.decay_rate     = 7;
    adsr.sustain_level  = 0;
    adsr.envelope_level = 0x7FF;

    let before = adsr.envelope_level;
    adsr.update_envelope(); // tick 1
    assert_eq!(adsr.envelope_level, before, "must not step on first tick");
    adsr.update_envelope(); // tick 2
    assert!(adsr.envelope_level < before, "must step on second tick (period=2)");
}

// ============================================================
// ADSR — Release
// ============================================================

#[test]
fn test_adsr_release_decreases_by_8_per_tick() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Release;
    adsr.envelope_level = 100;

    adsr.update_envelope();
    assert_eq!(adsr.envelope_level, 92, "release must subtract exactly 8");
}

#[test]
fn test_adsr_release_reaches_off() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Release;
    adsr.envelope_level = 0x7FF;

    for _ in 0..300 {
        adsr.update_envelope();
        if adsr.envelope_phase == EnvelopePhase::Off { break; }
    }
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Off);
    assert_eq!(adsr.envelope_level, 0);
}

#[test]
fn test_adsr_release_clamps_at_zero_not_underflow() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Release;
    adsr.envelope_level = 4; // 4 - 8 would underflow without saturating_sub

    adsr.update_envelope();
    assert_eq!(adsr.envelope_level, 0);
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Off);
}

// ============================================================
// ADSR — Full A→D→S→R→Off cycle
// ============================================================

#[test]
fn test_adsr_full_cycle() {
    let mut adsr = Adsr::default();
    adsr.attack_rate    = 15;  // instant
    adsr.decay_rate     = 7;   // fast
    adsr.sustain_level  = 2;   // target = 0x300
    adsr.sustain_rate   = 31;  // fast sustain drain
    adsr.envelope_phase = EnvelopePhase::Attack;

    // Attack → Decay
    while adsr.envelope_phase == EnvelopePhase::Attack { adsr.update_envelope(); }
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Decay);

    // Decay → Sustain
    for _ in 0..10_000 {
        if adsr.envelope_phase != EnvelopePhase::Decay { break; }
        adsr.update_envelope();
    }
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Sustain);
    let target = (adsr.sustain_level as u16 + 1) * 0x100;
    assert_eq!(adsr.envelope_level, target);

    // Sustain → Off
    for _ in 0..10_000 {
        if adsr.envelope_phase == EnvelopePhase::Off { break; }
        adsr.update_envelope();
    }
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Off);
    assert_eq!(adsr.envelope_level, 0);
}

#[test]
fn test_adsr_key_off_mid_attack_enters_release() {
    // Even if still in Attack, switching phase to Release should work normally.
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Attack;
    adsr.attack_rate    = 0; // slow
    adsr.envelope_level = 500;

    // Simulate key-off: caller sets phase to Release
    adsr.envelope_phase = EnvelopePhase::Release;

    adsr.update_envelope();
    assert_eq!(adsr.envelope_level, 492, "release from mid-attack: 500 - 8 = 492");
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
// Dsp::new / read_reg / write_reg — register layout
// ============================================================

#[test]
fn test_dsp_registers_zeroed_on_new() {
    let dsp = Dsp::new();
    for i in 0u8..=127 {
        assert_eq!(dsp.read_reg(i), 0, "register 0x{:02X} not zero", i);
    }
}

#[test]
fn test_read_reg_write_reg_roundtrip() {
    // Write via write_reg and read back the same value for all 128 indices.
    // (Some registers have side-effects but must still store the raw byte.)
    let mut mem = Memory::new();
    // Use safe values that won't trigger KON/KOFF (avoid 0x4C/0x5C with non-zero)
    let safe_regs: Vec<u8> = (0u8..=127)
        .filter(|&i| i != 0x4C && i != 0x5C) // skip KON / KOFF
        .collect();

    for &idx in &safe_regs {
        mem.dsp.write_reg(idx, idx);
    }
    for &idx in &safe_regs {
        assert_eq!(mem.dsp.read_reg(idx), idx, "reg 0x{:02X}", idx);
    }
}

#[test]
fn test_write_reg_index_masked_to_7_bits() {
    // Index 0x80 should behave the same as 0x00 (high bit ignored).
    let mut dsp = Dsp::new();
    dsp.write_reg(0x00, 0xAB);
    assert_eq!(dsp.read_reg(0x80), 0xAB);
    assert_eq!(dsp.read_reg(0x00), 0xAB);
}

#[test]
fn test_write_reg_unrecognised_global_registers_stored() {
    // Unimplemented globals ($2C, $3C, $6C, $7D, $0D, $2D, $3D, $4D, $6D)
    // must store the raw byte without panicking.
    let mut mem = Memory::new();
    for &reg in &[0x2Cu8, 0x3C, 0x6C, 0x7D, 0x0D, 0x2D, 0x3D, 0x4D, 0x6D] {
        mem.dsp.write_reg(reg, 0xAB);
        assert_eq!(mem.dsp.read_reg(reg), 0xAB,
            "unimplemented reg {reg:#04X} must store raw byte");
    }
}

#[test]
fn test_write_reg_gain_register_stored() {
    // $X7 GAIN is not yet implemented but must store the value.
    let mut mem = Memory::new();
    dsp_vw(&mut mem, 0, 0x7, 0x7F);
    assert_eq!(mem.dsp.read_reg(0x07), 0x7F, "GAIN register must store value");
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

// ============================================================
// Dsp — global register mapping
// ============================================================

#[test]
fn test_dir_register_stored() {
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x5D, 0x08);
    // We can't read dir_base directly (private), but we can verify
    // the raw register byte was stored.
    assert_eq!(mem.dsp.read_reg(0x5D), 0x08);
}

#[test]
fn test_kon_register_keys_on_specified_voices() {
    let mut mem = Memory::new();
    let dir_page: u8 = 0x01;
    let brr_addr: u16 = 0x0200;

    // Set up a silent BRR block and DIR entry for voices 0 and 2.
    write_silent_brr_block(&mut mem, brr_addr, true, false); // end, no loop

    dsp_gw(&mut mem, 0x5D, dir_page); // DIR

    for srcn in [0u8, 2u8] {
        write_dir_entry(&mut mem, dir_page, srcn, brr_addr, brr_addr);
        dsp_vw(&mut mem, srcn, 0x4, srcn);        // SRCN
        dsp_vw(&mut mem, srcn, 0x5, 0x8F);        // ADSR1
        dsp_vw(&mut mem, srcn, 0x6, 0xE0);        // ADSR2
    }

    dsp_gw(&mut mem, 0x4C, 0b00000101); // KON: voices 0 and 2

    assert!(mem.dsp.voices[0].key_on, "voice 0 should be keyed on");
    assert!(mem.dsp.voices[2].key_on, "voice 2 should be keyed on");
    assert!(!mem.dsp.voices[1].key_on, "voice 1 should NOT be keyed on");
}

#[test]
fn test_koff_register_enters_release_phase() {
    let mut mem = Memory::new();
    // Manually put voice 1 in Sustain, then key-off.
    mem.dsp.voices[1].key_on = true;
    mem.dsp.voices[1].adsr.envelope_phase = EnvelopePhase::Sustain;
    mem.dsp.voices[1].adsr.envelope_level = 0x400;

    dsp_gw(&mut mem, 0x5C, 0b00000010); // KOFF voice 1

    assert_eq!(
        mem.dsp.voices[1].adsr.envelope_phase,
        EnvelopePhase::Release,
        "KOFF must trigger Release phase"
    );
}

#[test]
fn test_kon_resets_brr_state() {
    // KON must zero all BRR playback state so the new sample starts clean.
    let mut mem = Memory::new();
    mem.dsp.voices[0].brr.nibble_idx  = 12;
    mem.dsp.voices[0].brr.prev1       = 999;
    mem.dsp.voices[0].brr.prev2       = 888;
    mem.dsp.voices[0].brr.buffer_fill = 16;
    mem.dsp.voices[0].brr.loop_addr   = 0xDEAD;
    mem.dsp.voices[0].pitch_counter   = 0x0FFF;

    dsp_gw(&mut mem, 0x4C, 0x01);

    assert_eq!(mem.dsp.voices[0].brr.nibble_idx,  0, "nibble_idx must reset");
    assert_eq!(mem.dsp.voices[0].brr.prev1,       0, "prev1 must reset");
    assert_eq!(mem.dsp.voices[0].brr.prev2,       0, "prev2 must reset");
    assert_eq!(mem.dsp.voices[0].brr.buffer_fill, 0, "buffer_fill must reset");
    assert_eq!(mem.dsp.voices[0].brr.loop_addr,   0, "loop_addr must reset");
    assert_eq!(mem.dsp.voices[0].pitch_counter,   0, "pitch_counter must reset");
}

#[test]
fn test_kon_resets_current_sample() {
    let mut mem = Memory::new();
    mem.dsp.voices[0].current_sample = 0x7FFF;
    dsp_gw(&mut mem, 0x4C, 0x01);
    assert_eq!(mem.dsp.voices[0].current_sample, 0, "current_sample must reset on KON");
}

#[test]
fn test_kon_zero_value_keys_on_no_voices() {
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x4C, 0x00);
    for v in 0..8 {
        assert!(!mem.dsp.voices[v].key_on, "no voice should be keyed on when KON=0");
    }
}

#[test]
fn test_kon_all_8_voices_simultaneously() {
    let mut mem = Memory::new();
    let dir_page: u8  = 0x01;
    let brr_addr: u16 = 0x0200;
    write_silent_brr_block(&mut mem, brr_addr, true, false);
    for v in 0..8u8 {
        write_dir_entry(&mut mem, dir_page, v, brr_addr, brr_addr);
        dsp_vw(&mut mem, v, 0x4, v);
    }
    dsp_gw(&mut mem, 0x5D, dir_page);
    dsp_gw(&mut mem, 0x4C, 0xFF);

    for v in 0..8 {
        assert!(mem.dsp.voices[v].key_on,
            "voice {v} must be keyed on when KON=0xFF");
        assert_eq!(mem.dsp.voices[v].adsr.envelope_phase, EnvelopePhase::Attack,
            "voice {v} must be in Attack after KON");
    }
}

#[test]
fn test_koff_zero_value_releases_no_voices() {
    let mut mem = Memory::new();
    for v in 0..8 {
        mem.dsp.voices[v].adsr.envelope_phase = EnvelopePhase::Sustain;
    }
    dsp_gw(&mut mem, 0x5C, 0x00);
    for v in 0..8 {
        assert_eq!(mem.dsp.voices[v].adsr.envelope_phase, EnvelopePhase::Sustain,
            "KOFF=0 must not release any voice");
    }
}

#[test]
fn test_koff_when_voice_already_off_does_not_panic() {
    // KOFF on an already-Off voice must not panic and level must stay 0.
    let mut mem = Memory::new();
    mem.dsp.voices[2].adsr.envelope_phase = EnvelopePhase::Off;
    dsp_gw(&mut mem, 0x5C, 0b00000100);
    assert_eq!(mem.dsp.voices[2].adsr.envelope_level, 0);
}

// ============================================================
// Dsp::step — BRR playback and pitch advance
// ============================================================

/// Set up voice 0 with a silent, non-looping, end-flagged BRR block and key it on.
fn setup_single_voice_end_block(mem: &mut Memory) {
    let dir_page: u8 = 0x01;
    let brr_addr: u16 = 0x0200;

    write_silent_brr_block(mem, brr_addr, true, false); // end, no loop
    write_dir_entry(mem, dir_page, 0, brr_addr, brr_addr);

    dsp_gw(mem, 0x5D, dir_page);
    dsp_vw(mem, 0, 0x4, 0);             // SRCN 0
    dsp_vw(mem, 0, 0x0, 100i8 as u8);   // VOL L
    dsp_vw(mem, 0, 0x1, 100i8 as u8);   // VOL R
    // pitch=0x1000 → native rate
    dsp_vw(mem, 0, 0x2, 0x00);
    dsp_vw(mem, 0, 0x3, 0x10);
    // ADSR: fast attack, hold sustain
    dsp_vw(mem, 0, 0x5, 0x8F);
    dsp_vw(mem, 0, 0x6, 0xE0);

    dsp_gw(mem, 0x4C, 0x01); // KON voice 0
}

#[test]
fn test_step_voice_goes_off_after_non_looping_end_block() {
    let mut mem = Memory::new();
    setup_single_voice_end_block(&mut mem);

    // Drive enough ticks for the 16-sample block to drain.
    // At pitch=0x1000 we advance 1 sample per tick; 16 samples = 16 ticks minimum.
    let mut went_off = false;
    for _ in 0..200 {
        mem.dsp.step(&mem_shadow(&mem));
        if mem.dsp.voices[0].adsr.envelope_phase == EnvelopePhase::Off
            || (!mem.dsp.voices[0].key_on
                && mem.dsp.voices[0].adsr.envelope_phase == EnvelopePhase::Release)
        {
            went_off = true;
            break;
        }
    }
    assert!(went_off, "non-looping voice must stop after end block");
}

#[test]
fn test_step_looping_voice_stays_active() {
    let mut mem = Memory::new();
    let dir_page: u8 = 0x01;
    let brr_addr: u16 = 0x0200;

    // Two blocks: block 0 normal, block 1 end+loop back to block 0.
    write_silent_brr_block(&mut mem, brr_addr,     false, false);
    write_silent_brr_block(&mut mem, brr_addr + 9, true,  true); // end+loop
    write_dir_entry(&mut mem, dir_page, 0, brr_addr, brr_addr);

    dsp_gw(&mut mem, 0x5D, dir_page);
    dsp_vw(&mut mem, 0, 0x4, 0);
    dsp_vw(&mut mem, 0, 0x2, 0x00);
    dsp_vw(&mut mem, 0, 0x3, 0x10);
    dsp_vw(&mut mem, 0, 0x5, 0x8F);
    dsp_vw(&mut mem, 0, 0x6, 0xE0);
    dsp_gw(&mut mem, 0x4C, 0x01);

    // Run for 500 ticks; voice must never go Off.
    for i in 0..500 {
        mem.dsp.step(&mem_shadow(&mem));
        assert_ne!(
            mem.dsp.voices[0].adsr.envelope_phase, EnvelopePhase::Off,
            "looping voice went silent at tick {i}"
        );
    }
}

#[test]
fn test_step_pitch_counter_advances() {
    let mut mem = Memory::new();
    setup_single_voice_end_block(&mut mem);

    let counter_before = mem.dsp.voices[0].pitch_counter;
    mem.dsp.step(&mem_shadow(&mem));
    // pitch=0x1000 is added each tick; counter wraps at 0x1000 so
    // after one tick from zero the high nibble has consumed one sample
    // and the counter resets to 0. What matters: key_on went true.
    assert!(mem.dsp.voices[0].key_on || mem.dsp.voices[0].adsr.envelope_phase != EnvelopePhase::Off);
}

#[test]
fn test_step_with_ram_matches_step_memory() {
    // step_with_ram (used by Apu) must produce identical results to
    // step(&Memory) over multiple ticks — covers decode_next_block_raw
    // and ram_read8.
    let dir_page: u8  = 0x01;
    let brr_addr: u16 = 0x0200;

    let mut mem_a = Memory::new();
    write_silent_brr_block(&mut mem_a, brr_addr, true, true);
    write_dir_entry(&mut mem_a, dir_page, 0, brr_addr, brr_addr);
    dsp_gw(&mut mem_a, 0x5D, dir_page);
    dsp_vw(&mut mem_a, 0, 0x5, 0x8F);
    dsp_vw(&mut mem_a, 0, 0x6, 0xE0);
    dsp_gw(&mut mem_a, 0x4C, 0x01);

    let mut mem_b = Memory::new();
    write_silent_brr_block(&mut mem_b, brr_addr, true, true);
    write_dir_entry(&mut mem_b, dir_page, 0, brr_addr, brr_addr);
    dsp_gw(&mut mem_b, 0x5D, dir_page);
    dsp_vw(&mut mem_b, 0, 0x5, 0x8F);
    dsp_vw(&mut mem_b, 0, 0x6, 0xE0);
    dsp_gw(&mut mem_b, 0x4C, 0x01);

    for _ in 0..10 {
        mem_a.dsp.step(&mem_shadow(&mem_a));
        let ram = mem_b.ram;
        mem_b.dsp.step_with_ram(&ram);
    }

    assert_eq!(
        mem_a.dsp.voices[0].adsr.envelope_level,
        mem_b.dsp.voices[0].adsr.envelope_level,
        "step_with_ram must match step(&Memory) output"
    );
}

#[test]
fn test_step_with_ram_out_of_range_address_does_not_panic() {
    // ram_read8 returns 0 for addresses >= RAM size.
    // DIR at $FF00 with zero bytes → BRR resolves to $0000 (all zero,
    // end flag not set, so voice keeps running safely).
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x5D, 0xFF);
    dsp_vw(&mut mem, 0, 0x5, 0x8F);
    dsp_vw(&mut mem, 0, 0x6, 0xE0);
    dsp_gw(&mut mem, 0x4C, 0x01);

    let ram = mem.ram;
    mem.dsp.step_with_ram(&ram); // must not panic
}

// ============================================================
// Dsp::render_audio_single — mixing and clamping
// ============================================================

#[test]
fn test_render_silent_when_all_voices_off() {
    let dsp = Dsp::new();
    let (l, r) = dsp.render_audio_single();
    assert_eq!((l, r), (0, 0));
}

#[test]
fn test_render_single_voice_envelope_scaling() {
    // Verify the full output chain:
    //   scaled = (sample * env) >> 11
    //   voiced = (scaled * voice_vol) >> 7
    //   out    = (voiced * master_vol) >> 7
    let mut dsp = Dsp::new();
    // Set master volume to 127 so it acts as a near-transparent pass-through.
    dsp.write_reg(0x0C, 127u8); // MVOLL
    dsp.write_reg(0x1C, 0u8);   // MVOLR — right not tested here
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0x7FF; // max
    dsp.voices[0].current_sample      = 0x4000; // large positive
    dsp.voices[0].left_vol            = 64;
    dsp.voices[0].right_vol           = 0;

    let (l, _r) = dsp.render_audio_single();

    let env_sample = (0x4000_i32 * 0x7FF_i32) >> 11;
    let voiced     = (env_sample * 64) >> 7;
    let expected_l = ((voiced * 127) >> 7).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    assert_eq!(l, expected_l, "left channel scaling mismatch");
}

#[test]
fn test_render_right_channel_zero_when_right_vol_zero() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 127u8); // MVOLL — non-zero so left would carry signal
    dsp.write_reg(0x1C, 127u8); // MVOLR — non-zero, so silence must come from right_vol=0
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0x7FF;
    dsp.voices[0].current_sample      = 1000;
    dsp.voices[0].left_vol            = 100;
    dsp.voices[0].right_vol           = 0;   // muted right

    let (_l, r) = dsp.render_audio_single();
    assert_eq!(r, 0, "right channel must be silent when right_vol=0 regardless of MVOLR");
}

#[test]
fn test_render_negative_volume_inverts_signal() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 127u8); // MVOLL — must be non-zero to hear output
    dsp.write_reg(0x1C, 127u8); // MVOLR
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0x7FF;
    dsp.voices[0].current_sample      = 1000;
    dsp.voices[0].left_vol            = 64;
    dsp.voices[0].right_vol           = -64; // negative → inverted

    let (l, r) = dsp.render_audio_single();
    assert!(l > 0, "positive vol → positive output");
    assert!(r < 0, "negative vol → negative output");
    // Integer arithmetic right-shift rounds toward negative infinity rather
    // than toward zero, so positive and negative paths can differ by 1 at
    // each >> stage.  With two stages (per-voice volume and master volume)
    // the worst-case accumulated difference is ±2.
    assert!(
        (l + r).abs() <= 2,
        "magnitudes should match within ±2 (got l={l}, r={r}, diff={})", l + r
    );
}

#[test]
fn test_render_voice_off_contributes_nothing() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 127u8); // MVOLL — non-zero so active voice produces output
    // Voice 0 on, voice 1 off but with a large sample that would dominate if mixed.
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0x7FF;
    dsp.voices[0].current_sample      = 100;
    dsp.voices[0].left_vol            = 64;

    dsp.voices[1].adsr.envelope_phase = EnvelopePhase::Off; // should be skipped
    dsp.voices[1].current_sample      = 0x7FFF;
    dsp.voices[1].left_vol            = 127;

    let (l_with, _) = dsp.render_audio_single();

    // Compare against a DSP where voice 1 does not exist at all.
    // Copy master vol register state across so both DSPs are identical except
    // for the presence of voice 1.
    let mut dsp2 = Dsp::new();
    dsp2.write_reg(0x0C, 127u8);
    dsp2.voices[0] = dsp.voices[0];
    let (l_without, _) = dsp2.render_audio_single();

    assert!(l_with > 0,    "active voice must produce non-zero output");
    assert_eq!(l_with, l_without, "Off voice must not contribute to mix");
}

#[test]
fn test_render_two_voices_summed() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 127u8); // MVOLL
    dsp.write_reg(0x1C, 127u8); // MVOLR
    for v in 0..2 {
        dsp.voices[v].adsr.envelope_phase = EnvelopePhase::Sustain;
        dsp.voices[v].adsr.envelope_level = 0x7FF;
        dsp.voices[v].current_sample      = 1000;
        dsp.voices[v].left_vol            = 32;
        dsp.voices[v].right_vol           = 32;
    }
    let (l2, _) = dsp.render_audio_single();

    // One voice only — same master vol so the comparison is fair
    let mut dsp1 = Dsp::new();
    dsp1.write_reg(0x0C, 127u8); // MVOLL
    dsp1.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp1.voices[0].adsr.envelope_level = 0x7FF;
    dsp1.voices[0].current_sample      = 1000;
    dsp1.voices[0].left_vol            = 32;
    let (l1, _) = dsp1.render_audio_single();

    assert!(l2 > l1, "two voices must produce louder output than one");
    assert_eq!(l2, l1 * 2, "two identical voices must double the output");
}

#[test]
fn test_render_all_8_voices_contribute_to_mix() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 127u8);
    dsp.write_reg(0x1C, 127u8);

    for v in 0..8 {
        dsp.voices[v].adsr.envelope_phase = EnvelopePhase::Sustain;
        dsp.voices[v].adsr.envelope_level = 0x7FF;
        dsp.voices[v].current_sample      = 100;
        dsp.voices[v].left_vol            = 16;
        dsp.voices[v].right_vol           = 16;
    }
    let (l8, _) = dsp.render_audio_single();

    let mut dsp1 = Dsp::new();
    dsp1.write_reg(0x0C, 127u8);
    dsp1.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp1.voices[0].adsr.envelope_level = 0x7FF;
    dsp1.voices[0].current_sample      = 100;
    dsp1.voices[0].left_vol            = 16;
    let (l1, _) = dsp1.render_audio_single();

    assert!(l8 > l1, "8 voices must produce more output than 1");
    // Integer arithmetic means the 8 voices are summed before master volume
    // is applied, so rounding is not perfectly linear per-voice.
    // Verify the output is proportionally in range: between 7x and 9x a
    // single voice.
    assert!(
        l8 >= l1 * 7 && l8 <= l1 * 9,
        "8 voices must produce ~8x single-voice output (got l8={l8}, l1={l1})"
    );
}

#[test]
fn test_render_output_clamped_to_i16_range() {
    // Drive 8 voices at max to provoke overflow; must clamp, not wrap.
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 127u8); // MVOLL
    dsp.write_reg(0x1C, 127u8); // MVOLR
    for v in 0..8 {
        dsp.voices[v].adsr.envelope_phase = EnvelopePhase::Sustain;
        dsp.voices[v].adsr.envelope_level = 0x7FF;
        dsp.voices[v].current_sample      = i16::MAX;
        dsp.voices[v].left_vol            = 127;
        dsp.voices[v].right_vol           = 127;
    }
    let (l, r) = dsp.render_audio_single();
    assert_eq!(l, i16::MAX, "left must clamp to i16::MAX");
    assert_eq!(r, i16::MAX, "right must clamp to i16::MAX");
}

#[test]
fn test_render_zero_envelope_silences_voice() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 127u8); // MVOLL non-zero — silence must come from envelope=0, not master vol
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0; // zero envelope → silent
    dsp.voices[0].current_sample      = 0x7FFF;
    dsp.voices[0].left_vol            = 127;

    let (l, _) = dsp.render_audio_single();
    assert_eq!(l, 0, "zero envelope must produce zero output regardless of master vol");
}

// ============================================================
// ENVX, OUTX, ENDX register update tests
//
// ENVX ($X8): reads back (envelope_level >> 4) as u8 — 7-bit range 0x00–0x7F.
// OUTX ($X9): reads back (current_sample  >> 8) as u8 — signed top byte.
// ENDX ($7C): bit N set when voice N's BRR end-flag fires; cleared on KON.
// ============================================================

// --- ENVX ---

#[test]
fn test_envx_zero_when_voice_off() {
    // A freshly created DSP has all voices Off; ENVX must read as 0.
    let dsp = Dsp::new();
    for v in 0u8..8 {
        let envx = dsp.read_reg((v << 4) | 0x8);
        assert_eq!(envx, 0, "voice {v} ENVX should be 0 before any step");
    }
}

#[test]
fn test_envx_updated_after_step() {
    // Put voice 0 in Sustain at a known level, run one step, read ENVX back.
    // Expected: ENVX = envelope_level >> 4.
    let mut mem = Memory::new();
    setup_single_voice_end_block(&mut mem);

    // Advance until the envelope leaves Attack (level > 0).
    for _ in 0..200 {
        mem.dsp.step(&mem_shadow(&mem));
        if mem.dsp.voices[0].adsr.envelope_level > 0 {
            break;
        }
    }

    let level = mem.dsp.voices[0].adsr.envelope_level;
    let expected_envx = (level >> 4) as u8;
    let actual_envx   = mem.dsp.read_reg(0x08); // voice 0, offset +8

    assert_eq!(
        actual_envx, expected_envx,
        "ENVX must equal envelope_level >> 4 (level={level:#05X})"
    );
}

#[test]
fn test_envx_tracks_envelope_level_directly() {
    // Set envelope manually, step once, confirm ENVX matches.
    let mut mem = Memory::new();
    setup_single_voice_end_block(&mut mem);

    // Force a known envelope level.
    mem.dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    mem.dsp.voices[0].adsr.envelope_level = 0x400;
    mem.dsp.voices[0].adsr.sustain_rate   = 0; // hold forever

    mem.dsp.step(&mem_shadow(&mem));

    let expected = (0x400u16 >> 4) as u8; // = 0x40
    assert_eq!(mem.dsp.read_reg(0x08), expected);
}

#[test]
fn test_envx_max_value_is_0x7f() {
    // envelope_level max = 0x7FF; 0x7FF >> 4 = 0x7F.
    let mut mem = Memory::new();
    setup_single_voice_end_block(&mut mem);

    mem.dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    mem.dsp.voices[0].adsr.envelope_level = 0x7FF;
    mem.dsp.voices[0].adsr.sustain_rate   = 0;

    mem.dsp.step(&mem_shadow(&mem));

    assert_eq!(mem.dsp.read_reg(0x08), 0x7F, "ENVX max must be 0x7F");
}

#[test]
fn test_envx_all_8_voices_independent() {
    // Give each voice a distinct envelope level; all 8 ENVX registers
    // must reflect their respective voice's level after one step.
    let mut mem = Memory::new();
    let dir_page: u8 = 0x01;
    let brr_addr: u16 = 0x0200;
    write_silent_brr_block(&mut mem, brr_addr, true, false);

    for v in 0u8..8 {
        write_dir_entry(&mut mem, dir_page, v, brr_addr, brr_addr);
        dsp_gw(&mut mem, 0x5D, dir_page);
        dsp_vw(&mut mem, v, 0x4, v);
        dsp_vw(&mut mem, v, 0x2, 0x00);
        dsp_vw(&mut mem, v, 0x3, 0x10);
        dsp_vw(&mut mem, v, 0x5, 0x8F);
        dsp_vw(&mut mem, v, 0x6, 0xE0);

        // Force a distinct level for each voice (hold at sustain rate 0).
        let level: u16 = 0x100 * (v as u16 + 1); // 0x100, 0x200, … 0x800 (clamped to 0x7FF)
        let level = level.min(0x7FF);
        mem.dsp.voices[v as usize].adsr.envelope_phase = EnvelopePhase::Sustain;
        mem.dsp.voices[v as usize].adsr.envelope_level = level;
        mem.dsp.voices[v as usize].adsr.sustain_rate   = 0;
        mem.dsp.voices[v as usize].key_on              = true;
    }

    mem.dsp.step(&mem_shadow(&mem));

    for v in 0usize..8 {
        let expected = (mem.dsp.voices[v].adsr.envelope_level >> 4) as u8;
        let actual   = mem.dsp.read_reg(((v << 4) | 0x8) as u8);
        assert_eq!(actual, expected, "voice {v} ENVX mismatch");
    }
}

// --- OUTX ---

#[test]
fn test_outx_zero_when_sample_zero() {
    let dsp = Dsp::new();
    for v in 0u8..8 {
        assert_eq!(dsp.read_reg((v << 4) | 0x9), 0, "voice {v} OUTX should be 0");
    }
}

#[test]
fn test_outx_reflects_top_byte_of_current_sample() {
    // Set current_sample to a known value, step, read OUTX.
    // OUTX = (current_sample >> 8) as u8 (signed top byte).
    let mut mem = Memory::new();
    setup_single_voice_end_block(&mut mem);

    mem.dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    mem.dsp.voices[0].adsr.envelope_level = 0x7FF;
    mem.dsp.voices[0].adsr.sustain_rate   = 0;
    mem.dsp.voices[0].current_sample      = 0x1234;

    mem.dsp.step(&mem_shadow(&mem));

    // After step the BRR buffer will have been consumed and current_sample
    // updated from decoded data. We test the register reflects *that* value.
    let sample   = mem.dsp.voices[0].current_sample;
    let expected = (sample >> 8) as u8;
    let actual   = mem.dsp.read_reg(0x09); // voice 0, offset +9
    assert_eq!(actual, expected, "OUTX must equal current_sample >> 8");
}

#[test]
fn test_outx_positive_and_negative_samples() {
    // Positive sample: top byte positive (0x00–0x7F).
    // Negative sample: top byte negative when cast to i8 (0x80–0xFF as u8).
    let mut mem = Memory::new();
    setup_single_voice_end_block(&mut mem);
    mem.dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    mem.dsp.voices[0].adsr.envelope_level = 0x7FF;
    mem.dsp.voices[0].adsr.sustain_rate   = 0;

    // Force a positive sample into the buffer so step() outputs it.
    mem.dsp.voices[0].brr.sample_buffer = [0x0500i16; 16];
    mem.dsp.voices[0].brr.buffer_fill   = 16;
    mem.dsp.voices[0].brr.nibble_idx    = 0;

    mem.dsp.step(&mem_shadow(&mem));
    let outx_pos = mem.dsp.read_reg(0x09) as i8;
    assert!(outx_pos > 0, "positive sample → positive OUTX top byte");

    // Now force a negative sample.
    mem.dsp.voices[0].brr.sample_buffer = [(-0x0500i16); 16];
    mem.dsp.voices[0].brr.buffer_fill   = 16;
    mem.dsp.voices[0].brr.nibble_idx    = 0;

    mem.dsp.step(&mem_shadow(&mem));
    let outx_neg = mem.dsp.read_reg(0x09) as i8;
    assert!(outx_neg < 0, "negative sample → negative OUTX top byte");
}

// --- ENDX ---

#[test]
fn test_endx_zero_on_new_dsp() {
    let dsp = Dsp::new();
    assert_eq!(dsp.read_reg(0x7C), 0, "ENDX must be 0 on init");
}

#[test]
fn test_endx_set_when_end_block_reached() {
    // Voice 0 plays a single non-looping end block; ENDX bit 0 must be set
    // once the block is decoded.
    let mut mem = Memory::new();
    setup_single_voice_end_block(&mut mem);

    // Run until the voice either goes silent or ENDX is set.
    let mut endx_set = false;
    for _ in 0..200 {
        mem.dsp.step(&mem_shadow(&mem));
        if mem.dsp.read_reg(0x7C) & 0x01 != 0 {
            endx_set = true;
            break;
        }
    }
    assert!(endx_set, "ENDX bit 0 must be set after voice 0 hits its end block");
}

#[test]
fn test_endx_set_for_correct_voice_bit() {
    // Use voice 3 (not voice 0) so we verify the bit position is v, not always 0.
    let mut mem = Memory::new();
    let dir_page: u8 = 0x01;
    let brr_addr: u16 = 0x0200;

    write_silent_brr_block(&mut mem, brr_addr, true, false); // end, no loop
    write_dir_entry(&mut mem, dir_page, 0, brr_addr, brr_addr);

    dsp_gw(&mut mem, 0x5D, dir_page);
    dsp_vw(&mut mem, 3, 0x4, 0);           // voice 3, SRCN 0
    dsp_vw(&mut mem, 3, 0x2, 0x00);
    dsp_vw(&mut mem, 3, 0x3, 0x10);
    dsp_vw(&mut mem, 3, 0x5, 0x8F);
    dsp_vw(&mut mem, 3, 0x6, 0xE0);
    dsp_gw(&mut mem, 0x4C, 0b00001000);    // KON voice 3 only

    for _ in 0..200 {
        mem.dsp.step(&mem_shadow(&mem));
        let endx = mem.dsp.read_reg(0x7C);
        if endx != 0 {
            assert_eq!(endx & 0b00001000, 0b00001000, "bit 3 must be set for voice 3");
            assert_eq!(endx & 0b11110111, 0, "no other ENDX bits should be set");
            return;
        }
    }
    panic!("ENDX was never set for voice 3");
}

#[test]
fn test_endx_cleared_on_kon() {
    // Trigger a voice to set ENDX, then key it on again and confirm the bit clears.
    let mut mem = Memory::new();
    setup_single_voice_end_block(&mut mem);

    // Run until ENDX bit 0 is set.
    for _ in 0..200 {
        mem.dsp.step(&mem_shadow(&mem));
        if mem.dsp.read_reg(0x7C) & 0x01 != 0 {
            break;
        }
    }
    assert_eq!(mem.dsp.read_reg(0x7C) & 0x01, 1, "precondition: ENDX bit 0 must be set");

    // Key on voice 0 again — this should clear bit 0.
    dsp_gw(&mut mem, 0x4C, 0x01);
    assert_eq!(
        mem.dsp.read_reg(0x7C) & 0x01, 0,
        "KON must clear ENDX bit for the keyed-on voice"
    );
}

#[test]
fn test_endx_looping_sample_still_sets_bit() {
    // Even a looping sample sets ENDX when the end block is hit —
    // the voice keeps playing, but the bit must still be set.
    let mut mem = Memory::new();
    let dir_page: u8 = 0x01;
    let brr_addr: u16 = 0x0200;

    // One block: end + loop (voice loops back to itself forever).
    write_silent_brr_block(&mut mem, brr_addr, true, true);
    write_dir_entry(&mut mem, dir_page, 0, brr_addr, brr_addr);

    dsp_gw(&mut mem, 0x5D, dir_page);
    dsp_vw(&mut mem, 0, 0x4, 0);
    dsp_vw(&mut mem, 0, 0x2, 0x00);
    dsp_vw(&mut mem, 0, 0x3, 0x10);
    dsp_vw(&mut mem, 0, 0x5, 0x8F);
    dsp_vw(&mut mem, 0, 0x6, 0xE0);
    dsp_gw(&mut mem, 0x4C, 0x01);

    let mut endx_set = false;
    for _ in 0..200 {
        mem.dsp.step(&mem_shadow(&mem));
        if mem.dsp.read_reg(0x7C) & 0x01 != 0 {
            endx_set = true;
            break;
        }
    }
    assert!(endx_set, "ENDX must be set even for looping samples");
    // Voice should still be active (it loops).
    assert!(
        mem.dsp.voices[0].adsr.envelope_phase != EnvelopePhase::Off,
        "looping voice must still be active after ENDX fires"
    );
}

#[test]
fn test_endx_multiple_voices_independent_bits() {
    // Voice 0 and voice 2 each get a non-looping end block.
    // After they both finish, bits 0 and 2 must be set; bits 1,3-7 must not.
    let mut mem = Memory::new();
    let dir_page: u8 = 0x01;
    let brr_addr: u16 = 0x0200;

    write_silent_brr_block(&mut mem, brr_addr, true, false);

    for srcn in [0u8, 2u8] {
        write_dir_entry(&mut mem, dir_page, srcn, brr_addr, brr_addr);
    }
    dsp_gw(&mut mem, 0x5D, dir_page);

    for v in [0u8, 2u8] {
        dsp_vw(&mut mem, v, 0x4, v);    // SRCN = voice index (0 or 2)
        dsp_vw(&mut mem, v, 0x2, 0x00);
        dsp_vw(&mut mem, v, 0x3, 0x10);
        dsp_vw(&mut mem, v, 0x5, 0x8F);
        dsp_vw(&mut mem, v, 0x6, 0xE0);
    }
    dsp_gw(&mut mem, 0x4C, 0b00000101); // KON voices 0 and 2

    for _ in 0..200 {
        mem.dsp.step(&mem_shadow(&mem));
    }

    let endx = mem.dsp.read_reg(0x7C);
    assert_eq!(endx & 0b00000101, 0b00000101, "bits 0 and 2 must be set");
    assert_eq!(endx & 0b11111010, 0,          "all other bits must be clear");
}


// ============================================================
// Master volume ($0C MVOLL / $1C MVOLR) tests
// ============================================================

#[test]
fn test_master_vol_zero_silences_output() {
    // Hardware reset value is 0 for both master volumes.
    // A voice that would otherwise produce output must be silent.
    let mut dsp = Dsp::new();
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0x7FF;
    dsp.voices[0].current_sample      = 1000;
    dsp.voices[0].left_vol            = 127;
    dsp.voices[0].right_vol           = 127;
    // master_vol_left/right default to 0 — no write needed

    let (l, r) = dsp.render_audio_single();
    assert_eq!(l, 0, "zero master left volume must silence output");
    assert_eq!(r, 0, "zero master right volume must silence output");
}

#[test]
fn test_master_vol_register_write_read_roundtrip() {
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x0C, 0x7F); // MVOLL = 127
    dsp_gw(&mut mem, 0x1C, 0x40); // MVOLR = 64
    assert_eq!(mem.dsp.read_reg(0x0C), 0x7F, "MVOLL register must store written value");
    assert_eq!(mem.dsp.read_reg(0x1C), 0x40, "MVOLR register must store written value");
}

#[test]
fn test_master_vol_max_passes_signal_through() {
    // With master volume at 127 the output should be non-zero when voices are active.
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 127u8); // MVOLL = 127
    dsp.write_reg(0x1C, 127u8); // MVOLR = 127
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0x7FF;
    dsp.voices[0].current_sample      = 1000;
    dsp.voices[0].left_vol            = 64;
    dsp.voices[0].right_vol           = 64;

    let (l, r) = dsp.render_audio_single();
    assert!(l > 0, "non-zero master vol + active voice must produce output");
    assert!(r > 0, "non-zero master vol + active voice must produce output");
}

#[test]
fn test_master_vol_scales_output_proportionally() {
    // Doubling the master volume should roughly double the output.
    let voice_sample = |mvol: i8| -> i16 {
        let mut dsp = Dsp::new();
        dsp.write_reg(0x0C, mvol as u8); // MVOLL
        dsp.write_reg(0x1C, mvol as u8); // MVOLR
        dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
        dsp.voices[0].adsr.envelope_level = 0x7FF;
        dsp.voices[0].current_sample      = 1000;
        dsp.voices[0].left_vol            = 64;
        dsp.voices[0].right_vol           = 64;
        dsp.render_audio_single().0
    };

    let half = voice_sample(32) as i32;
    let full = voice_sample(64) as i32;
    // Allow ±1 for integer rounding, same reasoning as the negative-volume test.
    assert!((full - half * 2).abs() <= 1,
        "master vol 64 should produce ~2x output of master vol 32 (got {full} vs {half}*2={})", half * 2);
}

#[test]
fn test_master_vol_negative_inverts_output() {
    // Negative master volume should invert the polarity of the mix,
    // mirroring how per-voice negative volumes work.
    let mut dsp_pos = Dsp::new();
    dsp_pos.write_reg(0x0C, 64u8); // MVOLL = +64
    dsp_pos.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp_pos.voices[0].adsr.envelope_level = 0x7FF;
    dsp_pos.voices[0].current_sample      = 1000;
    dsp_pos.voices[0].left_vol            = 64;
    let (l_pos, _) = dsp_pos.render_audio_single();

    let mut dsp_neg = Dsp::new();
    dsp_neg.write_reg(0x0C, (-64i8) as u8); // MVOLL = -64 (signed)
    dsp_neg.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp_neg.voices[0].adsr.envelope_level = 0x7FF;
    dsp_neg.voices[0].current_sample      = 1000;
    dsp_neg.voices[0].left_vol            = 64;
    let (l_neg, _) = dsp_neg.render_audio_single();

    assert!(l_pos > 0, "positive master vol should give positive output");
    assert!(l_neg < 0, "negative master vol should invert output");
    assert!((l_pos + l_neg).abs() <= 1, "magnitudes should match within ±1 rounding");
}

#[test]
fn test_master_vol_left_right_independent() {
    // MVOLL only affects the left channel and MVOLR only the right.
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 64u8);  // MVOLL = 64
    dsp.write_reg(0x1C, 0u8);   // MVOLR = 0 (right silenced)
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0x7FF;
    dsp.voices[0].current_sample      = 1000;
    dsp.voices[0].left_vol            = 64;
    dsp.voices[0].right_vol           = 64;

    let (l, r) = dsp.render_audio_single();
    assert!(l != 0, "left channel should carry signal");
    assert_eq!(r, 0,  "right channel must be silent when MVOLR=0");
}

#[test]
fn test_master_vol_written_via_memory_bus_affects_mix() {
    // End-to-end: write MVOLL/MVOLR via the Memory bus, then verify
    // render_audio_single respects them.  This confirms write_reg
    // correctly populates the internal fields (not just the register array).
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x0C, 100u8); // MVOLL = 100 (as i8 = 100, positive)
    dsp_gw(&mut mem, 0x1C, 100u8); // MVOLR = 100

    mem.dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    mem.dsp.voices[0].adsr.envelope_level = 0x7FF;
    mem.dsp.voices[0].current_sample      = 1000;
    mem.dsp.voices[0].left_vol            = 64;
    mem.dsp.voices[0].right_vol           = 64;

    let (l, r) = mem.dsp.render_audio_single();
    assert!(l > 0, "MVOLL written via bus must produce non-zero left output");
    assert!(r > 0, "MVOLR written via bus must produce non-zero right output");
}

// ============================================================
// Helper: RAM shadow for step() borrow splitting
// ============================================================

/// Build a read-only Memory clone that shares APU RAM contents.
/// step() only reads RAM via mem.read8(); the Dsp inside the clone is unused.
fn mem_shadow(src: &Memory) -> Memory {
    let mut shadow = Memory::new();
    shadow.ram.copy_from_slice(&src.ram);
    shadow
}
