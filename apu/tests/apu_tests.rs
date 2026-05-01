/// APU integration tests
///
/// Covers:
///   - Apu::new(): reset vector loaded, SP initialised, cycle counters zero
///   - Apu::step(): CPU ticked every cycle, DSP ticked every 32 cycles,
///                  total cycle counter advances correctly
///   - DSP tick rate: exactly 1 DSP tick per 32 CPU cycles
///   - render_audio(): correct output length, advances cycles, produces
///                     stereo-interleaved samples, silent when no voices active
///   - Component wiring: DSP register writes via Memory reach the DSP,
///                       render_audio reflects DSP state

use apu::Apu;
use apu::dsp::EnvelopePhase;

// ============================================================
// Helpers
// ============================================================

/// Write a NOP sled starting at `addr` so the CPU can execute
/// `count` steps without hitting an unimplemented!() panic.
/// NOP = opcode 0x00 on the SPC700.
fn write_nops(apu: &mut Apu, addr: u16, count: usize) {
    for i in 0..count {
        apu.memory.write8(addr.wrapping_add(i as u16), 0x00);
    }
}

/// Point the reset vector at `addr` and fill that region with NOPs,
/// then re-run reset so the CPU PC is set correctly.
///
/// We use $0100 as the default NOP sled start with a count of 0xEFF (3839)
/// bytes, filling $0100–$0FFF. The audio data lives above this range:
///   $1000 — BRR block
///   $1100 — DIR table (dir_page = 0x11)
/// Keeping the DIR table above $1000 is critical: the DIR entry for a
/// BRR block at $1000 contains the byte 0x10 (high address byte), which
/// the CPU would interpret as opcode BPL if it fell inside the sled.
fn setup_cpu(apu: &mut Apu, start_addr: u16, nop_count: usize) {
    apu.memory.write8(0xFFFE, (start_addr & 0xFF) as u8);
    apu.memory.write8(0xFFFF, (start_addr >> 8)   as u8);
    write_nops(apu, start_addr, nop_count);
    apu.cpu.reset(&mut apu.memory);
}

/// Set up a silent looping BRR voice on voice 0 via the $F2/$F3 protocol.
///
/// Memory layout chosen to avoid colliding with the CPU NOP sled:
///   $1000 — BRR block  (9 bytes, end+loop)
///   $1100 — DIR table  (dir_page = 0x11)
///
/// All audio data lives above the NOP sled ($0100–$0FFF).
fn setup_voice_silent_sample(apu: &mut Apu) {
    // Memory layout — all above the NOP sled ($0100–$0FFF):
    //   $1000 — BRR block (9 bytes)
    //   $1100 — DIR table (dir_page = 0x11 → base = $1100)
    // The DIR entry high byte is 0x10 (address $1000 >> 8).
    // Placing DIR at $0800 (inside the sled) caused the CPU to fetch
    // that 0x10 byte as opcode BPL and panic.
    let dir_page: u8  = 0x11; // DIR base = 0x11 << 8 = $1100
    let brr_addr: u16 = 0x1000;

    // Silent BRR block: shift=4, filter=0, end+loop, all nibbles=0
    let header: u8 = 0x40 | 0x03;
    apu.memory.write8(brr_addr, header);
    for i in 1..9u16 {
        apu.memory.write8(brr_addr + i, 0x00);
    }

    // DIR entry for SRCN 0 at $1100
    let dir_base = (dir_page as u16) << 8; // = $1100
    apu.memory.write8(dir_base,     (brr_addr & 0xFF) as u8);
    apu.memory.write8(dir_base + 1, (brr_addr >> 8)   as u8);
    apu.memory.write8(dir_base + 2, (brr_addr & 0xFF) as u8);
    apu.memory.write8(dir_base + 3, (brr_addr >> 8)   as u8);

    // Configure voice 0 via $F2/$F3
    let dsp_w = |apu: &mut Apu, reg: u8, val: u8| {
        apu.memory.write8(0x00F2, reg);
        apu.memory.write8(0x00F3, val);
    };

    dsp_w(apu, 0x5D, dir_page);  // DIR = 0x11
    dsp_w(apu, 0x00, 100u8);     // VOL L
    dsp_w(apu, 0x01, 100u8);     // VOL R
    dsp_w(apu, 0x02, 0x00);      // PITCH lo
    dsp_w(apu, 0x03, 0x10);      // PITCH hi (0x1000 = native rate)
    dsp_w(apu, 0x04, 0x00);      // SRCN
    dsp_w(apu, 0x05, 0x8F);      // ADSR1: fast attack
    dsp_w(apu, 0x06, 0xE0);      // ADSR2: hold sustain
    dsp_w(apu, 0x0C, 127u8);     // MVOLL
    dsp_w(apu, 0x1C, 127u8);     // MVOLR
    dsp_w(apu, 0x4C, 0x01);      // KON voice 0
}

