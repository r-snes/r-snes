//! The "no ROM loaded" waiting music, v2: a three-voice tracker.
//!
//! Nothing here bypasses the emulator: the music is a real SPC700 sound
//! driver (hand-assembled below, byte-verified against a dual-implementation
//! reference simulation — see the golden tests) plus three BRR-encoded
//! instruments, uploaded into a freshly booted `Apu` over the IPL boot
//! protocol. Compared to v1, the driver now exercises the APU the way a
//! game driver does:
//!
//! - **3 simultaneous voices**: lead melody, bass, and arpeggio, each with
//!   its own independent track, instrument, and envelope.
//! - **Vibrato** on the lead: the driver rewrites the voice 0 pitch
//!   registers on every 64 Hz tick from a 16-step table (~4 Hz wobble).
//! - **Multi-block BRR with prediction filters**: instruments are encoded
//!   from waveform tables by a real BRR encoder that picks the best
//!   shift/filter per block (filters 1-3 get used; the loop-start block is
//!   forced to filter 0 so the loop needs no history).
//! - **GAIN mode**: the arpeggio voice runs on direct gain instead of ADSR.
//!
//! The composition is an original chiptune-style piece (A minor, 120 BPM,
//! 8 bars, 16-second loop; all three tracks are exactly 1024 ticks).
//!
//! ARAM layout (uploaded as one contiguous block):
//! ```text
//! $0200  driver code (entry point)
//! $03E0  vibrato offset table (16 bytes)
//! $0400  track, voice 0 (lead)
//! $0500  track, voice 1 (bass)
//! $0600  track, voice 2 (arpeggio)
//! $0700  sample directory (DIR): 3 entries
//! $0710  BRR instruments (lead, bass, arp)
//! ```
//!
//! Track format, read independently per voice:
//! ```text
//! [pitch_hi, pitch_lo, ticks]  — key the note on for `ticks` 64 Hz ticks
//! [$FE, ticks]                 — rest (key off)
//! [$FF]                        — end of track: loop from the top
//! ```

use crate::Apu;

/// Where the uploaded block lands in ARAM, and the driver entry point.
const LOAD_ADDR: u16 = 0x0200;
/// Vibrato offset table (16 bytes; driver code must fit below this).
const VIB_ADDR: u16 = 0x03E0;
/// Per-voice track tables (256 bytes max each).
const TRACK_ADDR: [u16; 3] = [0x0400, 0x0500, 0x0600];
/// DSP sample directory (page-aligned; DIR register = page).
const DIR_ADDR: u16 = 0x0700;
/// First BRR instrument; the rest follow contiguously.
const BRR_ADDR: u16 = DIR_ADDR + 0x10;

/// Sentinel pitch marking a rest in the track tables below.
pub const REST: u16 = 0xFFFF;

/// KON/KOFF bit per voice.
const KBIT: [u8; 3] = [0x01, 0x02, 0x04];

/// Vibrato pitch offsets, indexed by a 16-step phase advancing once per
/// 64 Hz tick (~4 Hz cycle). Offsets are unsigned (0..=24) around a +12
/// center; the resulting constant detune of about a tenth of a semitone
/// at the lead's pitch range is inaudible, and keeping the offsets
/// unsigned keeps the driver's 16-bit add carry-only.
const VIBTAB: [u8; 16] = [12, 17, 21, 23, 24, 23, 21, 17, 12, 7, 3, 1, 0, 1, 3, 7];

// -------------------------------------------------------------------------
// Instruments: waveform tables (integer, so builds are deterministic) that
// the BRR encoder below turns into multi-block filtered BRR at build time.
// Lead/bass are 64-sample loops (500 Hz at pitch $1000), arp is 32 samples
// (1 kHz at pitch $1000).
// -------------------------------------------------------------------------

/// Bright saw-ish lead: harmonics 1-5 at 1/n.
const WAVE_LEAD: [i16; 64] = [
    0, 3976, 7542, 10354, 12194, 13000, 12864, 12008,
    10730, 9346, 8126, 7251, 6788, 6694, 6838, 7045,
    7146, 7014, 6597, 5922, 5081, 4204, 3423, 2837,
    2485, 2344, 2332, 2336, 2242, 1964, 1471, 790,
    0, -790, -1471, -1964, -2242, -2336, -2332, -2344,
    -2485, -2837, -3423, -4204, -5081, -5922, -6597, -7014,
    -7146, -7045, -6838, -6694, -6788, -7251, -8126, -9346,
    -10730, -12008, -12864, -13000, -12194, -10354, -7542, -3976,
];

