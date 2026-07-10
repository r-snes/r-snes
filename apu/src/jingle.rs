//! The "no ROM loaded" waiting music.
//!
//! Nothing here bypasses the emulator: the jingle is a real SPC700 sound
//! driver (hand-assembled below, byte-verified against a reference
//! simulation — see the golden tests) plus a BRR-encoded tone, uploaded
//! into a freshly booted `Apu` by acting as the main CPU and speaking the
//! IPL boot protocol through the communication ports. Once the execute
//! handoff fires, everything you hear is the SPC700 sequencing notes off
//! timer 0 and the DSP decoding BRR through the ADSR envelope — the same
//! path a game's sound driver takes.
//!
//! ARAM layout (all uploaded as one contiguous block):
//! ```text
//! $0200  driver code (entry point)
//! $02C0  song table
//! $0300  sample directory (DIR): 1 entry -> $0310
//! $0310  BRR sample: one looped 16-sample block
//! ```
//!
//! Song table format, read sequentially by the driver:
//! ```text
//! [pitch_hi, pitch_lo, ticks]  — play a note for `ticks` timer-0 ticks
//! [$FE, ticks]                 — rest (key off) for `ticks` ticks
//! [$FF]                        — end of song: loop from the top
//! ```
//! Timer 0 runs at 64 Hz (divisor 125), so 16 ticks = 250 ms.

use crate::Apu;

/// Where the uploaded block lands in ARAM, and the driver entry point.
const LOAD_ADDR: u16 = 0x0200;
/// Song table location (driver code must fit below this).
const SONG_ADDR: u16 = 0x02C0;
/// DSP sample directory location (page-aligned; DIR register = page).
const DIR_ADDR: u16 = 0x0300;
/// BRR sample data location.
const BRR_ADDR: u16 = 0x0310;

/// A 16-sample loop played at 32 kHz completes 2000 cycles per second,
/// so DSP pitch 0x1000 (native rate) sounds at 2 kHz.
const BASE_FREQ: f64 = 2000.0;

/// Note frequencies (Hz) used by the melody.
const C5: f64 = 523.25;
const E5: f64 = 659.25;
const G5: f64 = 783.99;
const A5: f64 = 880.00;
const C6: f64 = 1046.50;
const D6: f64 = 1174.66;

/// The waiting melody: an original four-bar pentatonic loop at 64 Hz
/// ticks (16 ticks = 250 ms). `None` is a rest.
pub const MELODY: [(Option<f64>, u8); 16] = [
    (Some(C5), 16), (Some(E5), 16), (Some(G5), 16), (Some(A5), 16),
    (Some(G5), 16), (Some(E5), 16), (Some(G5), 24), (None,      8),
    (Some(A5), 16), (Some(C6), 16), (Some(D6), 16), (Some(C6), 16),
    (Some(G5), 24), (Some(E5), 16), (Some(C5), 16), (None,      8),
];

// -------------------------------------------------------------------------
// Mini SPC700 assembler — just enough for the driver below.
// -------------------------------------------------------------------------

struct Asm {
    org:  u16,
    code: Vec<u8>,
}

impl Asm {
    fn new(org: u16) -> Self {
        Self { org, code: Vec::new() }
    }

    /// Address of the next byte to be emitted.
    fn here(&self) -> u16 {
        self.org + self.code.len() as u16
    }

    fn db(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
    }

    /// MOV dp, #imm — note the operand order: opcode, imm, dp.
    fn mov_dp_imm(&mut self, dp: u8, imm: u8) { self.db(&[0x8F, imm, dp]); }

    /// Write a DSP register through the $F2/$F3 address latch.
    fn dsp_write(&mut self, reg: u8, val: u8) {
        self.mov_dp_imm(0xF2, reg);
        self.mov_dp_imm(0xF3, val);
    }

    fn mov_x_imm(&mut self, imm: u8)     { self.db(&[0xCD, imm]); }
    fn mov_a_abs_x(&mut self, addr: u16) { self.db(&[0xF5, addr as u8, (addr >> 8) as u8]); }
    fn mov_a_dp(&mut self, dp: u8)       { self.db(&[0xE4, dp]); }
    fn mov_dp_a(&mut self, dp: u8)       { self.db(&[0xC4, dp]); }
    fn mov_y_a(&mut self)                { self.db(&[0xFD]); }
    fn inc_x(&mut self)                  { self.db(&[0x3D]); }
    fn dec_y(&mut self)                  { self.db(&[0xDC]); }
    fn cmp_a_imm(&mut self, imm: u8)     { self.db(&[0x68, imm]); }

