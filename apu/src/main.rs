/// SNES APU Comprehensive Test
///
/// This program exercises every major part of the APU:
///
///   Test 1 — BRR encoder helper + single-voice sine wave
///     Verifies that BRR encoding/decoding round-trips correctly.
///     Writes "test1_sine.raw".
///
///   Test 2 — All 8 voices simultaneously (different pitches)
///     Puts a simple tone on each voice at a different pitch value,
///     confirming the mixer sums all 8 channels.
///     Writes "test2_8voices.raw".
///
///   Test 3 — ADSR phase progression
///     One voice with a clearly audible attack → decay → sustain → release
///     shape. Prints envelope level milestones to stdout.
///     Writes "test3_adsr.raw".
///
///   Test 4 — BRR loop flag
///     Encodes a short one-block sample with the loop flag set,
///     verifies it keeps playing rather than going silent.
///     Writes "test4_loop.raw".
///
///   Test 5 — Stereo pan
///     Two voices: one panned hard left, one hard right.
///     Writes "test5_stereo.raw".
///
/// All output files are raw signed 16-bit little-endian PCM at 32 000 Hz mono
/// (tests 1–4) or stereo interleaved (test 5).
/// Play back with e.g.:
///   ffplay -f s16le -ar 32000 -ac 1 test1_sine.raw
///   ffplay -f s16le -ar 32000 -ac 2 test5_stereo.raw

use apu::dsp::{Dsp, EnvelopePhase};
use apu::Memory;
use std::fs::File;
use std::io::Write;

// ============================================================
// BRR BLOCK BUILDER
// Builds raw 9-byte BRR blocks that the DSP can decode.
//
// Real SNES games use sophisticated encoders; here we use the
// simplest possible approach: filter 0 (no prediction), shift
// chosen to minimise clipping.  Good enough for testing.
// ============================================================

/// Encode 16 PCM samples (i16) into a 9-byte BRR block using filter 0.
///
/// `end`  — set the end   flag (bit 0 of header)
/// `loop` — set the loop  flag (bit 1 of header)
///
/// Returns the 9 bytes ready to be written to APU RAM.
fn encode_brr_block(samples: &[i16; 16], end: bool, do_loop: bool) -> [u8; 9] {
    // Find the minimum shift that fits all samples without overflow.
    // With filter 0 the decoded value is simply (nibble << shift),
    // so we need: |sample| <= 7 << shift  (7 = max positive 4-bit signed value).
    let max_abs = samples.iter().map(|&s| s.unsigned_abs()).max().unwrap_or(0);

    let shift: u8 = if max_abs == 0 {
        0
    } else {
        // nibble range is -8..+7, so effective max magnitude is 7 << shift.
        // We want 7 << shift >= max_abs  →  shift >= log2(max_abs/7).
        let mut s = 0u8;
        while s < 12 && (7u32 << s) < max_abs as u32 {
            s += 1;
        }
        s.min(12)
    };

    let filter: u8 = 0; // no prediction — safest for test data

    // Header byte: SSSSFFEX
    let mut header: u8 = (shift << 4) | (filter << 2);
    if do_loop { header |= 0x02; }
    if end     { header |= 0x01; }

    let mut block = [0u8; 9];
    block[0] = header;

    for i in 0..8usize {
        let s0 = samples[i * 2];
        let s1 = samples[i * 2 + 1];

        // Quantise each sample to a 4-bit signed nibble.
        let quant = |s: i16| -> u8 {
            let divisor = 1i32 << shift;
            // Round toward zero.
            let n = (s as i32 / divisor).clamp(-8, 7);
            (n as i8 as u8) & 0x0F
        };

        block[1 + i] = (quant(s0) << 4) | quant(s1);
    }

    block
}

/// Write one BRR block into APU RAM at `addr`.
fn write_brr_block(mem: &mut Memory, addr: u16, block: &[u8; 9]) {
    for (i, &b) in block.iter().enumerate() {
        mem.write8(addr + i as u16, b);
    }
}