/// Warm triangle bass: odd harmonics at 1/n^2 with alternating sign.
const WAVE_BASS: [i16; 64] = [
    0, 956, 1882, 2758, 3580, 4361, 5132, 5927,
    6779, 7706, 8702, 9734, 10741, 11647, 12371, 12838,
    13000, 12838, 12371, 11647, 10741, 9734, 8702, 7706,
    6779, 5927, 5132, 4361, 3580, 2758, 1882, 956,
    0, -956, -1882, -2758, -3580, -4361, -5132, -5927,
    -6779, -7706, -8702, -9734, -10741, -11647, -12371, -12838,
    -13000, -12838, -12371, -11647, -10741, -9734, -8702, -7706,
    -6779, -5927, -5132, -4361, -3580, -2758, -1882, -956,
];

/// Hollow square-ish arpeggio: odd harmonics 1,3,5,7 at 1/n.
const WAVE_ARP: [i16; 32] = [
    0, 8860, 12000, 10357, 9036, 10060, 10989, 10152,
    9339, 10152, 10989, 10060, 9036, 10357, 12000, 8860,
    0, -8860, -12000, -10357, -9036, -10060, -10989, -10152,
    -9339, -10152, -10989, -10060, -9036, -10357, -12000, -8860,
];

// -------------------------------------------------------------------------
// The music. Pitches are precomputed DSP pitch values (comments give the
// note names); ticks are 64 Hz timer ticks, so an eighth note at 120 BPM
// is 16 ticks. All three tracks last exactly 1024 ticks (16 s) per loop.
// -------------------------------------------------------------------------

pub const TRACK_LEAD: &[(u16, u8)] = &[
    (0x1519, 32), // E5
    (0x1c29, 16), // A5
    (0x1916, 16), // G5
    (0x1519, 32), // E5
    (0x10be, 32), // C5
    (0x12cb, 16), // D5
    (0x10be, 16), // C5
    (0x0e14, 32), // A4
    (0x10be, 32), // C5
    (REST, 32),
    (0x1519, 32), // E5
    (0x1916, 16), // G5
    (0x1519, 16), // E5
    (0x10be, 64), // C5
    (0x12cb, 16), // D5
    (0x1519, 16), // E5
    (0x12cb, 16), // D5
    (0x0fce, 16), // B4
    (0x0c8b, 64), // G4
    (0x1c29, 48), // A5
    (0x1916, 16), // G5
    (0x1519, 32), // E5
    (0x10be, 32), // C5
    (0x165a, 32), // F5
    (0x1519, 16), // E5
    (0x12cb, 16), // D5
    (0x10be, 32), // C5
    (0x0e14, 32), // A4
    (0x12cb, 32), // D5
    (0x165a, 32), // F5
    (0x1519, 48), // E5
    (0x0fce, 16), // B4
    (0x0e14, 112), // A4
    (REST, 16),
];

pub const TRACK_BASS: &[(u16, u8)] = &[
    (0x0385, 32), // A2
    (0x0385, 32), // A2
    (0x0546, 64), // E3
    (0x02cb, 32), // F2
    (0x02cb, 32), // F2
    (0x0430, 64), // C3
    (0x0430, 32), // C3
    (0x0430, 32), // C3
    (0x0323, 64), // G2
    (0x0323, 32), // G2
    (0x0323, 32), // G2
    (0x04b3, 64), // D3
    (0x0385, 32), // A2
    (0x0385, 32), // A2
    (0x0546, 64), // E3
    (0x02cb, 32), // F2
    (0x02cb, 32), // F2
    (0x0430, 64), // C3
    (0x04b3, 64), // D3
    (0x02a3, 64), // E2
    (0x0385, 128), // A2
];

