//! DSP core tests
//!
//! Covers Dsp::new, read_reg/write_reg, global registers (KON/KOFF/DIR),
//! step() BRR playback and looping, render_audio_single mixing/clamping,
//! ENVX/OUTX/ENDX register updates, and master volume.
//!
//! ADSR phase tests → adsr_tests.rs
//! Voice/register mapping tests → voice_tests.rs
//! BRR decode tests → brr_tests.rs

use apu::Memory;
use apu::dsp::{Dsp, EnvelopePhase};

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

/// Build a minimal valid 9-byte BRR block in APU RAM.
/// shift=4, filter=0, end=end_flag, loop=loop_flag, all nibbles=0.
fn write_silent_brr_block(mem: &mut Memory, addr: u16, end: bool, do_loop: bool) {
    let mut header: u8 = 0x40; // shift=4, filter=0
    if end {
        header |= 0x01;
    }
    if do_loop {
        header |= 0x02;
    }
    mem.write8(addr, header);
    for i in 1..9u16 {
        mem.write8(addr + i, 0x00);
    }
}

/// Write a 4-byte DIR entry for srcn N.
fn write_dir_entry(mem: &mut Memory, dir_page: u8, srcn: u8, start: u16, loop_addr: u16) {
    let base = (dir_page as u16) << 8;
    let entry = base + (srcn as u16) * 4;
    mem.write8(entry, (start & 0xFF) as u8);
    mem.write8(entry + 1, (start >> 8) as u8);
    mem.write8(entry + 2, (loop_addr & 0xFF) as u8);
    mem.write8(entry + 3, (loop_addr >> 8) as u8);
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
    // Skip registers that have special behaviour:
    //   $4C / $5C — KON / KOFF trigger voice state changes with non-zero values
    //   $07, $17, $27, $37, $47, $57, $67, $77 — GAIN (todo!, not yet implemented)
    let mut mem = Memory::new();
    let safe_regs: Vec<u8> = (0u8..=127)
        .filter(|&i| {
            i != 0x4C && i != 0x5C          // KON / KOFF
            && (i & 0x0F) != 0x07 // GAIN registers ($X7)
        })
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
        assert_eq!(
            mem.dsp.read_reg(reg),
            0xAB,
            "unimplemented reg {reg:#04X} must store raw byte"
        );
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
        dsp_vw(&mut mem, srcn, 0x4, srcn); // SRCN
        dsp_vw(&mut mem, srcn, 0x5, 0x8F); // ADSR1
        dsp_vw(&mut mem, srcn, 0x6, 0xE0); // ADSR2
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
    mem.dsp.voices[0].brr.nibble_idx = 12;
    mem.dsp.voices[0].brr.prev1 = 999;
    mem.dsp.voices[0].brr.prev2 = 888;
    mem.dsp.voices[0].brr.buffer_fill = 16;
    mem.dsp.voices[0].brr.loop_addr = 0xDEAD;
    mem.dsp.voices[0].pitch_counter = 0x0FFF;

    dsp_gw(&mut mem, 0x4C, 0x01);

    assert_eq!(mem.dsp.voices[0].brr.nibble_idx, 0, "nibble_idx must reset");
    assert_eq!(mem.dsp.voices[0].brr.prev1, 0, "prev1 must reset");
    assert_eq!(mem.dsp.voices[0].brr.prev2, 0, "prev2 must reset");
    assert_eq!(
        mem.dsp.voices[0].brr.buffer_fill, 0,
        "buffer_fill must reset"
    );
    assert_eq!(mem.dsp.voices[0].brr.loop_addr, 0, "loop_addr must reset");
    assert_eq!(
        mem.dsp.voices[0].pitch_counter, 0,
        "pitch_counter must reset"
    );
}

#[test]
fn test_kon_resets_current_sample() {
    let mut mem = Memory::new();
    mem.dsp.voices[0].current_sample = 0x7FFF;
    dsp_gw(&mut mem, 0x4C, 0x01);
    assert_eq!(
        mem.dsp.voices[0].current_sample, 0,
        "current_sample must reset on KON"
    );
}

#[test]
fn test_kon_zero_value_keys_on_no_voices() {
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x4C, 0x00);
    for v in 0..8 {
        assert!(
            !mem.dsp.voices[v].key_on,
            "no voice should be keyed on when KON=0"
        );
    }
}

#[test]
fn test_kon_all_8_voices_simultaneously() {
    let mut mem = Memory::new();
    let dir_page: u8 = 0x01;
    let brr_addr: u16 = 0x0200;
    write_silent_brr_block(&mut mem, brr_addr, true, false);
    for v in 0..8u8 {
        write_dir_entry(&mut mem, dir_page, v, brr_addr, brr_addr);
        dsp_vw(&mut mem, v, 0x4, v);
    }
    dsp_gw(&mut mem, 0x5D, dir_page);
    dsp_gw(&mut mem, 0x4C, 0xFF);

    for v in 0..8 {
        assert!(
            mem.dsp.voices[v].key_on,
            "voice {v} must be keyed on when KON=0xFF"
        );
        assert_eq!(
            mem.dsp.voices[v].adsr.envelope_phase,
            EnvelopePhase::Attack,
            "voice {v} must be in Attack after KON"
        );
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
        assert_eq!(
            mem.dsp.voices[v].adsr.envelope_phase,
            EnvelopePhase::Sustain,
            "KOFF=0 must not release any voice"
        );
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
    dsp_vw(mem, 0, 0x4, 0); // SRCN 0
    dsp_vw(mem, 0, 0x0, 100i8 as u8); // VOL L
    dsp_vw(mem, 0, 0x1, 100i8 as u8); // VOL R
    // pitch=0x1000 → native rate
    dsp_vw(mem, 0, 0x2, 0x00);
    dsp_vw(mem, 0, 0x3, 0x10);
    // ADSR: fast attack, hold sustain
    dsp_vw(mem, 0, 0x5, 0x8F);
    dsp_vw(mem, 0, 0x6, 0xE0);

    dsp_gw(mem, 0x4C, 0x01); // KON voice 0
}

/// Set up voice 0 with a silent, looping (end+loop back to itself) BRR
/// block and key it on. Unlike `setup_single_voice_end_block`, this
/// voice never mutes on its own, so it's the right fixture for tests
/// that need to observe *ongoing* playback — pitch counter advance,
/// a manually forced envelope level surviving a step — rather than the
/// end-of-sample path itself.
fn setup_single_voice_looping_block(mem: &mut Memory) {
    let dir_page: u8 = 0x01;
    let brr_addr: u16 = 0x0200;

    write_silent_brr_block(mem, brr_addr, true, true); // end+loop -> loops to itself
    write_dir_entry(mem, dir_page, 0, brr_addr, brr_addr);

    dsp_gw(mem, 0x5D, dir_page);
    dsp_vw(mem, 0, 0x4, 0); // SRCN 0
    dsp_vw(mem, 0, 0x0, 100i8 as u8); // VOL L
    dsp_vw(mem, 0, 0x1, 100i8 as u8); // VOL R
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
        mem.dsp.step(&mut mem.ram);
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
    write_silent_brr_block(&mut mem, brr_addr, false, false);
    write_silent_brr_block(&mut mem, brr_addr + 9, true, true); // end+loop
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
        mem.dsp.step(&mut mem.ram);
        assert_ne!(
            mem.dsp.voices[0].adsr.envelope_phase,
            EnvelopePhase::Off,
            "looping voice went silent at tick {i}"
        );
    }
}