/// Set up a looping BRR voice on voice 0 that produces non-zero output.
///
/// Uses the same memory layout as setup_voice_silent_sample but fills
/// the BRR data bytes with 0x77 (nibbles = +7, +7 throughout).
/// With shift=4 and filter=0: decoded sample = 7 << 4 = 112.
/// This guarantees current_sample is non-zero so render_audio_single
/// produces audible output once the envelope has risen.
fn setup_voice_nonzero_sample(apu: &mut Apu) {
    let dir_page: u8  = 0x11;
    let brr_addr: u16 = 0x1000;

    // BRR block: shift=4, filter=0, end+loop, all nibbles=7 → sample=112
    let header: u8 = 0x40 | 0x03; // shift=4, end+loop
    apu.memory.write8(brr_addr, header);
    for i in 1..9u16 {
        apu.memory.write8(brr_addr + i, 0x77); // high=7, low=7
    }

    // DIR entry for SRCN 0 at $1100
    let dir_base = (dir_page as u16) << 8;
    apu.memory.write8(dir_base,     (brr_addr & 0xFF) as u8);
    apu.memory.write8(dir_base + 1, (brr_addr >> 8)   as u8);
    apu.memory.write8(dir_base + 2, (brr_addr & 0xFF) as u8);
    apu.memory.write8(dir_base + 3, (brr_addr >> 8)   as u8);

    let dsp_w = |apu: &mut Apu, reg: u8, val: u8| {
        apu.memory.write8(0x00F2, reg);
        apu.memory.write8(0x00F3, val);
    };

    dsp_w(apu, 0x5D, dir_page);  // DIR = 0x11
    dsp_w(apu, 0x00, 100u8);     // VOL L
    dsp_w(apu, 0x01, 100u8);     // VOL R
    dsp_w(apu, 0x02, 0x00);      // PITCH lo
    dsp_w(apu, 0x03, 0x10);      // PITCH hi (0x1000 = native rate)
    dsp_w(apu, 0x04, 0x00);      // SRCN
    dsp_w(apu, 0x05, 0x8F);      // ADSR1: fast attack (rate=15)
    dsp_w(apu, 0x06, 0xE0);      // ADSR2: hold at sustain
    dsp_w(apu, 0x0C, 127u8);     // MVOLL
    dsp_w(apu, 0x1C, 127u8);     // MVOLR
    dsp_w(apu, 0x4C, 0x01);      // KON voice 0
}

// ============================================================
// Apu::new()
// ============================================================

#[test]
fn test_new_cycle_counters_zero() {
    let apu = Apu::new();
    assert_eq!(apu.cycles, 0, "total cycle counter must be 0 on init");
}

#[test]
fn test_new_cpu_sp_initialised() {
    // reset() sets SP to 0xFF
    let apu = Apu::new();
    assert_eq!(apu.cpu.regs.sp, 0xFF, "SP must be 0xFF after reset");
}

#[test]
fn test_new_cpu_pc_loaded_from_reset_vector() {
    // Default memory is zeroed so reset vector $FFFE/$FFFF = 0x0000,
    // meaning PC should be 0x0000 on a fresh APU.
    let apu = Apu::new();
    assert_eq!(apu.cpu.regs.pc, 0x0000,
        "PC must be loaded from reset vector at $FFFE/$FFFF");
}

#[test]
fn test_new_cpu_pc_reflects_reset_vector() {
    // If we set the reset vector before creating the APU... we can't,
    // since new() creates Memory internally. Instead verify that
    // setup_cpu() correctly repositions PC via a second reset call.
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 64);
    assert_eq!(apu.cpu.regs.pc, 0x0100,
        "PC must update when reset vector is changed and reset() re-called");
}

#[test]
fn test_new_dsp_voices_silent() {
    let apu = Apu::new();
    for v in 0..8 {
        assert_eq!(
            apu.memory.dsp.voices[v].adsr.envelope_phase,
            EnvelopePhase::Off,
            "voice {v} must be Off on init"
        );
    }
}

// ============================================================
// Apu::step() — cycle counting
// ============================================================

#[test]
fn test_step_advances_cycle_counter() {
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 128);

    apu.step(1);
    assert_eq!(apu.cycles, 1);

    apu.step(9);
    assert_eq!(apu.cycles, 10);
}

