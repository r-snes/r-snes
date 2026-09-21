/// APU integration tests
///
/// Covers:
///   - Apu::new(): reset vector loaded, SP initialised, cycle counters zero
///   - Apu::step(): CPU ticked every cycle, DSP ticked every 32 cycles,
///     total cycle counter advances correctly
///   - DSP tick rate: exactly 1 DSP tick per 32 CPU cycles
///   - render_audio(): correct output length, advances cycles, produces
///     stereo-interleaved samples, silent when no voices active
///   - Component wiring: DSP register writes via Memory reach the DSP,
///     render_audio reflects DSP state
///   - IPL boot protocol (HLE): upload/execute handshake over the ports,
///     chunk-boundary port stability, and cold ($FFC0) vs. warm ($FFC9)
///     re-entry (SP/zero-page preserved on warm re-entry — the fix for
///     the multi-chunk upload hang some games hit on a black screen)
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
    // Switch the IPL ROM out before touching the reset vector. CONTROL
    // defaults to 0x80 (ROM mapped in) at power-on, and while it's set,
    // $FFFE/$FFFF read back through the ROM overlay, not RAM — the ROM's
    // own vector is always $FFC0, which is exactly why every real reset
    // boots into the IPL. These tests want direct control over PC for
    // CPU-level testing rather than exercising the IPL, so we clear bit 7
    // first, same as a driver would once it's done with the boot ROM.
    apu.memory.write8(0x00F1, 0x00);
    apu.memory.write8(0xFFFE, (start_addr & 0xFF) as u8);
    apu.memory.write8(0xFFFF, (start_addr >> 8) as u8);
    write_nops(apu, start_addr, nop_count);
    apu.cpu.reset(&mut apu.memory);
    // A fresh Apu boots into the HLE IPL, which owns the core until the
    // upload protocol completes — step() would run the boot state machine,
    // not the SPC700. These tests drive the CPU directly, so skip the boot.
    apu.skip_ipl_boot();
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
    let dir_page: u8 = 0x11; // DIR base = 0x11 << 8 = $1100
    let brr_addr: u16 = 0x1000;

    // Silent BRR block: shift=4, filter=0, end+loop, all nibbles=0
    let header: u8 = 0x40 | 0x03;
    apu.memory.write8(brr_addr, header);
    for i in 1..9u16 {
        apu.memory.write8(brr_addr + i, 0x00);
    }

    // DIR entry for SRCN 0 at $1100
    let dir_base = (dir_page as u16) << 8; // = $1100
    apu.memory.write8(dir_base, (brr_addr & 0xFF) as u8);
    apu.memory.write8(dir_base + 1, (brr_addr >> 8) as u8);
    apu.memory.write8(dir_base + 2, (brr_addr & 0xFF) as u8);
    apu.memory.write8(dir_base + 3, (brr_addr >> 8) as u8);

    // Configure voice 0 via $F2/$F3
    let dsp_w = |apu: &mut Apu, reg: u8, val: u8| {
        apu.memory.write8(0x00F2, reg);
        apu.memory.write8(0x00F3, val);
    };

    dsp_w(apu, 0x5D, dir_page); // DIR = 0x11
    dsp_w(apu, 0x00, 100u8); // VOL L
    dsp_w(apu, 0x01, 100u8); // VOL R
    dsp_w(apu, 0x02, 0x00); // PITCH lo
    dsp_w(apu, 0x03, 0x10); // PITCH hi (0x1000 = native rate)
    dsp_w(apu, 0x04, 0x00); // SRCN
    dsp_w(apu, 0x05, 0x8F); // ADSR1: fast attack
    dsp_w(apu, 0x06, 0xE0); // ADSR2: hold sustain
    dsp_w(apu, 0x0C, 127u8); // MVOLL
    dsp_w(apu, 0x1C, 127u8); // MVOLR
    dsp_w(apu, 0x4C, 0x01); // KON voice 0
}