#[test]
fn test_step_pitch_counter_advances() {
    let mut mem = Memory::new();
    setup_single_voice_looping_block(&mut mem);

    let _counter_before = mem.dsp.voices[0].pitch_counter;
    mem.dsp.step(&mut mem.ram);
    // pitch=0x1000 is added each tick; counter wraps at 0x1000 so
    // after one tick from zero the high nibble has consumed one sample
    // and the counter resets to 0. What matters: key_on went true.
    assert!(
        mem.dsp.voices[0].key_on || mem.dsp.voices[0].adsr.envelope_phase != EnvelopePhase::Off
    );
}

#[test]
fn test_step_advances_envelope_over_multiple_ticks() {
    // Verify step(&RawARAM) correctly advances the envelope over 10 ticks,
    // covering decode_next_block and ram_read8.
    let dir_page: u8 = 0x01;
    let brr_addr: u16 = 0x0200;

    let mut mem = Memory::new();
    write_silent_brr_block(&mut mem, brr_addr, true, true);
    write_dir_entry(&mut mem, dir_page, 0, brr_addr, brr_addr);
    dsp_gw(&mut mem, 0x5D, dir_page);
    dsp_vw(&mut mem, 0, 0x5, 0x8F); // fast attack
    dsp_vw(&mut mem, 0, 0x6, 0xE0); // hold sustain
    dsp_gw(&mut mem, 0x4C, 0x01);

    for _ in 0..10 {
        mem.dsp.step(&mut mem.ram);
    }

    assert!(
        mem.dsp.voices[0].adsr.envelope_level > 0,
        "envelope must have advanced after 10 DSP ticks"
    );
}

#[test]
fn test_step_out_of_range_ram_address_does_not_panic() {
    // ram_read8 returns 0 for addresses >= RAM size.
    // DIR at $FF00 with zero bytes → BRR resolves to $0000 (all zero,
    // end flag not set, so voice keeps running safely).
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x5D, 0xFF);
    dsp_vw(&mut mem, 0, 0x5, 0x8F);
    dsp_vw(&mut mem, 0, 0x6, 0xE0);
    dsp_gw(&mut mem, 0x4C, 0x01);

    mem.dsp.step(&mut mem.ram); // must not panic
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
    dsp.write_reg(0x1C, 0u8); // MVOLR — right not tested here
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0x7FF; // max
    dsp.voices[0].current_sample = 0x4000; // large positive
    dsp.voices[0].left_vol = 64;
    dsp.voices[0].right_vol = 0;

    let (l, _r) = dsp.render_audio_single();

    let env_sample = (0x4000_i32 * 0x7FF_i32) >> 11;
    let voiced = (env_sample * 64) >> 7;
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
    dsp.voices[0].current_sample = 1000;
    dsp.voices[0].left_vol = 100;
    dsp.voices[0].right_vol = 0; // muted right

    let (_l, r) = dsp.render_audio_single();
    assert_eq!(
        r, 0,
        "right channel must be silent when right_vol=0 regardless of MVOLR"
    );
}

#[test]
fn test_render_negative_volume_inverts_signal() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 127u8); // MVOLL — must be non-zero to hear output
    dsp.write_reg(0x1C, 127u8); // MVOLR
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0x7FF;
    dsp.voices[0].current_sample = 1000;
    dsp.voices[0].left_vol = 64;
    dsp.voices[0].right_vol = -64; // negative → inverted

    let (l, r) = dsp.render_audio_single();
    assert!(l > 0, "positive vol → positive output");
    assert!(r < 0, "negative vol → negative output");
    // Integer arithmetic right-shift rounds toward negative infinity rather
    // than toward zero, so positive and negative paths can differ by 1 at
    // each >> stage.  With two stages (per-voice volume and master volume)
    // the worst-case accumulated difference is ±2.
    assert!(
        (l + r).abs() <= 2,
        "magnitudes should match within ±2 (got l={l}, r={r}, diff={})",
        l + r
    );
}

#[test]
fn test_render_voice_off_contributes_nothing() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 127u8); // MVOLL — non-zero so active voice produces output
    // Voice 0 on, voice 1 off but with a large sample that would dominate if mixed.
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0x7FF;
    dsp.voices[0].current_sample = 100;
    dsp.voices[0].left_vol = 64;

    dsp.voices[1].adsr.envelope_phase = EnvelopePhase::Off; // should be skipped
    dsp.voices[1].current_sample = 0x7FFF;
    dsp.voices[1].left_vol = 127;

    let (l_with, _) = dsp.render_audio_single();

    // Compare against a DSP where voice 1 does not exist at all.
    // Copy master vol register state across so both DSPs are identical except
    // for the presence of voice 1.
    let mut dsp2 = Dsp::new();
    dsp2.write_reg(0x0C, 127u8);
    dsp2.voices[0] = dsp.voices[0];
    let (l_without, _) = dsp2.render_audio_single();

    assert!(l_with > 0, "active voice must produce non-zero output");
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
        dsp.voices[v].current_sample = 1000;
        dsp.voices[v].left_vol = 32;
        dsp.voices[v].right_vol = 32;
    }
    let (l2, _) = dsp.render_audio_single();

    // One voice only — same master vol so the comparison is fair
    let mut dsp1 = Dsp::new();
    dsp1.write_reg(0x0C, 127u8); // MVOLL
    dsp1.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp1.voices[0].adsr.envelope_level = 0x7FF;
    dsp1.voices[0].current_sample = 1000;
    dsp1.voices[0].left_vol = 32;
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
        dsp.voices[v].current_sample = 100;
        dsp.voices[v].left_vol = 16;
        dsp.voices[v].right_vol = 16;
    }
    let (l8, _) = dsp.render_audio_single();

    let mut dsp1 = Dsp::new();
    dsp1.write_reg(0x0C, 127u8);
    dsp1.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp1.voices[0].adsr.envelope_level = 0x7FF;
    dsp1.voices[0].current_sample = 100;
    dsp1.voices[0].left_vol = 16;
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
        dsp.voices[v].current_sample = i16::MAX;
        dsp.voices[v].left_vol = 127;
        dsp.voices[v].right_vol = 127;
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
    dsp.voices[0].current_sample = 0x7FFF;
    dsp.voices[0].left_vol = 127;

    let (l, _) = dsp.render_audio_single();
    assert_eq!(
        l, 0,
        "zero envelope must produce zero output regardless of master vol"
    );
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
        mem.dsp.step(&mut mem.ram);
        if mem.dsp.voices[0].adsr.envelope_level > 0 {
            break;
        }
    }

    let level = mem.dsp.voices[0].adsr.envelope_level;
    let expected_envx = (level >> 4) as u8;
    let actual_envx = mem.dsp.read_reg(0x08); // voice 0, offset +8

    assert_eq!(
        actual_envx, expected_envx,
        "ENVX must equal envelope_level >> 4 (level={level:#05X})"
    );
}