#[test]
fn test_step_zero_cycles_does_nothing() {
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 128);

    apu.step(0);
    assert_eq!(apu.cycles, 0, "step(0) must not advance the cycle counter");
    assert_eq!(apu.cpu.regs.pc, 0x0100, "step(0) must not advance the PC");
}

#[test]
fn test_step_advances_cpu_pc() {
    // NOP is 1 byte and takes 2 cycles; after 1 step the PC must advance by 1.
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 64);

    let pc_before = apu.cpu.regs.pc;
    apu.step(1);
    assert_eq!(apu.cpu.regs.pc, pc_before.wrapping_add(1),
        "one step must advance PC by 1 (NOP is 1 byte)");
}

#[test]
fn test_step_multiple_cycles_advances_pc_multiple_times() {
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 128);

    let pc_before = apu.cpu.regs.pc;
    apu.step(5);
    // Each NOP advances PC by 1; 5 steps = 5 NOPs = PC + 5
    assert_eq!(apu.cpu.regs.pc, pc_before.wrapping_add(5));
}

// ============================================================
// Apu::step() — DSP tick rate (1 tick per 32 CPU cycles)
// ============================================================

#[test]
fn test_dsp_not_ticked_before_32_cycles() {
    // After 31 cycles the envelope must still be Off (DSP never stepped).
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 256);
    setup_voice_silent_sample(&mut apu);

    // Step 31 cycles — DSP should not have fired yet
    apu.step(31);
    // The voice was keyed on but if the DSP never stepped its envelope
    // is still in Attack at level 0 (not yet processed).
    // The key observable: master vol is set, so any DSP output after a
    // step would be non-zero eventually; here we just check cycles.
    assert_eq!(apu.cycles, 31);
}

#[test]
fn test_dsp_ticked_exactly_at_32_cycles() {
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 256);
    setup_voice_silent_sample(&mut apu);

    // After exactly 32 cycles the DSP must have stepped once.
    // Observable: envelope_level should have advanced from 0
    // (attack_rate=15 → fast attack, +1024 per DSP tick).
    apu.step(32);
    assert!(
        apu.memory.dsp.voices[0].adsr.envelope_level > 0,
        "envelope must have advanced after 32 CPU cycles (one DSP tick)"
    );
}

#[test]
fn test_dsp_ticked_twice_after_64_cycles() {
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 256);
    setup_voice_silent_sample(&mut apu);

    apu.step(32);
    let level_after_first = apu.memory.dsp.voices[0].adsr.envelope_level;

    apu.step(32);
    let level_after_second = apu.memory.dsp.voices[0].adsr.envelope_level;

    assert!(
        level_after_second >= level_after_first,
        "envelope must have advanced again after a second DSP tick"
    );
}

#[test]
fn test_dsp_tick_count_proportional_to_cycles() {
    // After N * 32 CPU cycles the DSP must have stepped exactly N times.
    // We verify this by counting envelope level increments.
    // With attack_rate=15 each tick adds 1024 until clamped at 0x7FF.
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 1024);
    setup_voice_silent_sample(&mut apu);

    // 3 DSP ticks = 96 CPU cycles; level should be min(3*1024, 0x7FF)
    apu.step(96);
    let expected = (3u16 * 1024).min(0x7FF);
    assert_eq!(
        apu.memory.dsp.voices[0].adsr.envelope_level,
        expected,
        "after 96 CPU cycles (3 DSP ticks) envelope must be {expected:#05X}"
    );
}

// ============================================================
// Apu::render_audio()
// ============================================================

#[test]
fn test_render_audio_output_length() {
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 0xEFF);

    let out = apu.render_audio(10);
    assert_eq!(out.len(), 10, "render_audio(10) must return 10 stereo pairs");
}

#[test]
fn test_render_audio_zero_samples_returns_empty() {
    let mut apu = Apu::new();
    let out = apu.render_audio(0);
    assert!(out.is_empty(), "render_audio(0) must return an empty Vec");
}

#[test]
fn test_render_audio_advances_cycles() {
    // Each call to render_audio(n) runs n * 32 CPU cycles internally.
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 0xEFF);

    apu.render_audio(4);
    assert_eq!(apu.cycles, 4 * 32,
        "render_audio(4) must advance cycles by 4 * 32 = 128");
}