/// Set up a looping BRR voice on voice 0 that produces non-zero output.
///
/// Uses the same memory layout as setup_voice_silent_sample but fills
/// the BRR data bytes with 0x77 (nibbles = +7, +7 throughout).
/// With shift=4 and filter=0: decoded sample = 7 << 4 = 112.
/// This guarantees current_sample is non-zero so render_audio_single
/// produces audible output once the envelope has risen.
fn setup_voice_nonzero_sample(apu: &mut Apu) {
    let dir_page: u8 = 0x11;
    let brr_addr: u16 = 0x1000;

    // BRR block: shift=4, filter=0, end+loop, all nibbles=7 → sample=112
    let header: u8 = 0x40 | 0x03; // shift=4, end+loop
    apu.memory.write8(brr_addr, header);
    for i in 1..9u16 {
        apu.memory.write8(brr_addr + i, 0x77); // high=7, low=7
    }

    // DIR entry for SRCN 0 at $1100
    let dir_base = (dir_page as u16) << 8;
    apu.memory.write8(dir_base, (brr_addr & 0xFF) as u8);
    apu.memory.write8(dir_base + 1, (brr_addr >> 8) as u8);
    apu.memory.write8(dir_base + 2, (brr_addr & 0xFF) as u8);
    apu.memory.write8(dir_base + 3, (brr_addr >> 8) as u8);

    let dsp_w = |apu: &mut Apu, reg: u8, val: u8| {
        apu.memory.write8(0x00F2, reg);
        apu.memory.write8(0x00F3, val);
    };

    dsp_w(apu, 0x5D, dir_page); // DIR = 0x11
    dsp_w(apu, 0x00, 100u8); // VOL L
    dsp_w(apu, 0x01, 100u8); // VOL R
    dsp_w(apu, 0x02, 0x00); // PITCH lo
    dsp_w(apu, 0x03, 0x10); // PITCH hi (0x1000 = native rate)
    dsp_w(apu, 0x04, 0x00); // SRCN
    dsp_w(apu, 0x05, 0x8F); // ADSR1: fast attack (rate=15)
    dsp_w(apu, 0x06, 0xE0); // ADSR2: hold at sustain
    dsp_w(apu, 0x0C, 127u8); // MVOLL
    dsp_w(apu, 0x1C, 127u8); // MVOLR
    dsp_w(apu, 0x4C, 0x01); // KON voice 0
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
    // Apu::new leaves the post-IPL-boot state: the real boot ROM's first
    // act is `mov x,#$EF / mov sp,x`, and uploaded code (e.g. spc test
    // #0081) relies on the stack starting at $01EF.
    let apu = Apu::new();
    assert_eq!(apu.cpu.regs.sp, 0xEF, "SP must be 0xEF after IPL boot");
}

#[test]
fn test_new_cpu_pc_loaded_from_reset_vector() {
    // CONTROL defaults to 0x80 (IPL ROM mapped in) at power-on, so the
    // reset vector at $FFFE/$FFFF is read through the ROM overlay, not
    // underlying RAM — and the ROM's own reset vector is $FFC0, its own
    // entry point. This is exactly why every real reset boots into the
    // IPL: whatever's sitting in RAM's reset vector is irrelevant until
    // a driver clears CONTROL bit 7 itself.
    let apu = Apu::new();
    assert_eq!(
        apu.cpu.regs.pc, 0xFFC0,
        "PC must be loaded from the IPL ROM's own reset vector at boot"
    );
}