pub const TRACK_ARP: &[(u16, u8)] = &[
    (0x0385, 16), // A3
    (0x0546, 16), // E4
    (0x070a, 16), // A4
    (0x0546, 16), // E4
    (0x0385, 16), // A3
    (0x0546, 16), // E4
    (0x070a, 16), // A4
    (0x0546, 16), // E4
    (0x02cb, 16), // F3
    (0x0430, 16), // C4
    (0x0596, 16), // F4
    (0x0430, 16), // C4
    (0x02cb, 16), // F3
    (0x0430, 16), // C4
    (0x0596, 16), // F4
    (0x0430, 16), // C4
    (0x0430, 16), // C4
    (0x0646, 16), // G4
    (0x085f, 16), // C5
    (0x0646, 16), // G4
    (0x0430, 16), // C4
    (0x0646, 16), // G4
    (0x085f, 16), // C5
    (0x0646, 16), // G4
    (0x0323, 16), // G3
    (0x04b3, 16), // D4
    (0x0646, 16), // G4
    (0x04b3, 16), // D4
    (0x0323, 16), // G3
    (0x04b3, 16), // D4
    (0x0646, 16), // G4
    (0x04b3, 16), // D4
    (0x0385, 16), // A3
    (0x0546, 16), // E4
    (0x070a, 16), // A4
    (0x0546, 16), // E4
    (0x0385, 16), // A3
    (0x0546, 16), // E4
    (0x070a, 16), // A4
    (0x0546, 16), // E4
    (0x02cb, 16), // F3
    (0x0430, 16), // C4
    (0x0596, 16), // F4
    (0x0430, 16), // C4
    (0x02cb, 16), // F3
    (0x0430, 16), // C4
    (0x0596, 16), // F4
    (0x0430, 16), // C4
    (0x04b3, 16), // D4
    (0x070a, 16), // A4
    (0x0966, 16), // D5
    (0x070a, 16), // A4
    (0x0546, 16), // E4
    (0x07e7, 16), // B4
    (0x0a8c, 16), // E5
    (0x07e7, 16), // B4
    (0x0385, 16), // A3
    (0x0546, 16), // E4
    (0x070a, 16), // A4
    (0x0546, 16), // E4
    (0x0385, 32), // A3
    (REST, 32),
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

    fn mov_a_abs_x(&mut self, addr: u16) { self.db(&[0xF5, addr as u8, (addr >> 8) as u8]); }
    fn mov_a_dp(&mut self, dp: u8)       { self.db(&[0xE4, dp]); }
    fn mov_dp_a(&mut self, dp: u8)       { self.db(&[0xC4, dp]); }
    fn mov_x_dp(&mut self, dp: u8)       { self.db(&[0xF8, dp]); }
    fn mov_dp_x(&mut self, dp: u8)       { self.db(&[0xD8, dp]); }
    fn mov_x_a(&mut self)                { self.db(&[0x5D]); }
    fn inc_x(&mut self)                  { self.db(&[0x3D]); }
    fn dec_dp(&mut self, dp: u8)         { self.db(&[0x8B, dp]); }
    fn cmp_a_imm(&mut self, imm: u8)     { self.db(&[0x68, imm]); }
    fn adc_a_imm(&mut self, imm: u8)     { self.db(&[0x88, imm]); }
    fn adc_a_dp(&mut self, dp: u8)       { self.db(&[0x84, dp]); }
    fn and_a_imm(&mut self, imm: u8)     { self.db(&[0x28, imm]); }
    fn clrc(&mut self)                   { self.db(&[0x60]); }
    fn jmp_abs(&mut self, addr: u16)     { self.db(&[0x5F, addr as u8, (addr >> 8) as u8]); }

    /// Emit a branch to an already-known (backward) target.
    fn branch_to(&mut self, opcode: u8, target: u16) {
        let rel = target as i32 - (self.here() as i32 + 2);
        assert!((-128..=127).contains(&rel), "branch out of range: {rel}");
        self.db(&[opcode, rel as i8 as u8]);
    }

    fn beq_to(&mut self, target: u16) { self.branch_to(0xF0, target); }

    /// Emit a branch with a forward target to be patched; returns the
    /// position of the displacement byte.
    fn placeholder(&mut self, opcode: u8) -> usize {
        self.db(&[opcode, 0x00]);
        self.code.len() - 1
    }

    fn patch(&mut self, pos: usize, target: u16) {
        let rel = target as i32 - (self.org as i32 + pos as i32 + 1);
        assert!((-128..=127).contains(&rel), "patched branch out of range: {rel}");
        self.code[pos] = rel as i8 as u8;
    }
}