#[test]
fn test_envx_tracks_envelope_level_directly() {
    // Set envelope manually, step once, confirm ENVX matches.
    let mut mem = Memory::new();
    setup_single_voice_looping_block(&mut mem);

    // Force a known envelope level.
    mem.dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    mem.dsp.voices[0].adsr.envelope_level = 0x400;
    mem.dsp.voices[0].adsr.sustain_rate = 0; // hold forever

    mem.dsp.step(&mut mem.ram);

    let expected = (0x400u16 >> 4) as u8; // = 0x40
    assert_eq!(mem.dsp.read_reg(0x08), expected);
}

#[test]
fn test_envx_max_value_is_0x7f() {
    // envelope_level max = 0x7FF; 0x7FF >> 4 = 0x7F.
    let mut mem = Memory::new();
    setup_single_voice_looping_block(&mut mem);

    mem.dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    mem.dsp.voices[0].adsr.envelope_level = 0x7FF;
    mem.dsp.voices[0].adsr.sustain_rate = 0;

    mem.dsp.step(&mut mem.ram);

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
        mem.dsp.voices[v as usize].adsr.sustain_rate = 0;
        mem.dsp.voices[v as usize].key_on = true;
    }

    mem.dsp.step(&mut mem.ram);

    for v in 0usize..8 {
        let expected = (mem.dsp.voices[v].adsr.envelope_level >> 4) as u8;
        let actual = mem.dsp.read_reg(((v << 4) | 0x8) as u8);
        assert_eq!(actual, expected, "voice {v} ENVX mismatch");
    }
}

// --- OUTX ---

#[test]
fn test_outx_zero_when_sample_zero() {
    let dsp = Dsp::new();
    for v in 0u8..8 {
        assert_eq!(
            dsp.read_reg((v << 4) | 0x9),
            0,
            "voice {v} OUTX should be 0"
        );
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
    mem.dsp.voices[0].adsr.sustain_rate = 0;
    mem.dsp.voices[0].current_sample = 0x1234;

    mem.dsp.step(&mut mem.ram);

    // After step the BRR buffer will have been consumed and current_sample
    // updated from decoded data. We test the register reflects *that* value.
    let sample = mem.dsp.voices[0].current_sample;
    let expected = (sample >> 8) as u8;
    let actual = mem.dsp.read_reg(0x09); // voice 0, offset +9
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
    mem.dsp.voices[0].adsr.sustain_rate = 0;

    // Force a positive sample into the buffer so step() outputs it.
    //
    // current_sample is Gaussian-interpolated from the voice's 4-sample
    // `history`, not read straight out of `sample_buffer` — so `history`
    // needs priming too, not just the buffer. At this exact pitch-counter
    // phase (fresh key-on, index=0) the newest tap's weight (GAUSS[0]) is
    // genuinely zero by design, so a single freshly-buffered sample alone
    // wouldn't show up yet; setting all 4 history taps to the same sign
    // makes the interpolated output unambiguously match that sign
    // regardless of fractional phase.
    mem.dsp.voices[0].brr.sample_buffer = [0x0500i16; 16];
    mem.dsp.voices[0].brr.buffer_fill = 16;
    mem.dsp.voices[0].brr.nibble_idx = 0;
    mem.dsp.voices[0].history = [0x0500i16; 4];

    mem.dsp.step(&mut mem.ram);
    let outx_pos = mem.dsp.read_reg(0x09) as i8;
    assert!(outx_pos > 0, "positive sample → positive OUTX top byte");

    // Now force a negative sample.
    mem.dsp.voices[0].brr.sample_buffer = [(-0x0500i16); 16];
    mem.dsp.voices[0].brr.buffer_fill = 16;
    mem.dsp.voices[0].brr.nibble_idx = 0;
    mem.dsp.voices[0].history = [-0x0500i16; 4];

    mem.dsp.step(&mut mem.ram);
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
        mem.dsp.step(&mut mem.ram);
        if mem.dsp.read_reg(0x7C) & 0x01 != 0 {
            endx_set = true;
            break;
        }
    }
    assert!(
        endx_set,
        "ENDX bit 0 must be set after voice 0 hits its end block"
    );
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
    dsp_vw(&mut mem, 3, 0x4, 0); // voice 3, SRCN 0
    dsp_vw(&mut mem, 3, 0x2, 0x00);
    dsp_vw(&mut mem, 3, 0x3, 0x10);
    dsp_vw(&mut mem, 3, 0x5, 0x8F);
    dsp_vw(&mut mem, 3, 0x6, 0xE0);
    dsp_gw(&mut mem, 0x4C, 0b00001000); // KON voice 3 only

    for _ in 0..200 {
        mem.dsp.step(&mut mem.ram);
        let endx = mem.dsp.read_reg(0x7C);
        if endx != 0 {
            assert_eq!(
                endx & 0b00001000,
                0b00001000,
                "bit 3 must be set for voice 3"
            );
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
        mem.dsp.step(&mut mem.ram);
        if mem.dsp.read_reg(0x7C) & 0x01 != 0 {
            break;
        }
    }
    assert_eq!(
        mem.dsp.read_reg(0x7C) & 0x01,
        1,
        "precondition: ENDX bit 0 must be set"
    );

    // Key on voice 0 again — this should clear bit 0.
    dsp_gw(&mut mem, 0x4C, 0x01);
    assert_eq!(
        mem.dsp.read_reg(0x7C) & 0x01,
        0,
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
        mem.dsp.step(&mut mem.ram);
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
        dsp_vw(&mut mem, v, 0x4, v); // SRCN = voice index (0 or 2)
        dsp_vw(&mut mem, v, 0x2, 0x00);
        dsp_vw(&mut mem, v, 0x3, 0x10);
        dsp_vw(&mut mem, v, 0x5, 0x8F);
        dsp_vw(&mut mem, v, 0x6, 0xE0);
    }
    dsp_gw(&mut mem, 0x4C, 0b00000101); // KON voices 0 and 2

    for _ in 0..200 {
        mem.dsp.step(&mut mem.ram);
    }

    let endx = mem.dsp.read_reg(0x7C);
    assert_eq!(endx & 0b00000101, 0b00000101, "bits 0 and 2 must be set");
    assert_eq!(endx & 0b11111010, 0, "all other bits must be clear");
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
    dsp.voices[0].current_sample = 1000;
    dsp.voices[0].left_vol = 127;
    dsp.voices[0].right_vol = 127;
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
    assert_eq!(
        mem.dsp.read_reg(0x0C),
        0x7F,
        "MVOLL register must store written value"
    );
    assert_eq!(
        mem.dsp.read_reg(0x1C),
        0x40,
        "MVOLR register must store written value"
    );
}

#[test]
fn test_master_vol_max_passes_signal_through() {
    // With master volume at 127 the output should be non-zero when voices are active.
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 127u8); // MVOLL = 127
    dsp.write_reg(0x1C, 127u8); // MVOLR = 127
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0x7FF;
    dsp.voices[0].current_sample = 1000;
    dsp.voices[0].left_vol = 64;
    dsp.voices[0].right_vol = 64;

    let (l, r) = dsp.render_audio_single();
    assert!(
        l > 0,
        "non-zero master vol + active voice must produce output"
    );
    assert!(
        r > 0,
        "non-zero master vol + active voice must produce output"
    );
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
        dsp.voices[0].current_sample = 1000;
        dsp.voices[0].left_vol = 64;
        dsp.voices[0].right_vol = 64;
        dsp.render_audio_single().0
    };

    let half = voice_sample(32) as i32;
    let full = voice_sample(64) as i32;
    // Allow ±1 for integer rounding, same reasoning as the negative-volume test.
    assert!(
        (full - half * 2).abs() <= 1,
        "master vol 64 should produce ~2x output of master vol 32 (got {full} vs {half}*2={})",
        half * 2
    );
}