    /// Emit a branch to an already-known (backward) target.
    fn branch_to(&mut self, opcode: u8, target: u16) {
        let rel = target as i32 - (self.here() as i32 + 2);
        assert!((-128..=127).contains(&rel), "branch out of range: {rel}");
        self.db(&[opcode, rel as i8 as u8]);
    }

    fn beq_to(&mut self, target: u16) { self.branch_to(0xF0, target); }
    fn bne_to(&mut self, target: u16) { self.branch_to(0xD0, target); }
    fn bra_to(&mut self, target: u16) { self.branch_to(0x2F, target); }

    /// Emit a BEQ with a forward target to be patched; returns the
    /// position of the displacement byte.
    fn beq_placeholder(&mut self) -> usize {
        self.db(&[0xF0, 0x00]);
        self.code.len() - 1
    }

    fn patch(&mut self, pos: usize, target: u16) {
        let rel = target as i32 - (self.org as i32 + pos as i32 + 1);
        assert!((-128..=127).contains(&rel), "patched branch out of range: {rel}");
        self.code[pos] = rel as i8 as u8;
    }
}

// -------------------------------------------------------------------------
// Content builders
// -------------------------------------------------------------------------

/// Assemble the sound driver (see the module docs for the song format).
///
/// The builders and `pitch_of`/`MELODY` are `pub` so the integration
/// tests can golden-check the emitted bytes from outside the crate.
pub fn build_driver() -> Vec<u8> {
    let mut a = Asm::new(LOAD_ADDR);

    // --- one-time DSP / timer setup ---
    a.dsp_write(0x6C, 0x20);              // FLG: no reset, unmuted, echo writes off
    a.dsp_write(0x0C, 0x7F);              // MVOLL
    a.dsp_write(0x1C, 0x7F);              // MVOLR
    a.dsp_write(0x5D, (DIR_ADDR >> 8) as u8); // DIR page
    a.dsp_write(0x00, 0x48);              // V0 VOLL
    a.dsp_write(0x01, 0x48);              // V0 VOLR
    a.dsp_write(0x04, 0x00);              // V0 SRCN = sample 0
    a.dsp_write(0x05, 0xEF);              // V0 ADSR1: enable, decay 6, attack 15
    a.dsp_write(0x06, 0xB4);              // V0 ADSR2: sustain level 5, rate 20
    a.mov_dp_imm(0xFA, 125);              // timer 0 divisor: 8000 / 125 = 64 Hz
    a.mov_dp_imm(0xF1, 0x01);             // CONTROL: enable timer 0

    // --- song sequencer ---
    let song_top = a.here();
    a.mov_x_imm(0);                       // X = song table index

    let next_note = a.here();
    a.mov_a_abs_x(SONG_ADDR);             // byte 0: pitch hi / $FF end / $FE rest
    a.cmp_a_imm(0xFF);
    a.beq_to(song_top);
    a.cmp_a_imm(0xFE);
    let to_rest = a.beq_placeholder();

    a.mov_dp_imm(0xF2, 0x03);             // PITCHH = A
    a.mov_dp_a(0xF3);
    a.inc_x();
    a.mov_a_abs_x(SONG_ADDR);             // byte 1: pitch lo
    a.mov_dp_imm(0xF2, 0x02);             // PITCHL = A
    a.mov_dp_a(0xF3);
    a.inc_x();
    a.mov_a_abs_x(SONG_ADDR);             // byte 2: duration in timer ticks
    a.inc_x();
    a.mov_y_a();                          // Y = remaining ticks
    a.dsp_write(0x4C, 0x01);              // KON voice 0

    // Wait Y timer-0 ticks. $FD is clear-on-read, so each read returns the
    // ticks elapsed since the last read (0 = none yet).
    let wait_tick = a.here();
    a.mov_a_dp(0xFD);
    a.beq_to(wait_tick);
    a.dec_y();
    a.bne_to(wait_tick);
    a.dsp_write(0x5C, 0x01);              // KOFF voice 0 -> release fade
    a.bra_to(next_note);

    // Rest: skip the $FE marker, load the duration, and reuse the wait
    // loop. Its trailing KOFF is redundant during a rest but harmless.
    let rest = a.here();
    a.patch(to_rest, rest);
    a.inc_x();
    a.mov_a_abs_x(SONG_ADDR);             // duration
    a.inc_x();
    a.mov_y_a();
    a.bra_to(wait_tick);

    a.code
}