// -------------------------------------------------------------------------
// Driver
// -------------------------------------------------------------------------

/// Direct-page variables: per-voice track index and remaining ticks,
/// vibrato phase, voice 0 base pitch, and a scratch byte.
const IDX: [u8; 3] = [0x10, 0x12, 0x14];
const TCK: [u8; 3] = [0x11, 0x13, 0x15];
const VPH: u8 = 0x16;
const BLO: u8 = 0x17;
const BHI: u8 = 0x18;
const VOFF: u8 = 0x19;

/// Assemble the sound driver.
///
/// Everything below `build_driver` is `pub` so the integration tests can
/// golden-check the emitted bytes from outside the crate. The golden
/// arrays were generated by (and byte-verified against) a reference
/// SPC700 simulation, so a golden mismatch means "re-verify the driver",
/// not "update the constant".
pub fn build_driver() -> Vec<u8> {
    let mut a = Asm::new(LOAD_ADDR);

    // --- one-time DSP / timer / variable setup ---
    a.dsp_write(0x6C, 0x20);                    // FLG: unmuted, echo writes off
    a.dsp_write(0x0C, 0x60);                    // MVOLL (3 voices: headroom)
    a.dsp_write(0x1C, 0x60);                    // MVOLR
    a.dsp_write(0x5D, (DIR_ADDR >> 8) as u8);   // DIR page
    let vols: [u8; 3] = [0x38, 0x46, 0x24];
    for v in 0..3u8 {
        a.dsp_write((v << 4) | 0x0, vols[v as usize]); // VOLL
        a.dsp_write((v << 4) | 0x1, vols[v as usize]); // VOLR
        a.dsp_write((v << 4) | 0x4, v);                // SRCN = instrument v
    }
    a.dsp_write(0x05, 0xAD);                    // V0 ADSR1: attack 13, decay 2
    a.dsp_write(0x06, 0xAE);                    // V0 ADSR2: sustain 5, rate 14
    a.dsp_write(0x15, 0xBF);                    // V1 ADSR1: attack 15, decay 3
    a.dsp_write(0x16, 0x72);                    // V1 ADSR2: sustain 3, rate 18
    a.dsp_write(0x25, 0x00);                    // V2 ADSR1 bit7 clear: GAIN mode
    a.dsp_write(0x27, 0x48);                    // V2 GAIN: direct, level 0x480
    for v in 0..3 {
        a.mov_dp_imm(IDX[v], 0);                // track index = 0
        a.mov_dp_imm(TCK[v], 1);                // 1 tick left -> fetch on tick 1
    }
    a.mov_dp_imm(VPH, 0);
    a.mov_dp_imm(BLO, 0);
    a.mov_dp_imm(BHI, 0);
    a.mov_dp_imm(0xFA, 125);                    // timer 0: 8000 / 125 = 64 Hz
    a.mov_dp_imm(0xF1, 0x01);                   // CONTROL: enable timer 0

    // --- main tick loop ---
    let wait = a.here();
    a.mov_a_dp(0xFD);                           // $FD is clear-on-read
    a.beq_to(wait);

    // Vibrato: voice 0 pitch = base + VIBTAB[phase], every tick.
    a.mov_a_dp(VPH); a.clrc(); a.adc_a_imm(1); a.and_a_imm(0x0F); a.mov_dp_a(VPH);
    a.mov_x_a(); a.mov_a_abs_x(VIB_ADDR); a.mov_dp_a(VOFF);
    a.mov_a_dp(BLO); a.clrc(); a.adc_a_dp(VOFF);
    a.mov_dp_imm(0xF2, 0x02); a.mov_dp_a(0xF3); // PITCHL (MOVs preserve carry)
    a.mov_a_dp(BHI); a.adc_a_imm(0);
    a.mov_dp_imm(0xF2, 0x03); a.mov_dp_a(0xF3); // PITCHH

    // Per-voice sequencers, unrolled.
    for v in 0..3usize {
        a.dec_dp(TCK[v]);
        let to_skip = a.placeholder(0xD0);      // bne skip: note still sounding
        let fetch = a.here();
        a.mov_x_dp(IDX[v]);
        a.mov_a_abs_x(TRACK_ADDR[v]);
        a.cmp_a_imm(0xFF);
        let to_notloop = a.placeholder(0xD0);
        a.mov_dp_imm(IDX[v], 0);                // $FF: loop track from the top
        a.branch_to(0x2F, fetch);
        let notloop = a.here();
        a.patch(to_notloop, notloop);
        a.cmp_a_imm(0xFE);
        let to_rest = a.placeholder(0xF0);
        // Note event: A = pitch hi. KON alone retriggers (it resets the
        // envelope), so no KOFF is needed on a note change.
        if v == 0 { a.mov_dp_a(BHI); }
        a.mov_dp_imm(0xF2, ((v as u8) << 4) | 0x03); a.mov_dp_a(0xF3);
        a.inc_x(); a.mov_a_abs_x(TRACK_ADDR[v]);
        if v == 0 { a.mov_dp_a(BLO); }
        a.mov_dp_imm(0xF2, ((v as u8) << 4) | 0x02); a.mov_dp_a(0xF3);
        a.inc_x(); a.mov_a_abs_x(TRACK_ADDR[v]);
        a.mov_dp_a(TCK[v]);
        a.inc_x(); a.mov_dp_x(IDX[v]);
        a.dsp_write(0x4C, KBIT[v]);             // KON
        let to_skip2 = a.placeholder(0x2F);
        // Rest event: key off, load duration.
        let rest = a.here();
        a.patch(to_rest, rest);
        a.dsp_write(0x5C, KBIT[v]);             // KOFF -> release fade
        a.inc_x(); a.mov_a_abs_x(TRACK_ADDR[v]);
        a.mov_dp_a(TCK[v]);
        a.inc_x(); a.mov_dp_x(IDX[v]);
        let skip = a.here();
        a.patch(to_skip, skip);
        a.patch(to_skip2, skip);
    }

    a.jmp_abs(wait);
    a.code
}