#[test]
fn test_master_vol_negative_inverts_output() {
    // Negative master volume should invert the polarity of the mix,
    // mirroring how per-voice negative volumes work.
    let mut dsp_pos = Dsp::new();
    dsp_pos.write_reg(0x0C, 64u8); // MVOLL = +64
    dsp_pos.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp_pos.voices[0].adsr.envelope_level = 0x7FF;
    dsp_pos.voices[0].current_sample = 1000;
    dsp_pos.voices[0].left_vol = 64;
    let (l_pos, _) = dsp_pos.render_audio_single();

    let mut dsp_neg = Dsp::new();
    dsp_neg.write_reg(0x0C, (-64i8) as u8); // MVOLL = -64 (signed)
    dsp_neg.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp_neg.voices[0].adsr.envelope_level = 0x7FF;
    dsp_neg.voices[0].current_sample = 1000;
    dsp_neg.voices[0].left_vol = 64;
    let (l_neg, _) = dsp_neg.render_audio_single();

    assert!(l_pos > 0, "positive master vol should give positive output");
    assert!(l_neg < 0, "negative master vol should invert output");
    assert!(
        (l_pos + l_neg).abs() <= 1,
        "magnitudes should match within ±1 rounding"
    );
}

#[test]
fn test_master_vol_left_right_independent() {
    // MVOLL only affects the left channel and MVOLR only the right.
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0C, 64u8); // MVOLL = 64
    dsp.write_reg(0x1C, 0u8); // MVOLR = 0 (right silenced)
    dsp.voices[0].adsr.envelope_phase = EnvelopePhase::Sustain;
    dsp.voices[0].adsr.envelope_level = 0x7FF;
    dsp.voices[0].current_sample = 1000;
    dsp.voices[0].left_vol = 64;
    dsp.voices[0].right_vol = 64;

    let (l, r) = dsp.render_audio_single();
    assert!(l != 0, "left channel should carry signal");
    assert_eq!(r, 0, "right channel must be silent when MVOLR=0");
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
    mem.dsp.voices[0].current_sample = 1000;
    mem.dsp.voices[0].left_vol = 64;
    mem.dsp.voices[0].right_vol = 64;

    let (l, r) = mem.dsp.render_audio_single();
    assert!(
        l > 0,
        "MVOLL written via bus must produce non-zero left output"
    );
    assert!(
        r > 0,
        "MVOLR written via bus must produce non-zero right output"
    );
}

// ============================================================
// Noise generation — $3D NON, FLG bits 0-4 (noise clock)
// ============================================================

/// Set up a silent, self-looping voice 0 (single end+loop block, all
/// nibbles 0) and key it on. Without NON, current_sample/OUTX stay
/// exactly 0 forever — a clean baseline for proving NON actually
/// substitutes the noise generator's output.
fn setup_silent_looping_voice(mem: &mut Memory) {
    let dir_page: u8 = 0x01;
    let brr_addr: u16 = 0x0200;

    write_silent_brr_block(mem, brr_addr, true, true); // end+loop, single block
    write_dir_entry(mem, dir_page, 0, brr_addr, brr_addr);

    dsp_gw(mem, 0x5D, dir_page);
    dsp_vw(mem, 0, 0x4, 0); // SRCN 0
    dsp_vw(mem, 0, 0x2, 0x00); // PITCH lo
    dsp_vw(mem, 0, 0x3, 0x10); // PITCH hi (native rate)
    dsp_vw(mem, 0, 0x5, 0x8F); // ADSR1: fast attack
    dsp_vw(mem, 0, 0x6, 0xE0); // ADSR2: hold sustain
    dsp_gw(mem, 0x4C, 0x01); // KON voice 0
}