/// DSP pitch value for a frequency, given the 2 kHz base tone.
pub fn pitch_of(freq: f64) -> u16 {
    let p = (freq * 4096.0 / BASE_FREQ).round() as u16;
    assert!(p > 0 && p <= 0x3FFF, "pitch out of DSP range: {p:#06x}");
    p
}

/// Encode the melody into the driver's song table format.
pub fn build_song() -> Vec<u8> {
    let mut out = Vec::new();
    for &(freq, ticks) in MELODY.iter() {
        match freq {
            None => out.extend_from_slice(&[0xFE, ticks]),
            Some(f) => {
                let p = pitch_of(f);
                let hi = (p >> 8) as u8;
                assert!(hi < 0xFE, "pitch hi collides with song markers");
                out.extend_from_slice(&[hi, p as u8, ticks]);
            }
        }
    }
    out.push(0xFF);
    out
}

/// One looped BRR block: a 16-sample warm tone (fundamental plus a bit of
/// second harmonic), 4-bit quantized, shift 11, filter 0, loop+end flags.
/// With filter 0 there is no cross-block prediction history, so a
/// single-block loop is seamless by construction.
pub fn build_brr() -> Vec<u8> {
    let mut nibbles = [0u8; 16];
    for (i, n) in nibbles.iter_mut().enumerate() {
        let t = i as f64 / 16.0 * std::f64::consts::TAU;
        let v = 0.78 * t.sin() + 0.22 * (2.0 * t).sin();
        let q = (v * 7.0).round().clamp(-8.0, 7.0) as i8;
        *n = (q as u8) & 0x0F;
    }

    let header = (11 << 4) | 0x02 | 0x01; // shift 11, filter 0, loop + end
    let mut block = vec![header];
    for pair in nibbles.chunks_exact(2) {
        block.push((pair[0] << 4) | pair[1]);
    }
    block
}

/// Build the full contiguous ARAM image, from LOAD_ADDR through the end
/// of the BRR data, with the driver, song, DIR table, and sample placed
/// at their fixed addresses.
pub fn build_aram_image() -> Vec<u8> {
    let driver = build_driver();
    let song = build_song();
    let brr = build_brr();

    assert!(
        LOAD_ADDR + (driver.len() as u16) <= SONG_ADDR,
        "driver overflows into the song table: {} bytes",
        driver.len()
    );
    assert!(
        SONG_ADDR + (song.len() as u16) <= DIR_ADDR,
        "song overflows into the DIR table: {} bytes",
        song.len()
    );

    let size = (BRR_ADDR - LOAD_ADDR) as usize + brr.len();
    let mut img = vec![0u8; size];

    img[..driver.len()].copy_from_slice(&driver);

    let s = (SONG_ADDR - LOAD_ADDR) as usize;
    img[s..s + song.len()].copy_from_slice(&song);

    // DIR entry 0: [start_lo, start_hi, loop_lo, loop_hi] — both at BRR_ADDR.
    let d = (DIR_ADDR - LOAD_ADDR) as usize;
    let [brr_lo, brr_hi] = BRR_ADDR.to_le_bytes();
    img[d..d + 4].copy_from_slice(&[brr_lo, brr_hi, brr_lo, brr_hi]);

    let b = (BRR_ADDR - LOAD_ADDR) as usize;
    img[b..b + brr.len()].copy_from_slice(&brr);

    img
}

// -------------------------------------------------------------------------
// Host-side "fake main CPU": speak the IPL boot protocol
// -------------------------------------------------------------------------

/// Cycle budget for each protocol wait before declaring the boot wedged.
/// The whole upload normally completes in a few thousand cycles; a full
/// second of APU time means something is structurally broken.
const PROTOCOL_TIMEOUT_CYCLES: u32 = 1_024_000;