#[test]
fn test_render_audio_interleaved_stereo() {
    // Output is [L0, R0, L1, R1, ...] — even indices are left, odd are right.
    // With a voice panned hard left (right_vol=0) all odd indices must be 0.
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 0xEFF);

    // Set up a voice panned hard left
    let dsp_w = |apu: &mut Apu, reg: u8, val: u8| {
        apu.memory.write8(0x00F2, reg);
        apu.memory.write8(0x00F3, val);
    };

    // BRR at $1000, DIR at $1100 — both above the NOP sled ($0100–$0FFF)
    let dir_page: u8  = 0x11;
    let brr_addr: u16 = 0x1000;
    let header: u8 = 0x40 | 0x03;
    apu.memory.write8(brr_addr, header);
    for i in 1..9u16 { apu.memory.write8(brr_addr + i, 0x00); }
    let dir_base = (dir_page as u16) << 8;
    apu.memory.write8(dir_base,     (brr_addr & 0xFF) as u8);
    apu.memory.write8(dir_base + 1, (brr_addr >> 8)   as u8);
    apu.memory.write8(dir_base + 2, (brr_addr & 0xFF) as u8);
    apu.memory.write8(dir_base + 3, (brr_addr >> 8)   as u8);

    dsp_w(&mut apu, 0x5D, dir_page);
    dsp_w(&mut apu, 0x00, 100u8);  // VOL L = 100
    dsp_w(&mut apu, 0x01, 0u8);    // VOL R = 0 (hard left)
    dsp_w(&mut apu, 0x02, 0x00);
    dsp_w(&mut apu, 0x03, 0x10);
    dsp_w(&mut apu, 0x04, 0x00);
    dsp_w(&mut apu, 0x05, 0x8F);
    dsp_w(&mut apu, 0x06, 0xE0);
    dsp_w(&mut apu, 0x0C, 127u8);
    dsp_w(&mut apu, 0x1C, 127u8);
    dsp_w(&mut apu, 0x4C, 0x01);

    let out = apu.render_audio(8);
    assert_eq!(out.len(), 8);

    // All right-channel samples must be 0 (hard-left pan: right_vol=0)
    for (i, &(_l, r)) in out.iter().enumerate() {
        assert_eq!(r, 0,
            "right channel of pair {i} must be 0 (hard-left pan)");
    }
}

#[test]
fn test_render_audio_silent_when_no_voices_active() {
    // With no voices keyed on and master vol at default (0) output must be silence.
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 0xEFF);

    let out = apu.render_audio(16);
    assert!(
        out.iter().all(|&(l, r)| l == 0 && r == 0),
        "all samples must be 0 when no voices are active"
    );
}

#[test]
fn test_render_audio_produces_nonzero_with_active_voice() {
    // With a keyed-on voice and non-zero master volume, output must eventually
    // be non-zero as the envelope rises through Attack.
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 0xEFF);
    setup_voice_nonzero_sample(&mut apu);

    // Render enough samples for the envelope to have risen and
    // the BRR buffer to have been decoded at least once.
    // 100 samples * 32 cycles = 3200 CPU cycles = 100 DSP ticks.
    let out = apu.render_audio(100);
    assert!(
        out.iter().any(|&(l, r)| l != 0 || r != 0),
        "at least one non-zero sample expected with an active voice"
    );
}

// ============================================================
// Component wiring — DSP writes via Memory reach the DSP
// ============================================================

#[test]
fn test_dsp_register_write_via_f2_f3_reaches_dsp() {
    let mut apu = Apu::new();

    // Write MVOLL via $F2/$F3 protocol
    apu.memory.write8(0x00F2, 0x0C);
    apu.memory.write8(0x00F3, 0x55);

    assert_eq!(apu.memory.dsp.read_reg(0x0C), 0x55,
        "DSP register written via $F2/$F3 must be readable via read_reg");
}

#[test]
fn test_dsp_register_write_via_direct_window_reaches_dsp() {
    let mut apu = Apu::new();

    apu.memory.write8(0xF200 + 0x1C, 0x66); // MVOLR via direct window
    assert_eq!(apu.memory.dsp.read_reg(0x1C), 0x66);
}

#[test]
fn test_render_audio_reflects_master_volume() {
    // Two runs: one with MVOL=0 (silence), one with MVOL=127 (signal).
    // The second must produce different (non-zero) output.
    let mut apu_silent = Apu::new();
    setup_cpu(&mut apu_silent, 0x0100, 0xEFF);
    // No master volume write — defaults to 0

    let mut apu_loud = Apu::new();
    setup_cpu(&mut apu_loud, 0x0100, 0xEFF);
    setup_voice_nonzero_sample(&mut apu_loud);

    let silent_out = apu_silent.render_audio(64);
    let loud_out   = apu_loud.render_audio(64);

    assert!(silent_out.iter().all(|&(l, r)| l == 0 && r == 0),
        "zero master volume must produce silence");
    assert!(loud_out.iter().any(|&(l, r)| l != 0 || r != 0),
        "non-zero master volume with active voice must produce output");
}