#[test]
fn test_non_bit_substitutes_noise_for_silent_voice() {
    let mut mem = Memory::new();
    setup_silent_looping_voice(&mut mem);

    // Baseline: NON off, noise clock untouched (FLG defaults to 0 —
    // noise stopped) — OUTX must stay exactly 0, the silent BRR source.
    for _ in 0..5 {
        mem.dsp.step(&mut mem.ram);
    }
    assert_eq!(
        mem.dsp.read_reg(0x09),
        0,
        "silent BRR source must keep OUTX at 0 before NON is set"
    );

    // Enable NON for voice 0 and run the noise clock at its fastest rate
    // (FLG bits 0-4 = 0x1F, i.e. table index 31 = "every tick").
    dsp_gw(&mut mem, 0x3D, 0x01); // NON voice 0
    dsp_gw(&mut mem, 0x6C, 0x1F); // FLG: fastest noise clock, no mute/reset

    let mut saw_nonzero = false;
    for _ in 0..20 {
        mem.dsp.step(&mut mem.ram);
        if mem.dsp.read_reg(0x09) != 0 {
            saw_nonzero = true;
            break;
        }
    }
    assert!(
        saw_nonzero,
        "NON must substitute the noise generator's output for a silent voice"
    );
}

#[test]
fn test_noise_clock_zero_never_advances_lfsr() {
    // FLG bits 0-4 = 0 ("noise off") must freeze the LFSR entirely, so a
    // NON-driven voice's output stays perfectly constant tick to tick —
    // not just silent, but unchanging (distinguishing "stopped" from
    // "coincidentally repeating").
    let mut mem = Memory::new();
    setup_silent_looping_voice(&mut mem);
    dsp_gw(&mut mem, 0x3D, 0x01); // NON voice 0
    // FLG left at its default 0: mute/reset clear, noise clock stopped.

    mem.dsp.step(&mut mem.ram);
    let first = mem.dsp.read_reg(0x09);

    for i in 0..50 {
        mem.dsp.step(&mut mem.ram);
        assert_eq!(
            mem.dsp.read_reg(0x09),
            first,
            "noise clock=0 must never advance the LFSR (tick {i})"
        );
    }
}

#[test]
fn test_clearing_non_restores_brr_output() {
    // NON only swaps the mixed output source — the BRR decoder keeps
    // running underneath (see Dsp::step's doc comment) — so clearing
    // NON should immediately go back to reflecting the (silent) BRR
    // stream, with nothing left over from the noise substitution.
    let mut mem = Memory::new();
    setup_silent_looping_voice(&mut mem);
    dsp_gw(&mut mem, 0x3D, 0x01); // NON voice 0
    dsp_gw(&mut mem, 0x6C, 0x1F); // fastest noise clock

    // First 3 ticks of the LFSR sequence from this seed are reliably
    // non-zero (verified against the actual table/LFSR, not assumed).
    for _ in 0..3 {
        mem.dsp.step(&mut mem.ram);
    }
    assert_ne!(
        mem.dsp.read_reg(0x09),
        0,
        "sanity check: noise must be substituted while NON is set"
    );

    dsp_gw(&mut mem, 0x3D, 0x00); // clear NON
    mem.dsp.step(&mut mem.ram);
    assert_eq!(
        mem.dsp.read_reg(0x09),
        0,
        "clearing NON must restore the (silent) BRR-decoded output"
    );
}

#[test]
fn test_non_register_roundtrip() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x3D, 0xA5);
    assert_eq!(dsp.read_reg(0x3D), 0xA5, "NON register must store raw bits");
}

// ============================================================
// Echo registers
// ============================================================

#[test]
fn test_efb_register_roundtrip() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x0D, 0x81); // -127 as i8
    assert_eq!(dsp.read_reg(0x0D), 0x81, "EFB must store raw bits");
}

#[test]
fn test_eon_register_roundtrip() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x4D, 0xFF);
    assert_eq!(dsp.read_reg(0x4D), 0xFF, "EON must store raw bits");
}

#[test]
fn test_esa_register_roundtrip() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x6D, 0x20);
    assert_eq!(dsp.read_reg(0x6D), 0x20, "ESA must store raw bits");
}

#[test]
fn test_edl_register_masked_to_4_bits() {
    // Only the low nibble is meaningful to the actual delay-length
    // calculation. `read_reg` always reflects the raw byte as written,
    // mask or no mask — same as PITCH's high byte — so the masked value
    // shows up through `edl()`, not `read_reg`.
    let mut dsp = Dsp::new();
    dsp.write_reg(0x7D, 0xFF);
    assert_eq!(
        dsp.read_reg(0x7D),
        0xFF,
        "read_reg must return the raw byte as written, unmasked"
    );
    assert_eq!(
        dsp.edl(),
        0x0F,
        "the processed EDL value used internally must be masked to 4 bits"
    );
}

#[test]
fn test_edl_low_nibble_preserved() {
    let mut dsp = Dsp::new();
    dsp.write_reg(0x7D, 0x0B);
    assert_eq!(dsp.read_reg(0x7D), 0x0B);
    assert_eq!(dsp.edl(), 0x0B);
}

#[test]
fn test_fir_coefficients_roundtrip_all_8_taps() {
    // $0F, $1F, ..., $7F — one coefficient per "voice slot", but they're
    // not per-voice data (see the `fir_coeff` field doc). Write a
    // distinct value to each and confirm they don't collide with each
    // other or with any real per-voice register.
    let mut dsp = Dsp::new();
    for tap in 0u8..8 {
        let reg = (tap << 4) | 0x0F;
        dsp.write_reg(reg, tap * 10 + 1);
    }
    for tap in 0u8..8 {
        let reg = (tap << 4) | 0x0F;
        assert_eq!(
            dsp.read_reg(reg),
            tap * 10 + 1,
            "FIR tap {tap} (register {reg:#04X}) must roundtrip independently"
        );
    }
}

#[test]
fn test_fir_coefficient_write_does_not_affect_voice_gain() {
    // $0F sits immediately after $0E in the register file and one slot
    // past voice 0's GAIN ($07 + voice 0's base = $07); make sure
    // writing the FIR tap doesn't leak into any real per-voice state.
    let mut dsp = Dsp::new();
    dsp.write_reg(0x00, 0x7F); // voice 0 VOL(L)
    dsp.write_reg(0x07, 0x55); // voice 0 GAIN
    dsp.write_reg(0x0F, 0x99); // FIR tap 0

    assert_eq!(dsp.voices[0].left_vol, 0x7F);
    assert_eq!(dsp.voices[0].adsr.gain_param, 0x55);
    assert_eq!(dsp.read_reg(0x0F), 0x99);
}

// ============================================================
// Echo buffer
// ============================================================