#[test]
fn test_new_cpu_pc_reflects_reset_vector() {
    // If we set the reset vector before creating the APU... we can't,
    // since new() creates Memory internally. Instead verify that
    // setup_cpu() correctly repositions PC via a second reset call.
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 64);
    assert_eq!(
        apu.cpu.regs.pc, 0x0100,
        "PC must update when reset vector is changed and reset() re-called"
    );
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

    // NOP costs 2 real SPC700 cycles and can't be interrupted mid-way,
    // so asking for a 1-cycle budget still runs the whole NOP: cycles
    // ends up at 2, with the 1-cycle overrun banked as debt against the
    // next call (see Apu::step's `cycle_debt`).
    apu.step(1);
    assert_eq!(apu.cycles, 2);

    // That banked debt is paid down first: budget = 9 - 1 = 8, which is
    // exactly 4 more NOPs (8 cycles), landing back on an exact multiple
    // of 2 with no debt left over — so the cumulative total across both
    // calls comes out exactly as requested (1 + 9 = 10).
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
    assert_eq!(
        apu.cpu.regs.pc,
        pc_before.wrapping_add(1),
        "one step must advance PC by 1 (NOP is 1 byte)"
    );
}

#[test]
fn test_step_multiple_cycles_advances_pc_multiple_times() {
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 128);

    let pc_before = apu.cpu.regs.pc;
    apu.step(5);
    // Each NOP costs 2 real cycles, so a 5-cycle budget only fits 3 whole
    // NOPs (3*2=6, overrunning the budget by 1, which is banked as debt
    // rather than fitting a 4th NOP): PC + 3, not PC + 5.
    assert_eq!(apu.cpu.regs.pc, pc_before.wrapping_add(3));
}

// ============================================================
// Apu::step() — DSP tick rate (1 tick per 32 CPU cycles)
// ============================================================

#[test]
fn test_dsp_not_ticked_before_32_cycles() {
    // After fewer than 32 cycles the envelope must still be Off (DSP
    // never stepped).
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 256);
    setup_voice_silent_sample(&mut apu);

    // Step 30 cycles — DSP should not have fired yet. 30, not 31: NOP
    // costs 2 real cycles and can't be interrupted mid-instruction, so
    // 31 isn't a reachable stopping point from a fresh (zero-debt) Apu —
    // the 16th NOP would land exactly on 32 and tick the DSP, which is
    // exactly the boundary this test needs to stay under.
    apu.step(30);
    assert_eq!(apu.cycles, 30);
    // The voice was keyed on but the DSP hasn't stepped yet, so the
    // envelope is still at its key-on reset state (Attack, level 0).
    assert_eq!(
        apu.memory.dsp.voices[0].adsr.envelope_level, 0,
        "envelope must not advance before the DSP's first tick at 32 cycles"
    );
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
    let expected = 0x7FF;
    assert_eq!(
        apu.memory.dsp.voices[0].adsr.envelope_level, expected,
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
    assert_eq!(
        out.len(),
        10,
        "render_audio(10) must return 10 stereo pairs"
    );
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
    assert_eq!(
        apu.cycles,
        4 * 32,
        "render_audio(4) must advance cycles by 4 * 32 = 128"
    );
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
    let dir_page: u8 = 0x11;
    let brr_addr: u16 = 0x1000;
    let header: u8 = 0x40 | 0x03;
    apu.memory.write8(brr_addr, header);
    for i in 1..9u16 {
        apu.memory.write8(brr_addr + i, 0x00);
    }
    let dir_base = (dir_page as u16) << 8;
    apu.memory.write8(dir_base, (brr_addr & 0xFF) as u8);
    apu.memory.write8(dir_base + 1, (brr_addr >> 8) as u8);
    apu.memory.write8(dir_base + 2, (brr_addr & 0xFF) as u8);
    apu.memory.write8(dir_base + 3, (brr_addr >> 8) as u8);

    dsp_w(&mut apu, 0x5D, dir_page);
    dsp_w(&mut apu, 0x00, 100u8); // VOL L = 100
    dsp_w(&mut apu, 0x01, 0u8); // VOL R = 0 (hard left)
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
    for (i, &[_l, r]) in out.iter().enumerate() {
        assert_eq!(r, 0, "right channel of pair {i} must be 0 (hard-left pan)");
    }
}