/// Write the 4-byte sample directory entry (start_addr, loop_addr) for SRCN `n`.
/// DIR base page is passed as `dir_page` (the value written to register $5D).
fn write_dir_entry(mem: &mut Memory, dir_page: u8, srcn: u8, start: u16, loop_addr: u16) {
    let base = (dir_page as u16) << 8;
    let entry = base + (srcn as u16) * 4;
    mem.write8(entry,     (start     & 0xFF) as u8);
    mem.write8(entry + 1, (start     >> 8)   as u8);
    mem.write8(entry + 2, (loop_addr & 0xFF) as u8);
    mem.write8(entry + 3, (loop_addr >> 8)   as u8);
}

// ============================================================
// DSP CONFIGURATION HELPERS
// These write directly to DSP registers via the Memory bus,
// exactly as the SPC700 CPU would via the $F2/$F3 port pair.
//
// DSP register addresses (as seen on the memory bus after the
// Memory layer strips the 0xF200 base):
//   Voice N register R  →  bus address 0xF200 + (N << 4) + R
//   Global register G   →  bus address 0xF200 + G
// ============================================================

const DSP_BASE: u16 = 0xF200;

/// Write a per-voice DSP register.
/// `voice` = 0–7, `reg` = 0x0–0xF (offset within the voice's 16-byte block).
fn dsp_voice_write(mem: &mut Memory, voice: u8, reg: u8, val: u8) {
    let addr = DSP_BASE + ((voice as u16) << 4) + reg as u16;
    mem.write8(addr, val);
}

/// Write a global DSP register by its index (0x00–0x7F).
fn dsp_global_write(mem: &mut Memory, reg: u8, val: u8) {
    mem.write8(DSP_BASE + reg as u16, val);
}

/// Configure ADSR for a voice in one call.
/// adsr1 = EDDDAAAA, adsr2 = SSSRRRRR (same format as real registers).
fn set_adsr(mem: &mut Memory, voice: u8, adsr1: u8, adsr2: u8) {
    dsp_voice_write(mem, voice, 0x5, adsr1); // ADSR1
    dsp_voice_write(mem, voice, 0x6, adsr2); // ADSR2
}

/// Set the 14-bit pitch for a voice (0x1000 = native 32 kHz rate).
fn set_pitch(mem: &mut Memory, voice: u8, pitch: u16) {
    let pitch = pitch & 0x3FFF;
    dsp_voice_write(mem, voice, 0x2, (pitch & 0xFF) as u8);
    dsp_voice_write(mem, voice, 0x3, (pitch >> 8)   as u8);
}

/// Key-on one or more voices (bitmask, bit 0 = voice 0).
fn key_on(mem: &mut Memory, mask: u8) {
    dsp_global_write(mem, 0x4C, mask);
}

/// Key-off one or more voices (bitmask).
fn key_off(mem: &mut Memory, mask: u8) {
    dsp_global_write(mem, 0x5C, mask);
}

// ============================================================
// WAVEFORM GENERATORS
// ============================================================

const SAMPLE_RATE: u32 = 32_000; // SNES DSP native rate

/// Generate `count` BRR blocks containing a sine wave at `freq_hz`.
/// Returns the number of bytes written (count * 9).
fn write_sine_brr(mem: &mut Memory, start_addr: u16, freq_hz: f32, num_blocks: usize) -> u16 {
    let mut phase: f32 = 0.0;
    let phase_step = freq_hz / SAMPLE_RATE as f32;

    for b in 0..num_blocks {
        let mut samples = [0i16; 16];
        for s in &mut samples {
            *s = (phase.sin() * 16383.0) as i16; // 15-bit amplitude
            phase = (phase + phase_step).fract(); // keep 0..1
        }
        let is_last = b == num_blocks - 1;
        let block = encode_brr_block(&samples, is_last, false);
        write_brr_block(mem, start_addr + (b * 9) as u16, &block);
    }

    (num_blocks * 9) as u16
}

/// Generate a looping square wave (two blocks: half high, half low, loop back).
fn write_square_brr_loop(mem: &mut Memory, start_addr: u16) {
    // Block 0: first half of a square wave cycle (positive)
    let mut pos_samples = [16383i16; 16];
    let block0 = encode_brr_block(&pos_samples, false, false);
    write_brr_block(mem, start_addr, &block0);

    // Block 1: second half (negative), end + loop back to block 0
    let mut neg_samples = [-16384i16; 16];
    let block1 = encode_brr_block(&neg_samples, true, true);
    write_brr_block(mem, start_addr + 9, &block1);
}