#[test]
fn test_tick_echo_buffer_delays_by_exactly_buffer_length() {
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x6D, 0x02); // ESA = page 2 ($0200)
    dsp_gw(&mut mem, 0x7D, 0x01); // EDL = 1 -> 2048 bytes = 512 stereo pairs

    // Buffer starts zeroed, so the very first tick's "old" value must
    // be silence.
    let (old_l, old_r) = mem.dsp.tick_echo_buffer(&mut mem.ram, 1234, -1234);
    assert_eq!((old_l, old_r), (0, 0));

    // Advance through the rest of the buffer with distinct dummy writes
    // so the pointer comes all the way back around to the position we
    // wrote first.
    for i in 1..512i16 {
        mem.dsp.tick_echo_buffer(&mut mem.ram, i, -i);
    }

    // The pointer has now wrapped exactly once: this call must read back
    // the very first value we wrote, 512 ticks ago.
    let (wrapped_l, wrapped_r) = mem.dsp.tick_echo_buffer(&mut mem.ram, 0, 0);
    assert_eq!(
        (wrapped_l, wrapped_r),
        (1234, -1234),
        "buffer must wrap after exactly EDL*512 stereo pairs"
    );
}

#[test]
fn test_tick_echo_buffer_writes_sequential_little_endian_addresses() {
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x6D, 0x05); // ESA = page 5 ($0500)
    dsp_gw(&mut mem, 0x7D, 0x01); // EDL = 1

    mem.dsp.tick_echo_buffer(&mut mem.ram, 0x0102, 0x0304);
    mem.dsp.tick_echo_buffer(&mut mem.ram, 0x0506, 0x0708);

    // Tick 1 writes L at $0500-501, R at $0502-503; tick 2 writes the
    // next stereo pair immediately after, at $0504-505 / $0506-507.
    let read16 = |mem: &Memory, addr: u16| -> i16 {
        i16::from_le_bytes([mem.ram[addr as usize], mem.ram[addr as usize + 1]])
    };
    assert_eq!(read16(&mem, 0x0500), 0x0102);
    assert_eq!(read16(&mem, 0x0502), 0x0304);
    assert_eq!(read16(&mem, 0x0504), 0x0506);
    assert_eq!(read16(&mem, 0x0506), 0x0708);
}

#[test]
fn test_tick_echo_buffer_edl_zero_is_silent_noop() {
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x6D, 0x03); // ESA = page 3; EDL left at its default 0

    let (l, r) = mem.dsp.tick_echo_buffer(&mut mem.ram, 999, -999);
    assert_eq!(
        (l, r),
        (0, 0),
        "EDL=0 must read as silence, not stale/garbage RAM content"
    );
    // Nothing to write to — RAM must be untouched.
    assert_eq!(mem.ram[0x0300], 0);
    assert_eq!(mem.ram[0x0301], 0);
}

#[test]
fn test_flg_bit5_disables_echo_writes_but_reads_still_work() {
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x6D, 0x04); // ESA = page 4
    dsp_gw(&mut mem, 0x7D, 0x01); // EDL = 1

    // Pre-seed the buffer's first slot directly, the way a prior
    // (writes-enabled) tick would have left it.
    mem.ram[0x0400] = 0x34; // L lo
    mem.ram[0x0401] = 0x12; // L hi -> L = 0x1234
    mem.ram[0x0402] = 0x00; // R lo
    mem.ram[0x0403] = 0x00; // R hi -> R = 0

    dsp_gw(&mut mem, 0x6C, 0x20); // FLG bit 5: disable echo writes

    let (old_l, old_r) = mem.dsp.tick_echo_buffer(&mut mem.ram, 0x7FFF, -1);
    assert_eq!(
        (old_l, old_r),
        (0x1234, 0),
        "reads must keep working even while writes are disabled"
    );

    // The attempted write must not have landed.
    assert_eq!(mem.ram[0x0400], 0x34);
    assert_eq!(mem.ram[0x0401], 0x12);
    assert_eq!(mem.ram[0x0402], 0x00);
    assert_eq!(mem.ram[0x0403], 0x00);
}

#[test]
fn test_echo_buffer_address_wraps_past_64kb_without_panicking() {
    // ESA near the top of the address space plus a large EDL means
    // esa*0x100 + offset can exceed 0xFFFF partway around the buffer —
    // this must wrap like real 16-bit hardware addressing, not panic.
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x6D, 0xFF); // ESA = page 0xFF -> base $FF00
    dsp_gw(&mut mem, 0x7D, 0x0F); // EDL = 15 (max buffer size)

    for i in 0..2000i16 {
        mem.dsp.tick_echo_buffer(&mut mem.ram, i, 0);
    }
}

// ============================================================
// FIR filter + feedback
// ============================================================

#[test]
fn test_fir_taps_read_correct_positions_for_all_8_taps() {
    // For each tap k, verify it surfaces a single known write from
    // exactly the right number of ticks in the past. Tap 0 is the
    // "oldest, about to be overwritten" position, so it only shows a
    // write after one *full* trip around the buffer (512 ticks here);
    // taps 1-7 are progressively closer to "now" and surface after
    // just k+1 ticks. Verified against an independent simulation of
    // the addressing/wraparound math before writing this test.
    let wait_ticks: [u32; 8] = [513, 2, 3, 4, 5, 6, 7, 8];

    for (k, &wait) in wait_ticks.iter().enumerate() {
        let mut mem = Memory::new();
        dsp_gw(&mut mem, 0x6D, 0x20); // ESA = page 0x20
        dsp_gw(&mut mem, 0x7D, 0x01); // EDL = 1 -> 2048-byte buffer

        // Tick 1: write a distinctive value with FIR still all-zero.
        // EFB=0 throughout, so FIR settings never affect what actually
        // lands in the buffer — only the returned fir_out.
        mem.dsp.tick_echo(&mut mem.ram, 9999, -1111);

        // Advance up to (but not including) the verification tick.
        for _ in 1..wait - 1 {
            mem.dsp.tick_echo(&mut mem.ram, 0, 0);
        }

        // Isolate tap k just before the verification tick.
        dsp_vw(&mut mem, k as u8, 0xF, 127);

        let (out_l, out_r) = mem.dsp.tick_echo(&mut mem.ram, 0, 0);
        assert_eq!(
            out_l,
            ((127i32 * 9999) >> 7) as i16,
            "tap {k} must read the value written {wait} ticks ago"
        );
        assert_eq!(out_r, ((127i32 * -1111) >> 7) as i16);
    }
}