// -------------------------------------------------------------------------
// BRR encoder: turn a waveform table into multi-block BRR, choosing the
// best shift and prediction filter per block by exhaustive search
// (integer arithmetic only, so output is deterministic).
// -------------------------------------------------------------------------

/// The DSP's prediction filters — must match `dsp::brr::decode_brr_nibble`.
fn brr_predict(filter: u8, p1: i32, p2: i32) -> i32 {
    match filter {
        0 => 0,
        1 => p1 - (p1 >> 4),
        2 => (p1 * 2) - ((p1 * 3) >> 5) - p2 + (p2 >> 4),
        3 => (p1 * 2) - ((p1 * 13) >> 6) - p2 + ((p2 * 3) >> 4),
        _ => unreachable!(),
    }
}

/// Round-to-nearest division by 2^s (arithmetic shift; ties toward +inf).
fn div_round(d: i32, s: u8) -> i32 {
    if s == 0 { d } else { (d + (1 << (s - 1))) >> s }
}

/// Encode one waveform (length a multiple of 16) into looped BRR.
/// The first block — which is also the loop target — is forced to filter 0
/// so the loop is seamless without prediction history; later blocks pick
/// whichever filter/shift reconstructs the samples best.
pub fn encode_brr(samples: &[i16]) -> Vec<u8> {
    assert!(samples.len() % 16 == 0 && !samples.is_empty());
    let blocks = samples.len() / 16;
    let mut out = Vec::with_capacity(blocks * 9);
    let (mut p1, mut p2) = (0i32, 0i32);

    for b in 0..blocks {
        let chunk = &samples[b * 16..(b + 1) * 16];
        let filters: &[u8] = if b == 0 { &[0] } else { &[0, 1, 2, 3] };

        let mut best: Option<(i64, u8, u8, [u8; 16], i32, i32)> = None;
        for &f in filters {
            for shift in 0..=12u8 {
                let mut err = 0i64;
                let (mut q1, mut q2) = (p1, p2);
                let mut nib = [0u8; 16];
                for (i, &s) in chunk.iter().enumerate() {
                    let pred = brr_predict(f, q1, q2);
                    let n = div_round(s as i32 - pred, shift).clamp(-8, 7);
                    let dec = ((n << shift) + pred).clamp(-0x4000, 0x3FFF);
                    let d = (dec - s as i32) as i64;
                    err += d * d;
                    nib[i] = (n as u8) & 0x0F;
                    q2 = q1;
                    q1 = dec;
                }
                if best.as_ref().is_none_or(|(e, ..)| err < *e) {
                    best = Some((err, f, shift, nib, q1, q2));
                }
            }
        }

        let (_, f, shift, nib, q1, q2) = best.unwrap();
        (p1, p2) = (q1, q2);
        let flags = if b == blocks - 1 { 0x03 } else { 0x00 }; // loop+end on last
        out.push((shift << 4) | (f << 2) | flags);
        for i in 0..8 {
            out.push((nib[2 * i] << 4) | nib[2 * i + 1]);
        }
    }
    out
}