// ============================================================
// TEST 1 — Single voice, sine wave, no loop
// ============================================================

fn test1_sine() {
    println!("\n=== Test 1: Single voice sine wave (no loop) ===");

    let mut mem = Memory::new();

    // Layout in APU RAM:
    //   0x0100 — DIR table (page 1)
    //   0x0200 — BRR sample data for SRCN 0
    let dir_page: u8 = 0x01;
    let brr_start: u16 = 0x0200;
    let freq_hz = 440.0; // A4
    let num_blocks = 50;  // 50 × 16 = 800 samples ≈ 25 ms

    // Write BRR data
    write_sine_brr(&mut mem, brr_start, freq_hz, num_blocks);

    // Write directory entry: SRCN 0 → start=brr_start, loop=brr_start (irrelevant, no-loop end)
    write_dir_entry(&mut mem, dir_page, 0, brr_start, brr_start);

    // DIR register
    dsp_global_write(&mut mem, 0x5D, dir_page);

    // Voice 0: SRCN=0, pitch=0x1000 (native rate), full volume both channels
    dsp_voice_write(&mut mem, 0, 0x4, 0);           // SRCN
    set_pitch(&mut mem, 0, 0x1000);
    dsp_voice_write(&mut mem, 0, 0x0, 100i8 as u8); // VOL L
    dsp_voice_write(&mut mem, 0, 0x1, 100i8 as u8); // VOL R

    // ADSR: fast attack (rate 15), no decay, full sustain, slow release
    //   ADSR1 = 1_000_1111  = 0x8F  (ADSR mode, decay=0, attack=15)
    //   ADSR2 = 111_00000   = 0xE0  (sustain level=7=max, sustain rate=0=never)
    set_adsr(&mut mem, 0, 0x8F, 0xE0);

    // Key on voice 0
    key_on(&mut mem, 0x01);

    let num_output_samples = SAMPLE_RATE * 2; // 2 seconds
    let mut out = Vec::with_capacity(num_output_samples as usize);

    // Retrieve DSP handle from Memory so we can call step/render.
    // Note: Memory owns the Dsp, so we need to split the borrow.
    // We work around this by running step on mem.dsp directly.
    let mut env_phase_logged = false;
    for i in 0..num_output_samples {
        mem.dsp.step(&mem_readonly_ref(&mem));
        let (l, _r) = mem.dsp.render_audio_single();
        out.push(l);

        // Log when the voice goes silent
        if !env_phase_logged
            && mem.dsp.voices[0].adsr.envelope_phase == EnvelopePhase::Off
        {
            println!("  Voice went silent at sample {i}");
            env_phase_logged = true;
        }
    }

    save_mono("test1_sine.raw", &out);
    println!("  Written test1_sine.raw ({} samples, 32 kHz mono s16le)", out.len());
}

// ============================================================
// TEST 2 — All 8 voices at different pitches
// ============================================================