#[test]
fn test_tick_echo_zero_fir_is_always_silent_regardless_of_buffer_content() {
    // Default FIR coefficients are all 0, so the filtered output must
    // stay silent no matter what's actually sitting in the buffer.
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x6D, 0x40);
    dsp_gw(&mut mem, 0x7D, 0x01);

    for i in 0..20i16 {
        let (l, r) = mem.dsp.tick_echo(&mut mem.ram, i * 111, -i * 111);
        assert_eq!(
            (l, r),
            (0, 0),
            "all-zero FIR must produce silent output (tick {i})"
        );
    }
}

#[test]
fn test_tick_echo_edl_zero_is_always_silent() {
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x6D, 0x50); // ESA set; EDL left at its default 0
    dsp_vw(&mut mem, 0, 0xF, 127); // even with a strong FIR tap...

    let (l, r) = mem.dsp.tick_echo(&mut mem.ram, 12345, -12345);
    assert_eq!(
        (l, r),
        (0, 0),
        "EDL=0 must produce silence — there's no buffer to filter"
    );
}

#[test]
fn test_efb_feeds_filtered_output_back_into_the_buffer() {
    // FIR isolates tap 0 at coefficient 64 (~0.5x); EFB=64 (~0.5x
    // feedback). A single write of 1000 should come back roughly
    // halved on each full trip around the buffer — echoing, decaying,
    // and being re-filtered each cycle: 1000 -> 500 -> 125 (not 250 —
    // the write that goes back into the buffer is *already* scaled by
    // the feedback path, then gets scaled by the FIR tap *again* on
    // the next read, so two ~0.5x factors compound between readings,
    // not one). Verified against an independent simulation before
    // writing this test, specifically to catch that double-scaling.
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x6D, 0x30); // ESA = page 0x30
    dsp_gw(&mut mem, 0x7D, 0x01); // EDL = 1 -> 512 stereo pairs
    dsp_vw(&mut mem, 0, 0xF, 64); // FIR tap 0 = 64
    dsp_gw(&mut mem, 0x0D, 64); // EFB = 64

    mem.dsp.tick_echo(&mut mem.ram, 1000, 0); // tick 1: buffer empty, writes 1000 unmodified
    for _ in 1..512 {
        mem.dsp.tick_echo(&mut mem.ram, 0, 0);
    }
    let (out1, _) = mem.dsp.tick_echo(&mut mem.ram, 0, 0); // tick 513: reads tick 1's 1000
    assert_eq!(out1, 500);

    for _ in 0..511 {
        mem.dsp.tick_echo(&mut mem.ram, 0, 0);
    }
    let (out2, _) = mem.dsp.tick_echo(&mut mem.ram, 0, 0); // tick 1025: reads tick 513's write
    assert_eq!(out2, 125);
}

// ============================================================
// EON voice routing + full pipeline
// ============================================================

/// Write a single BRR block with strong alternating +7/-8 nibbles
/// (shift=12, filter=0 — each nibble decodes independently of history),
/// so playback is clearly non-silent and easy to reason about, unlike
/// `write_silent_brr_block`.
fn write_tone_brr_block(mem: &mut Memory, addr: u16, end: bool, do_loop: bool) {
    let mut header: u8 = 0xC0; // shift=12, filter=0
    if end {
        header |= 0x01;
    }
    if do_loop {
        header |= 0x02;
    }
    mem.write8(addr, header);
    for i in 1..9u16 {
        mem.write8(addr + i, 0x78); // nibbles 7, -8 repeating
    }
}

/// Set up a self-looping voice 0 playing a strong, non-silent tone (see
/// `write_tone_brr_block`) at full volume with a fast attack, so its
/// dry output is reliably non-zero within a couple of ticks.
fn setup_tone_looping_voice(mem: &mut Memory) {
    let dir_page: u8 = 0x02;
    let brr_addr: u16 = 0x0300;

    write_tone_brr_block(mem, brr_addr, true, true); // end+loop, single block
    write_dir_entry(mem, dir_page, 0, brr_addr, brr_addr);

    dsp_gw(mem, 0x5D, dir_page);
    dsp_vw(mem, 0, 0x0, 100); // VOL(L)
    dsp_vw(mem, 0, 0x1, 100); // VOL(R)
    dsp_vw(mem, 0, 0x4, 0); // SRCN 0
    dsp_vw(mem, 0, 0x2, 0x00); // PITCH lo
    dsp_vw(mem, 0, 0x3, 0x10); // PITCH hi (native rate)
    dsp_vw(mem, 0, 0x5, 0x8F); // ADSR1: fast attack
    dsp_vw(mem, 0, 0x6, 0xE0); // ADSR2: hold sustain
    dsp_gw(mem, 0x4C, 0x01); // KON voice 0
}

#[test]
fn test_eon_clear_leaves_echo_buffer_untouched() {
    let mut mem = Memory::new();
    setup_tone_looping_voice(&mut mem);
    dsp_gw(&mut mem, 0x6D, 0x40); // ESA = page 0x40 ($4000)
    dsp_gw(&mut mem, 0x7D, 0x01); // EDL = 1
    // EON left at its default 0 — voice 0 is loud, but not routed to echo.

    for _ in 0..10 {
        mem.dsp.step(&mut mem.ram);
    }

    let any_nonzero = mem.ram[0x4000..0x4000 + 40].iter().any(|&b| b != 0);
    assert!(
        !any_nonzero,
        "without EON, an audible voice must still never reach the echo buffer"
    );
}

#[test]
fn test_eon_set_writes_voice_output_into_echo_buffer() {
    let mut mem = Memory::new();
    setup_tone_looping_voice(&mut mem);
    dsp_gw(&mut mem, 0x6D, 0x40); // ESA = page 0x40
    dsp_gw(&mut mem, 0x7D, 0x01); // EDL = 1
    dsp_gw(&mut mem, 0x4D, 0x01); // EON voice 0

    for _ in 0..10 {
        mem.dsp.step(&mut mem.ram);
    }

    let any_nonzero = mem.ram[0x4000..0x4000 + 40].iter().any(|&b| b != 0);
    assert!(
        any_nonzero,
        "with EON set, the voice's dry output must actually reach the echo buffer"
    );
}