/// Step the APU until port 0 shows `want`, or time out.
fn wait_port0(apu: &mut Apu, want: u8, what: &str) -> Result<(), String> {
    for _ in 0..PROTOCOL_TIMEOUT_CYCLES {
        if apu.memory.cpu_port_read(0) == want {
            return Ok(());
        }
        apu.step(1);
    }
    Err(format!("IPL timeout waiting for {want:#04x} on port 0 ({what})"))
}

/// Upload `image` to `load_addr` over the IPL boot protocol and start
/// execution at `entry` — exactly what a game's boot code does through
/// $2140-$2143, performed here directly on the APU's port API.
fn upload_and_execute(
    apu: &mut Apu,
    load_addr: u16,
    image: &[u8],
    entry: u16,
) -> Result<(), String> {
    // 1. Wait for the boot announce.
    for _ in 0..PROTOCOL_TIMEOUT_CYCLES {
        if apu.memory.cpu_port_read(0) == 0xAA && apu.memory.cpu_port_read(1) == 0xBB {
            break;
        }
        apu.step(1);
    }
    if apu.memory.cpu_port_read(0) != 0xAA {
        return Err("IPL timeout waiting for the $AA/$BB announce".into());
    }

    // 2. Start command: target address, non-zero port 1 = transfer, $CC.
    let [lo, hi] = load_addr.to_le_bytes();
    apu.memory.cpu_port_write(2, lo);
    apu.memory.cpu_port_write(3, hi);
    apu.memory.cpu_port_write(1, 0x01);
    apu.memory.cpu_port_write(0, 0xCC);
    wait_port0(apu, 0xCC, "start-command ack")?;

    // 3. Byte transfer: data on port 1, then the byte index on port 0,
    // then wait for the index echo. The index is a wrapping u8; the IPL's
    // signed-delta comparison handles blocks longer than 256 bytes.
    let mut index: u8 = 0;
    for &byte in image {
        apu.memory.cpu_port_write(1, byte);
        apu.memory.cpu_port_write(0, index);
        wait_port0(apu, index, "byte-index echo")?;
        index = index.wrapping_add(1);
    }

    // 4. Execute command: index bumped by >= 2, port 1 = 0, entry address.
    let cmd = index.wrapping_add(2);
    let [lo, hi] = entry.to_le_bytes();
    apu.memory.cpu_port_write(2, lo);
    apu.memory.cpu_port_write(3, hi);
    apu.memory.cpu_port_write(1, 0x00);
    apu.memory.cpu_port_write(0, cmd);
    wait_port0(apu, cmd, "execute ack")?;

    // Ride out the HLE's exec-delay window so the driver is actually
    // running (and its DSP setup done) by the time we return.
    apu.step(1024);
    Ok(())
}

// -------------------------------------------------------------------------
// Public interface
// -------------------------------------------------------------------------

/// A standalone APU playing the waiting music, ready to generate audio.
pub struct IdleJingle {
    apu: Apu,
}

impl IdleJingle {
    /// Boot a fresh APU and upload the jingle driver through the full IPL
    /// protocol. Returns an error (instead of hanging or panicking) if the
    /// boot handshake wedges, so the frontend can just disable the music.
    pub fn new() -> Result<Self, String> {
        let mut apu = Apu::new();
        let image = build_aram_image();
        upload_and_execute(&mut apu, LOAD_ADDR, &image, LOAD_ADDR)?;
        // The boot and upload themselves stepped the APU for a few
        // thousand cycles, leaving ~70 pairs of pre-music silence in the
        // sample buffer. Discard them: playback then starts at t=0 of
        // the song, and `generate(n)` returns exactly n frames instead
        // of n + residue (which could also trip the buffer's 1-second
        // overflow cap mid-call and lose everything but the residue).
        apu.drain_samples();
        Ok(Self { apu })
    }

    /// The APU running the jingle, for inspection (tests, debugging).
    pub fn apu(&self) -> &Apu {
        &self.apu
    }

    /// Run the APU for `frames` output samples (32 kHz) and return them
    /// interleaved (L, R, L, R, ...), ready for the SDL audio queue.
    ///
    /// Keep single calls at or below one second (32,000 frames): the
    /// APU's internal buffer discards everything when it exceeds one
    /// second without a drain, and this method only drains at the end.
    pub fn generate(&mut self, frames: usize) -> Vec<i16> {
        // 32 CPU cycles per DSP output sample.
        self.apu.step(frames as u32 * 32);
        self.apu.drain_samples()
    }
}