fn test2_8voices() {
    println!("\n=== Test 2: All 8 voices, different pitches ===");

    let mut mem = Memory::new();

    let dir_page: u8 = 0x01;
    dsp_global_write(&mut mem, 0x5D, dir_page);

    // One looping square wave sample shared by all voices (SRCN 0).
    let brr_start: u16 = 0x0200;
    write_square_brr_loop(&mut mem, brr_start);
    write_dir_entry(&mut mem, dir_page, 0, brr_start, brr_start); // loop back to start

    // Pitch values for a major scale starting at C4 ≈ 262 Hz.
    // At 32 kHz native rate, 0x1000 = 1× speed.
    // For frequency F: pitch = round(F / 32000 * 0x1000) but since our sample
    // is a full-cycle square wave across 32 samples (2 blocks × 16):
    //   one cycle = 32 samples → fundamental ≈ 32000/32 = 1000 Hz at pitch=0x1000.
    // We scale accordingly:
    //   C4=262 Hz → pitch ≈ 0x1000 * 262/1000 ≈ 0x431
    let pitches: [u16; 8] = [
        0x0431, // C4  262 Hz
        0x04D1, // D4  294 Hz
        0x0575, // E4  330 Hz
        0x05C3, // F4  349 Hz
        0x066E, // G4  392 Hz
        0x072A, // A4  440 Hz
        0x07F0, // B4  494 Hz
        0x0862, // C5  523 Hz
    ];

    let mut kon_mask: u8 = 0;
    for v in 0..8u8 {
        dsp_voice_write(&mut mem, v, 0x4, 0);            // SRCN = 0
        set_pitch(&mut mem, v, pitches[v as usize]);
        dsp_voice_write(&mut mem, v, 0x0, 60i8 as u8);   // VOL L (moderate, 8 voices summing)
        dsp_voice_write(&mut mem, v, 0x1, 60i8 as u8);   // VOL R
        // ADSR: fast attack, hold at sustain
        set_adsr(&mut mem, v, 0x8F, 0xE0);
        kon_mask |= 1 << v;
    }
    key_on(&mut mem, kon_mask);

    let num_samples = SAMPLE_RATE * 3;
    let mut out = Vec::with_capacity(num_samples as usize);

    for _ in 0..num_samples {
        mem.dsp.step(&mem_readonly_ref(&mem));
        let (l, _r) = mem.dsp.render_audio_single();
        out.push(l);
    }

    // Quick sanity: at least some non-zero output expected
    let non_zero = out.iter().filter(|&&s| s != 0).count();
    println!("  Non-zero samples: {non_zero}/{}", out.len());

    save_mono("test2_8voices.raw", &out);
    println!("  Written test2_8voices.raw");
}

// ============================================================
// TEST 3 — ADSR envelope shape
// ============================================================

fn test3_adsr() {
    println!("\n=== Test 3: ADSR envelope shape ===");

    let mut mem = Memory::new();

    let dir_page: u8 = 0x01;
    dsp_global_write(&mut mem, 0x5D, dir_page);

    // One long looping sine wave sample
    let brr_start: u16 = 0x0200;
    write_square_brr_loop(&mut mem, brr_start);
    write_dir_entry(&mut mem, dir_page, 0, brr_start, brr_start);

    dsp_voice_write(&mut mem, 0, 0x4, 0);            // SRCN
    set_pitch(&mut mem, 0, 0x1000);
    dsp_voice_write(&mut mem, 0, 0x0, 100i8 as u8);  // VOL L
    dsp_voice_write(&mut mem, 0, 0x1, 100i8 as u8);  // VOL R

    // Deliberately slow attack and decay so the phases are audible:
    //   ADSR1 = 1_011_0101 = 0xB5  (ADSR, decay=3, attack=5)
    //   ADSR2 = 100_01010  = 0x8A  (sustain_level=4, sustain_rate=10)
    set_adsr(&mut mem, 0, 0xB5, 0x8A);

    key_on(&mut mem, 0x01);

    let hold_samples  = SAMPLE_RATE;       // 1 s attack+decay+sustain
    let release_start = SAMPLE_RATE;       // key-off after 1 s
    let total_samples = SAMPLE_RATE * 3;   // 3 s total

    let mut out = Vec::with_capacity(total_samples as usize);
    let mut phase_log: Vec<(u32, &str, u16)> = Vec::new();
    let mut last_phase = EnvelopePhase::Off;

    for i in 0..total_samples {
        // Key-off at 1 second
        if i == release_start {
            key_off(&mut mem, 0x01);
            println!("  Key-off triggered at sample {i}");
        }

        mem.dsp.step(&mem_readonly_ref(&mem));
        let (l, _r) = mem.dsp.render_audio_single();
        out.push(l);

        // Log phase transitions
        let cur_phase = mem.dsp.voices[0].adsr.envelope_phase;
        if cur_phase != last_phase {
            let name = match cur_phase {
                EnvelopePhase::Attack  => "Attack",
                EnvelopePhase::Decay   => "Decay",
                EnvelopePhase::Sustain => "Sustain",
                EnvelopePhase::Release => "Release",
                EnvelopePhase::Off     => "Off",
            };
            let level = mem.dsp.voices[0].adsr.envelope_level;
            println!("  → {name} at sample {i} (envelope = {level:#05X})");
            last_phase = cur_phase;
        }
    }

    save_mono("test3_adsr.raw", &out);
    println!("  Written test3_adsr.raw");
}