/// Encode a track table into the driver's byte format.
pub fn build_track(track: &[(u16, u8)]) -> Vec<u8> {
    let mut out = Vec::new();
    for &(p, ticks) in track {
        assert!(ticks > 0);
        if p == REST {
            out.extend_from_slice(&[0xFE, ticks]);
        } else {
            assert!(p <= 0x3FFF, "pitch out of DSP range");
            out.extend_from_slice(&[(p >> 8) as u8, p as u8, ticks]);
        }
    }
    out.push(0xFF);
    assert!(out.len() <= 256, "track too long for 8-bit indexing: {}", out.len());
    out
}

/// Build the full contiguous ARAM image: driver, vibrato table, the three
/// tracks, the DIR table, and the three BRR instruments.
pub fn build_aram_image() -> Vec<u8> {
    let driver = build_driver();
    assert!(
        LOAD_ADDR + (driver.len() as u16) <= VIB_ADDR,
        "driver overflows into the vibrato table: {} bytes",
        driver.len()
    );

    let tracks = [build_track(TRACK_LEAD), build_track(TRACK_BASS), build_track(TRACK_ARP)];
    let instruments = [
        encode_brr(&WAVE_LEAD),
        encode_brr(&WAVE_BASS),
        encode_brr(&WAVE_ARP),
    ];

    let brr_total: usize = instruments.iter().map(Vec::len).sum();
    let size = (BRR_ADDR - LOAD_ADDR) as usize + brr_total;
    let mut img = vec![0u8; size];
    let off = |addr: u16| (addr - LOAD_ADDR) as usize;

    img[..driver.len()].copy_from_slice(&driver);
    img[off(VIB_ADDR)..off(VIB_ADDR) + 16].copy_from_slice(&VIBTAB);
    for v in 0..3 {
        img[off(TRACK_ADDR[v])..off(TRACK_ADDR[v]) + tracks[v].len()]
            .copy_from_slice(&tracks[v]);
    }

    // DIR: 3 entries of [start_lo, start_hi, loop_lo, loop_hi]; each
    // instrument loops over its whole sample, so start == loop.
    let mut brr_addr = BRR_ADDR;
    for (i, brr) in instruments.iter().enumerate() {
        let [lo, hi] = brr_addr.to_le_bytes();
        img[off(DIR_ADDR) + i * 4..off(DIR_ADDR) + i * 4 + 4]
            .copy_from_slice(&[lo, hi, lo, hi]);
        img[off(brr_addr)..off(brr_addr) + brr.len()].copy_from_slice(brr);
        brr_addr += brr.len() as u16;
    }

    img
}

// -------------------------------------------------------------------------
// Host-side "fake main CPU": speak the IPL boot protocol
// -------------------------------------------------------------------------

/// Cycle budget for each protocol wait before declaring the boot wedged.
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
        // The boot and upload themselves stepped the APU, leaving a bit of
        // pre-music silence in the sample buffer. Discard it: playback
        // then starts at t=0 of the song, and `generate(n)` returns
        // exactly n frames instead of n + residue (which could also trip
        // the buffer's 1-second overflow cap mid-call).
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