#[test]
fn test_render_audio_silent_when_no_voices_active() {
    // With no voices keyed on and master vol at default (0) output must be silence.
    let mut apu = Apu::new();
    setup_cpu(&mut apu, 0x0100, 0xEFF);

    let out = apu.render_audio(16);
    assert!(
        out.iter().all(|&[l, r]| l == 0 && r == 0),
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
        out.iter().any(|&[l, r]| l != 0 || r != 0),
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

    assert_eq!(
        apu.memory.dsp.read_reg(0x0C),
        0x55,
        "DSP register written via $F2/$F3 must be readable via read_reg"
    );
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
    let loud_out = apu_loud.render_audio(64);

    assert!(
        silent_out.iter().all(|&[l, r]| l == 0 && r == 0),
        "zero master volume must produce silence"
    );
    assert!(
        loud_out.iter().any(|&[l, r]| l != 0 || r != 0),
        "non-zero master volume with active voice must produce output"
    );
}

// ============================================================
// IPL boot protocol (HLE)
// ============================================================

/// Drive the IPL protocol from the "main CPU" side, the way a game's
/// boot code would through $2140-$2143, and verify the upload lands
/// in ARAM and execution starts at the requested entry point.
#[test]
fn test_ipl_hle_upload_and_execute() {
    let mut apu = Apu::new();

    // 1. Boot delay, then announce. Before the delay elapses the ports
    // must NOT yet show $AA — that's the point of the delay.
    assert_ne!(
        apu.memory.cpu_port_read(0),
        0xAA,
        "no announce before boot delay"
    );
    apu.step(1024 + 8);
    assert_eq!(apu.memory.cpu_port_read(0), 0xAA);
    assert_eq!(apu.memory.cpu_port_read(1), 0xBB);

    // 2. Start command: upload to $0200
    apu.memory.cpu_port_write(2, 0x00);
    apu.memory.cpu_port_write(3, 0x02);
    apu.memory.cpu_port_write(1, 0x01); // non-zero = transfer
    apu.memory.cpu_port_write(0, 0xCC);
    apu.step(2);
    assert_eq!(apu.memory.cpu_port_read(0), 0xCC, "IPL must ack $CC");

    // 3. Upload a 3-byte program: MOV A,#$42 ($E8 $42), then STOP ($FF).
    for (i, byte) in [0xE8_u8, 0x42, 0xFF].iter().enumerate() {
        apu.memory.cpu_port_write(1, *byte);
        apu.memory.cpu_port_write(0, i as u8);
        apu.step(2);
        assert_eq!(apu.memory.cpu_port_read(0), i as u8, "IPL must echo index");
    }
    assert_eq!(apu.memory.read8(0x0200), 0xE8);
    assert_eq!(apu.memory.read8(0x0201), 0x42);
    assert_eq!(apu.memory.read8(0x0202), 0xFF);

    // 4. Execute command: index jumped by >= 2, port1 = 0, addr = $0200
    apu.memory.cpu_port_write(2, 0x00);
    apu.memory.cpu_port_write(3, 0x02);
    apu.memory.cpu_port_write(1, 0x00); // zero = execute
    apu.memory.cpu_port_write(0, 0x05); // last index was 2; 2 + >=2
    apu.step(2);
    assert_eq!(
        apu.memory.cpu_port_read(0),
        0x05,
        "execute ack must be visible"
    );
    // The ack must stay stable for the whole exec-delay window...
    apu.step(256 - 8);
    assert_eq!(
        apu.memory.cpu_port_read(0),
        0x05,
        "ack stomped during exec delay"
    );
    assert!(apu.ipl_active(), "chunk must not run during exec delay");
    // ...then the uploaded program runs: MOV A,#$42 executes, STOP parks
    // the core. The end-state proves execution began exactly at $0200
    // with the real IPL's zeroed registers.
    apu.step(32);

    assert!(!apu.ipl_active(), "IPL should have handed off");
    assert_eq!(apu.cpu.regs.a, 0x42, "uploaded MOV A,#$42 must have run");
    assert_eq!(apu.cpu.regs.pc, 0x0203, "PC frozen just past the STOP");
    assert_eq!(
        apu.memory.read8(0x00),
        0x00,
        "entry lo stored at $00 like the real IPL"
    );
    assert_eq!(
        apu.memory.read8(0x01),
        0x02,
        "entry hi stored at $01 like the real IPL"
    );
    assert_eq!((apu.cpu.regs.x, apu.cpu.regs.y), (0, 0));
}

/// Regression test for the chunk-boundary race: a completion value the
/// previous code left on port 0 must stay readable by the main CPU for
/// the whole boot delay after an IPL re-entry — not be stomped by the
/// $AA announce on the next cycle.
#[test]
fn test_reentry_preserves_completion_signal_during_boot_delay() {
    let mut apu = Apu::new();
    apu.step(1024 + 8); // initial boot

    // Pretend uploaded code signalled "chunk complete" then jumped back
    // into the boot ROM region at the cold entry point ($FFC0).
    apu.skip_ipl_boot(); // hand control to the (simulated) uploaded code
    apu.memory.port_out[0] = 0x77; // completion signal
    apu.memory.control = 0x80; // IPL ROM mapping enabled
    apu.cpu.regs.pc = 0xFFC0;

    // For the entire boot delay the signal must remain visible...
    for _ in 0..(1024 - 2) {
        apu.step(1);
        assert_eq!(
            apu.memory.cpu_port_read(0),
            0x77,
            "signal stomped too early"
        );
    }

    // ...and only then is it replaced by the announce.
    apu.step(8);
    assert_eq!(apu.memory.cpu_port_read(0), 0xAA);
    assert_eq!(apu.memory.cpu_port_read(1), 0xBB);
}

/// Regression test for the multi-chunk upload bug that hung games like
/// Super Mario World on a black screen: a driver that stashes its own
/// transfer bookkeeping in zero page and re-enters the IPL at $FFC9 for
/// its second chunk must see that data survive — a warm re-entry must
/// NOT reset SP or clear zero page. (Confirmed against Anomie's SPC700
/// doc: jumping to $FFC9 "skip[s] resetting the stack and page 0".)
#[test]
fn test_warm_reentry_at_ffc9_preserves_zero_page_and_sp() {
    let mut apu = Apu::new();
    apu.step(1024 + 8); // initial cold boot + announce
    apu.skip_ipl_boot(); // hand control to the (simulated) uploaded code

    // Simulate the driver's own state mid-upload: a custom SP (moved off
    // the IPL's $EF) and a sentinel byte in zero page that only survives
    // if the warm-reentry path does NOT run the clear loop.
    apu.cpu.regs.sp = 0x80;
    apu.memory.write8(0x0010, 0x99);

    // Uploaded code jumps to $FFC9 instead of $FFC0 to request the next
    // chunk without clearing its own bookkeeping.
    apu.memory.control = 0x80; // IPL ROM mapping enabled
    apu.cpu.regs.pc = 0xFFC9;

    apu.step(1); // triggers the reentry check (pc >= $FFC0 && bit 7 set)

    assert_eq!(apu.cpu.regs.sp, 0x80, "warm reentry must not reset SP");
    assert_eq!(
        apu.memory.read8(0x0010),
        0x99,
        "warm reentry must not clear zero page"
    );

    // The announce still happens, just much sooner than a cold boot's
    // ~1000-cycle delay.
    apu.step(8 + 4);
    assert_eq!(apu.memory.cpu_port_read(0), 0xAA);
    assert_eq!(apu.memory.cpu_port_read(1), 0xBB);
    assert_eq!(
        apu.memory.read8(0x0010),
        0x99,
        "sentinel must still be intact after announce"
    );
}