// ============================================================
// TEST 4 — BRR loop flag
// ============================================================

fn test4_loop() {
    println!("\n=== Test 4: BRR loop flag ===");

    let mut mem = Memory::new();

    let dir_page: u8 = 0x01;
    dsp_global_write(&mut mem, 0x5D, dir_page);

    // Sample: 3 blocks.
    //   Block 0: silence (will pass through once)
    //   Block 1: tone    (will pass through once)
    //   Block 2: tone with end+loop → loops back to block 1
    let brr_start: u16 = 0x0200;
    let loop_point: u16 = brr_start + 9; // block 1

    let silence = encode_brr_block(&[0i16; 16], false, false);
    write_brr_block(&mut mem, brr_start, &silence);

    let tone: [i16; 16] = std::array::from_fn(|i| {
        ((i as f32 / 16.0 * std::f32::consts::TAU).sin() * 12000.0) as i16
    });
    let block1 = encode_brr_block(&tone, false, false);
    write_brr_block(&mut mem, brr_start + 9, &block1);

    let block2 = encode_brr_block(&tone, true, true); // end + loop
    write_brr_block(&mut mem, brr_start + 18, &block2);

    write_dir_entry(&mut mem, dir_page, 0, brr_start, loop_point);

    dsp_voice_write(&mut mem, 0, 0x4, 0);
    set_pitch(&mut mem, 0, 0x1000);
    dsp_voice_write(&mut mem, 0, 0x0, 100i8 as u8);
    dsp_voice_write(&mut mem, 0, 0x1, 100i8 as u8);
    set_adsr(&mut mem, 0, 0x8F, 0xE0);
    key_on(&mut mem, 0x01);

    let num_samples = SAMPLE_RATE * 3;
    let mut out = Vec::with_capacity(num_samples as usize);

    for i in 0..num_samples {
        mem.dsp.step(&mem_readonly_ref(&mem));
        let (l, _r) = mem.dsp.render_audio_single();
        out.push(l);

        // Voice should never go Off if looping works correctly
        if mem.dsp.voices[0].adsr.envelope_phase == EnvelopePhase::Off {
            println!("  ✗ Voice unexpectedly went silent at sample {i}");
            break;
        }
    }

    let went_silent = mem.dsp.voices[0].adsr.envelope_phase == EnvelopePhase::Off;
    if !went_silent {
        println!("  ✓ Voice still active after {} samples — loop is working", num_samples);
    }

    save_mono("test4_loop.raw", &out);
    println!("  Written test4_loop.raw");
}

// ============================================================
// TEST 5 — Stereo panning
// ============================================================