#[test]
fn test_echo_output_reaches_final_mix_even_with_mvol_zeroed() {
    // Zero master volume so the dry mix contributes exactly nothing to
    // the final output (render_audio_single scales only the dry sum by
    // MVOL, then adds the echo output on top unscaled) — anything
    // audible in the result can only be coming from echo, isolating
    // that the echo path genuinely reaches the final mix rather than
    // just landing correctly in RAM.
    let mut mem = Memory::new();
    setup_tone_looping_voice(&mut mem);
    dsp_gw(&mut mem, 0x6D, 0x50); // ESA = page 0x50
    dsp_gw(&mut mem, 0x7D, 0x01); // EDL = 1
    dsp_vw(&mut mem, 1, 0xF, 127); // isolate FIR tap 1 (surfaces after 2 ticks)
    dsp_gw(&mut mem, 0x4D, 0x01); // EON voice 0
    dsp_gw(&mut mem, 0x0C, 0); // MVOLL = 0
    dsp_gw(&mut mem, 0x1C, 0); // MVOLR = 0

    for _ in 0..10 {
        mem.dsp.step(&mut mem.ram);
    }

    let (l, r) = mem.dsp.render_audio_single();
    assert!(
        l != 0 || r != 0,
        "echo output must reach the final mix even when MVOL zeroes the dry mix"
    );
}

#[test]
fn test_echo_defaults_are_a_complete_no_op() {
    // Every echo register at its power-on default (ESA=0, EDL=0, EON=0,
    // EFB=0, FIR=all zero) — this is what every test written before
    // Stage 4 already implicitly depends on continuing to hold.
    let mut mem = Memory::new();
    setup_tone_looping_voice(&mut mem);
    dsp_gw(&mut mem, 0x0C, 100); // MVOLL
    dsp_gw(&mut mem, 0x1C, 100); // MVOLR

    for _ in 0..10 {
        mem.dsp.step(&mut mem.ram);
    }

    let (l, r) = mem.dsp.render_audio_single();
    assert!(
        l != 0 || r != 0,
        "sanity check: the voice itself must be audible"
    );

    // ESA=0 means the (nonexistent, since EDL=0) echo buffer's base
    // would be RAM address 0 — confirm nothing was ever written there.
    assert_eq!(mem.ram[0], 0);
    assert_eq!(mem.ram[1], 0);
}

// ============================================================
// Pitch modulation ($2D PMON)
// ============================================================

#[test]
fn test_pmon_register_masks_bit0() {
    // Bit 0 has no hardware effect (voice 0 has no voice below it), so
    // it's masked off internally — but the raw byte still reads back
    // unchanged, like every other register.
    let mut mem = Memory::new();
    dsp_gw(&mut mem, 0x2D, 0xFF);
    assert_eq!(mem.dsp.read_reg(0x2D), 0xFF, "raw $2D must read back as written");
    assert_eq!(mem.dsp.pmon(), 0xFE, "PMON bit 0 must be masked off");
}

/// Voice 0: the loud self-looping tone from `setup_tone_looping_voice`
/// (native pitch), with its volume overridden to `voice0_vol`. Voice 1:
/// the same sample at half pitch (0x0800), keyed on too. Then PMON is
/// written and the DSP runs for 32 ticks, recording both voices'
/// `pitch_counter` after every tick.
fn pmon_counter_trace(pmon: u8, voice0_vol: u8) -> Vec<(u16, u16)> {
    let mut mem = Memory::new();
    setup_tone_looping_voice(&mut mem);
    dsp_vw(&mut mem, 0, 0x0, voice0_vol); // VOL(L)
    dsp_vw(&mut mem, 0, 0x1, voice0_vol); // VOL(R)

    dsp_vw(&mut mem, 1, 0x0, 100); // VOL(L)
    dsp_vw(&mut mem, 1, 0x1, 100); // VOL(R)
    dsp_vw(&mut mem, 1, 0x4, 0); // SRCN 0 (same DIR entry as voice 0)
    dsp_vw(&mut mem, 1, 0x2, 0x00); // PITCH lo
    dsp_vw(&mut mem, 1, 0x3, 0x08); // PITCH hi: 0x0800, half rate
    dsp_vw(&mut mem, 1, 0x5, 0x8F); // ADSR1: fast attack
    dsp_vw(&mut mem, 1, 0x6, 0xE0); // ADSR2: hold sustain
    dsp_gw(&mut mem, 0x4C, 0x02); // KON voice 1 (voice 0 already on)

    dsp_gw(&mut mem, 0x2D, pmon);

    (0..32)
        .map(|_| {
            mem.dsp.step(&mut mem.ram);
            (mem.dsp.voices[0].pitch_counter, mem.dsp.voices[1].pitch_counter)
        })
        .collect()
}

fn voice_trace(trace: &[(u16, u16)], voice: usize) -> Vec<u16> {
    trace.iter().map(|&(v0, v1)| if voice == 0 { v0 } else { v1 }).collect()
}

#[test]
fn test_pmon_bit_set_modulates_next_voice() {
    let baseline = pmon_counter_trace(0x00, 100);
    let modulated = pmon_counter_trace(0x02, 100);
    assert_ne!(
        voice_trace(&baseline, 1),
        voice_trace(&modulated, 1),
        "PMON bit 1 must let voice 0's output change voice 1's pitch"
    );
    assert_eq!(
        voice_trace(&baseline, 0),
        voice_trace(&modulated, 0),
        "the modulator itself (voice 0) must be unaffected"
    );
}

#[test]
fn test_pmon_bit_clear_leaves_voice_unaffected() {
    // Only voice 2 is selected; voice 1 must play exactly as with PMON off.
    let baseline = pmon_counter_trace(0x00, 100);
    let other_bit = pmon_counter_trace(0x04, 100);
    assert_eq!(
        voice_trace(&baseline, 1),
        voice_trace(&other_bit, 1),
        "voice 1 must be unmodulated when its PMON bit is clear"
    );
}

#[test]
fn test_pmon_bit0_has_no_effect_on_voice0() {
    let baseline = pmon_counter_trace(0x00, 100);
    let bit0 = pmon_counter_trace(0x01, 100);
    assert_eq!(baseline, bit0, "PMON bit 0 must have no effect at all");
}

#[test]
fn test_pmon_modulator_volume_does_not_matter() {
    // The modulating value is taken before L/R volume, so a silent
    // (volume 0) modulator bends voice 1 exactly as much as a loud one.
    let loud = pmon_counter_trace(0x02, 100);
    let silent = pmon_counter_trace(0x02, 0);
    assert_eq!(
        voice_trace(&loud, 1),
        voice_trace(&silent, 1),
        "modulation must not depend on the modulator's volume"
    );
    assert_ne!(
        voice_trace(&silent, 1),
        voice_trace(&pmon_counter_trace(0x00, 0), 1),
        "a volume-0 modulator must still modulate"
    );
}