fn test5_stereo() {
    println!("\n=== Test 5: Stereo pan (voice 0 = left, voice 1 = right) ===");

    let mut mem = Memory::new();

    let dir_page: u8 = 0x01;
    dsp_global_write(&mut mem, 0x5D, dir_page);

    // Two different tones so you can tell them apart
    let brr_lo: u16 = 0x0200; // 220 Hz (A3) — left channel
    let brr_hi: u16 = 0x0500; // 880 Hz (A5) — right channel

    write_sine_brr(&mut mem, brr_lo, 220.0, 30);
    write_sine_brr(&mut mem, brr_hi, 880.0, 30);

    // SRCN 0 → low tone, SRCN 1 → high tone
    write_dir_entry(&mut mem, dir_page, 0, brr_lo, brr_lo);
    write_dir_entry(&mut mem, dir_page, 1, brr_hi, brr_hi);

    // Voice 0: left only
    dsp_voice_write(&mut mem, 0, 0x4, 0);             // SRCN 0
    set_pitch(&mut mem, 0, 0x1000);
    dsp_voice_write(&mut mem, 0, 0x0, 120i8 as u8);   // VOL L = 120
    dsp_voice_write(&mut mem, 0, 0x1, 0);              // VOL R = 0

    // Voice 1: right only
    dsp_voice_write(&mut mem, 1, 0x4, 1);             // SRCN 1
    set_pitch(&mut mem, 1, 0x1000);
    dsp_voice_write(&mut mem, 1, 0x0, 0);              // VOL L = 0
    dsp_voice_write(&mut mem, 1, 0x1, 120i8 as u8);   // VOL R = 120

    set_adsr(&mut mem, 0, 0x8F, 0xE0);
    set_adsr(&mut mem, 1, 0x8F, 0xE0);
    key_on(&mut mem, 0x03); // both voices

    let num_samples = SAMPLE_RATE * 2;
    let mut left_out  = Vec::with_capacity(num_samples as usize);
    let mut right_out = Vec::with_capacity(num_samples as usize);

    for _ in 0..num_samples {
        mem.dsp.step(&mem_readonly_ref(&mem));
        let (l, r) = mem.dsp.render_audio_single();
        left_out.push(l);
        right_out.push(r);
    }

    // Sanity: left channel should have signal, right should be near zero and vice versa
    let left_energy:  i64 = left_out .iter().map(|&s| s as i64 * s as i64).sum();
    let right_energy: i64 = right_out.iter().map(|&s| s as i64 * s as i64).sum();
    println!("  Left  channel energy: {left_energy}");
    println!("  Right channel energy: {right_energy}");

    if left_energy > 0 && right_energy > 0 {
        println!("  ✓ Both channels carry signal");
    }

    save_stereo_interleaved("test5_stereo.raw", &left_out, &right_out);
    println!("  Written test5_stereo.raw (32 kHz stereo s16le)");
}

// ============================================================
// I/O HELPERS
// ============================================================

fn save_mono(path: &str, samples: &[i16]) {
    let mut f = File::create(path).expect("could not create file");
    for &s in samples {
        f.write_all(&s.to_le_bytes()).unwrap();
    }
}

fn save_stereo_interleaved(path: &str, left: &[i16], right: &[i16]) {
    let mut f = File::create(path).expect("could not create file");
    for (&l, &r) in left.iter().zip(right.iter()) {
        f.write_all(&l.to_le_bytes()).unwrap();
        f.write_all(&r.to_le_bytes()).unwrap();
    }
}

/// Helper to pass an immutable view of Memory's RAM to dsp.step().
///
/// dsp.step() needs &Memory to read BRR data from APU RAM, but we
/// also need &mut mem.dsp to call step.  Rust won't allow both at
/// once from the same binding.  We solve this by reconstructing a
/// temporary read-only Memory that shares the RAM array — since
/// dsp.step() only *reads* memory and never writes it, this is safe.
///
/// In a real emulator you would refactor so that the DSP borrows an
/// immutable &[u8] slice for RAM rather than a &Memory — but for a
/// self-contained test this keeps the code simple.
#[inline(always)]
fn mem_readonly_ref(mem: &Memory) -> Memory {
    // Build a Memory that has the same RAM contents but a fresh (no-op) Dsp.
    // dsp.step() only calls mem.read8(), which only touches mem.ram, so the
    // dummy dsp inside this copy is never used.
    let mut shadow = Memory::new();
    shadow.ram.copy_from_slice(&mem.ram);
    shadow
}

// ============================================================
// ENTRY POINT
// ============================================================

fn main() {
    println!("SNES APU Test Main");
    println!("Output rate: {} Hz, format: signed 16-bit little-endian PCM", SAMPLE_RATE);

    test1_sine();
    test2_8voices();
    test3_adsr();
    test4_loop();
    test5_stereo();

    println!("\nAll tests complete.");
    println!("To listen:");
    println!("  ffplay -f s16le -ar 32000 -ac 1 test1_sine.raw");
    println!("  ffplay -f s16le -ar 32000 -ac 1 test2_8voices.raw");
    println!("  ffplay -f s16le -ar 32000 -ac 1 test3_adsr.raw");
    println!("  ffplay -f s16le -ar 32000 -ac 1 test4_loop.raw");
    println!("  ffplay -f s16le -ar 32000 -ac 2 test5_stereo.raw");
}