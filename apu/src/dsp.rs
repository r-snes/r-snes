use crate::memory::Memory;

// ============================================================
// ENVELOPE RATE TABLE
// The real DSP uses a 32-entry lookup table to determine how
// many ticks pass between each envelope step. Index 0 means
// "never" (infinite hold). All other values are tick counts.
// ============================================================

/// Ticks between envelope updates for each rate index (0–31).
/// Rate 0 = never update (infinite). Rate 31 = update every tick.
const ENVELOPE_RATE_TABLE: [u16; 32] = [
    0,    // 0: never (infinite)
    2048, 1536, 1280, 1024, 768,
    640,  512,  384,  320,  256,
    192,  160,  128,  96,   80,
    64,   48,   40,   32,   24,
    20,   16,   12,   10,   8,
    6,    5,    4,    3,    2,
    1,    // 31: every tick
];

// ============================================================
// ADSR ENVELOPE
// Controls how loud a voice is over time using a 4-phase model.
// Envelope level is 11 bits wide (0x000–0x7FF).
// ============================================================

/// Current phase of the ADSR envelope state machine.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EnvelopePhase {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}

/// ADSR envelope for one voice.
#[derive(Debug, Clone, Copy)]
pub struct Adsr {
    /// true = ADSR mode, false = GAIN mode (GAIN not yet implemented)
    pub adsr_mode: bool,

    /// Attack rate index (0–15). Maps into the rate table.
    pub attack_rate: u8,

    /// Decay rate index (0–7). Maps into rate table as (rate*2 + 16).
    pub decay_rate: u8,

    /// Sustain level (0–7). Sustain target = (level + 1) * 0x100.
    pub sustain_level: u8,

    /// Sustain rate index (0–31). Direct index into rate table.
    pub sustain_rate: u8,

    /// Current 11-bit envelope volume (0x000–0x7FF).
    pub envelope_level: u16,

    /// Current phase of the envelope.
    pub envelope_phase: EnvelopePhase,

    /// Internal tick counter used to pace envelope updates.
    pub tick_counter: u16,
}

impl Default for Adsr {
    fn default() -> Self {
        Self {
            adsr_mode: false,
            attack_rate: 0,
            decay_rate: 0,
            sustain_level: 0,
            sustain_rate: 0,
            envelope_level: 0,
            envelope_phase: EnvelopePhase::Off,
            tick_counter: 0,
        }
    }
}

impl Adsr {
    /// Advance the envelope by one DSP tick (called once per output sample).
    ///
    /// The hardware only steps the envelope every N ticks, where N is
    /// determined by the rate table. Each phase has its own rate source.
    pub fn update_envelope(&mut self) {
        match self.envelope_phase {
            // -------------------------------------------------
            // ATTACK: rise linearly from 0 to 0x7FF.
            // Rate index comes from the 4-bit attack_rate field.
            // Special case: attack_rate == 15 uses a fixed +1024 step
            // for a near-instant attack.
            // -------------------------------------------------
            EnvelopePhase::Attack => {
                if self.attack_rate == 15 {
                    // Fast attack: fixed +1024 step, no rate gating
                    self.envelope_level = (self.envelope_level + 1024).min(0x7FF);
                } else {
                    // Normal attack: table-driven tick gating, +32 per step
                    let rate_idx = (self.attack_rate * 2 + 1) as usize;
                    let period = ENVELOPE_RATE_TABLE[rate_idx.min(31)];
                    if !self.tick_due(period) {
                        return;
                    }
                    self.envelope_level = (self.envelope_level + 32).min(0x7FF);
                }

                if self.envelope_level >= 0x7FF {
                    self.envelope_level = 0x7FF;
                    self.envelope_phase = EnvelopePhase::Decay;
                    self.tick_counter = 0;
                }
            }

            // -------------------------------------------------
            // DECAY: fall exponentially toward the sustain target.
            // Rate index = decay_rate * 2 + 16 (upper half of table).
            // Step = -(level >> 8) - 1  (exponential curve).
            // Transition when level reaches the sustain target.
            // -------------------------------------------------
            EnvelopePhase::Decay => {
                let rate_idx = (self.decay_rate * 2 + 16) as usize;
                let period = ENVELOPE_RATE_TABLE[rate_idx.min(31)];
                if !self.tick_due(period) {
                    return;
                }

                // Exponential step proportional to current level
                let step = (self.envelope_level >> 8) + 1;
                self.envelope_level = self.envelope_level.saturating_sub(step);

                // Sustain target: (sustain_level + 1) * 0x100
                let target = (self.sustain_level as u16 + 1) * 0x100;
                if self.envelope_level <= target {
                    self.envelope_level = target;
                    self.envelope_phase = EnvelopePhase::Sustain;
                    self.tick_counter = 0;
                }
            }

            // -------------------------------------------------
            // SUSTAIN: continue falling exponentially at the sustain rate.
            // Rate 0 = infinite hold (never step).
            // -------------------------------------------------
            EnvelopePhase::Sustain => {
                let rate_idx = self.sustain_rate as usize;
                let period = ENVELOPE_RATE_TABLE[rate_idx.min(31)];
                // period == 0 means hold forever
                if !self.tick_due(period) {
                    return;
                }

                let step = (self.envelope_level >> 8) + 1;
                self.envelope_level = self.envelope_level.saturating_sub(step);

                if self.envelope_level == 0 {
                    self.envelope_phase = EnvelopePhase::Off;
                }
            }

            // -------------------------------------------------
            // RELEASE: fixed linear fade of -8 per tick.
            // Entered on key-off; no rate table gating.
            // -------------------------------------------------
            EnvelopePhase::Release => {
                self.envelope_level = self.envelope_level.saturating_sub(8);
                if self.envelope_level == 0 {
                    self.envelope_phase = EnvelopePhase::Off;
                }
            }

            EnvelopePhase::Off => {}
        }
    }

    /// Returns true if enough ticks have elapsed for an envelope step.
    /// `period` == 0 means never, so always returns false in that case.
    fn tick_due(&mut self, period: u16) -> bool {
        if period == 0 {
            return false;
        }
        self.tick_counter += 1;
        if self.tick_counter >= period {
            self.tick_counter = 0;
            true
        } else {
            false
        }
    }
}

// ============================================================
// BRR DECODER STATE (per voice)
// Tracks position within the compressed BRR sample stream.
// ============================================================

/// BRR playback state for one voice.
#[derive(Debug, Clone, Copy)]
pub struct Brr {
    /// Address of the current BRR block header byte in APU RAM.
    pub addr: u16,

    /// Index of the next sample to consume from sample_buffer (0–15).
    pub nibble_idx: u8,

    /// Most recently decoded sample (p1 in filter equations).
    pub prev1: i16,

    /// Sample before prev1 (p2 in filter equations).
    pub prev2: i16,

    /// Loop start address (from the sample directory table).
    pub loop_addr: u16,

    /// Decoded sample buffer: holds all 16 samples from the current block.
    pub sample_buffer: [i16; 16],

    /// How many valid samples are in sample_buffer (always 16 after a decode).
    pub buffer_fill: u8,
}

impl Default for Brr {
    fn default() -> Self {
        Self {
            addr: 0,
            nibble_idx: 0,
            prev1: 0,
            prev2: 0,
            loop_addr: 0,
            sample_buffer: [0i16; 16],
            buffer_fill: 0,
        }
    }
}

// ============================================================
// VOICE
// Represents one of the 8 independent audio channels.
// ============================================================

/// One voice (channel) of the SNES APU DSP.
#[derive(Debug, Clone, Copy)]
pub struct Voice {
    /// Left channel volume, signed (-128..+127).
    pub left_vol: i8,

    /// Right channel volume, signed (-128..+127).
    pub right_vol: i8,

    /// 14-bit pitch value (0x0000–0x3FFF).
    /// 0x1000 = playback at the native 32 kHz sample rate.
    pub pitch: u16,

    /// Sample source number: index into the DIR table in APU RAM.
    pub srcn: u8,

    /// Whether this voice is currently keyed on (actively playing).
    pub key_on: bool,

    /// 16-bit pitch counter used to pace sample consumption.
    /// Conceptually a fixed-point accumulator; every 0x1000 units = 1 sample.
    pub pitch_counter: u16,

    /// Most recently output sample (16-bit, pre-envelope).
    pub current_sample: i16,

    /// ADSR envelope sub-state.
    pub adsr: Adsr,

    /// BRR decoder sub-state.
    pub brr: Brr,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            left_vol: 0,
            right_vol: 0,
            pitch: 0,
            srcn: 0,
            key_on: false,
            pitch_counter: 0,
            current_sample: 0,
            adsr: Adsr::default(),
            brr: Brr::default(),
        }
    }
}

// ============================================================
// GAUSSIAN INTERPOLATION TABLE
// 512-entry Gaussian kernel taken from the SNES DSP ROM.
// Used to smoothly interpolate between decoded BRR samples,
// eliminating the aliasing that would occur with nearest-neighbour.
// ============================================================

const GAUSS: [i16; 512] = [
      0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
      1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   2,   2,   2,   2,   2,
      2,   2,   3,   3,   3,   3,   3,   4,   4,   4,   4,   4,   5,   5,   5,   5,
      6,   6,   6,   6,   7,   7,   7,   8,   8,   8,   9,   9,   9,  10,  10,  10,
     11,  11,  11,  12,  12,  13,  13,  14,  14,  15,  15,  15,  16,  17,  17,  18,
     18,  19,  19,  20,  20,  21,  21,  22,  23,  23,  24,  24,  25,  26,  27,  27,
     28,  29,  29,  30,  31,  32,  32,  33,  34,  35,  36,  36,  37,  38,  39,  40,
     41,  42,  43,  44,  45,  46,  47,  48,  49,  50,  51,  52,  53,  54,  55,  56,
     58,  59,  60,  61,  62,  64,  65,  66,  67,  69,  70,  71,  73,  74,  76,  77,
     78,  80,  81,  83,  84,  86,  87,  89,  90,  92,  94,  95,  97,  99, 100, 102,
    104, 106, 107, 109, 111, 113, 115, 117, 118, 120, 122, 124, 126, 128, 130, 132,
    134, 137, 139, 141, 143, 145, 147, 150, 152, 154, 156, 159, 161, 163, 166, 168,
    171, 173, 175, 178, 180, 183, 186, 188, 191, 193, 196, 199, 201, 204, 207, 210,
    212, 215, 218, 221, 224, 227, 230, 233, 236, 239, 242, 245, 248, 251, 254, 257,
    260, 263, 267, 270, 273, 276, 280, 283, 286, 290, 293, 297, 300, 304, 307, 311,
    314, 318, 321, 325, 329, 332, 336, 340, 343, 347, 351, 355, 358, 362, 366, 370,
    374, 378, 381, 385, 389, 393, 397, 401, 405, 410, 414, 418, 422, 426, 430, 434,
    439, 443, 447, 451, 456, 460, 464, 469, 473, 477, 482, 486, 491, 495, 499, 504,
    508, 513, 517, 522, 527, 531, 536, 540, 545, 550, 554, 559, 564, 568, 573, 578,
    583, 587, 592, 597, 602, 607, 611, 616, 621, 626, 631, 636, 641, 646, 651, 656,
    661, 666, 671, 676, 681, 686, 691, 696, 702, 707, 712, 717, 722, 727, 733, 738,
    743, 748, 754, 759, 764, 769, 775, 780, 785, 791, 796, 801, 807, 812, 818, 823,
    828, 834, 839, 845, 850, 856, 861, 867, 872, 878, 883, 889, 894, 900, 906, 911,
    917, 922, 928, 934, 939, 945, 951, 956, 962, 968, 974, 979, 985, 991, 997,1002,
   1008,1014,1020,1026,1031,1037,1043,1049,1055,1061,1067,1073,1079,1084,1090,1096,
   1102,1108,1114,1120,1126,1132,1138,1144,1150,1156,1162,1169,1175,1181,1187,1193,
   1199,1205,1211,1217,1224,1230,1236,1242,1249,1255,1261,1267,1274,1280,1286,1293,
   1299,1305,1312,1318,1324,1331,1337,1343,1350,1356,1363,1369,1376,1382,1389,1395,
   1402,1408,1415,1421,1428,1434,1441,1447,1454,1461,1467,1474,1480,1487,1494,1500,
   1507,1514,1520,1527,1534,1541,1547,1554,1561,1568,1574,1581,1588,1595,1602,1609,
   1616,1623,1630,1636,1643,1650,1657,1664,1671,1678,1685,1692,1699,1706,1713,1720,
   1727,1734,1741,1748,1755,1762,1769,1777,1784,1791,1798,1805,1812,1819,1826,1833,
];

// ============================================================
// BRR DECODING FUNCTIONS
// ============================================================

/// Decode one 4-bit BRR nibble into a 16-bit PCM sample.
///
/// Hardware steps:
///   1. Sign-extend nibble from 4 bits to i16
///   2. Scale by shift: arithmetically shift left so the value is
///      placed at the top of a 16-bit integer, then shift right by
///      (12 - shift). This is equivalent to: (nibble << 12) >> (12 - shift).
///      Shifts > 12 are clamped (hardware saturates the sign bit).
///   3. Add the prediction filter result (computed in i32 to avoid overflow)
///   4. Clamp to 15-bit signed range (-16384..+16383) — hardware precision
///   5. The result is a valid i16 (fits in 16 bits)
pub fn decode_brr_nibble(nibble: i8, shift: u8, filter: u8, prev1: i16, prev2: i16) -> i16 {
    // Step 1 + 2: scale the nibble to a 16-bit range.
    let raw: i32 = if shift <= 12 {
        // Standard path: sign-extend to i32, align to bit 15, then scale back.
        let extended = nibble as i32; // already sign-extended from i8
        (extended << 12) >> (12u32.saturating_sub(shift as u32))
    } else {
        // Shifts 13–15: hardware quirk — the sign bit replicates, so the
        // result is either 0x0000 or 0xFFFF (0 or -1 in i16).
        if nibble < 0 { -1 } else { 0 }
    };

    // Step 3: prediction filter in i32 to prevent intermediate overflow.
    let p1 = prev1 as i32;
    let p2 = prev2 as i32;

    let predicted: i32 = match filter {
        0 => 0,
        // Filter 1: coefficient ≈ 15/16
        1 => p1 - (p1 >> 4),
        // Filter 2: p1 * ~61/32 - p2 * ~15/16
        2 => (p1 * 2) - ((p1 * 3) >> 5) - p2 + (p2 >> 4),
        // Filter 3: p1 * ~115/64 - p2 * ~13/16
        3 => (p1 * 2) - ((p1 * 13) >> 6) - p2 + ((p2 * 3) >> 4),
        _ => 0,
    };

    let result = raw + predicted;

    // Step 4: clamp to 15-bit signed range (-0x4000..+0x3FFF).
    // The SNES hardware uses 15 significant bits internally.
    let clamped = result.clamp(-0x4000, 0x3FFF);

    clamped as i16
}

// ============================================================
// RAW RAM ACCESSOR
//
// The DSP needs to read BRR sample data from APU RAM inside
// step().  Normally we would pass &Memory for this, but Memory
// owns Dsp — so calling self.memory.dsp.step(&self.memory) from
// Apu creates a simultaneous mutable + immutable borrow that the
// compiler rejects.
//
// The fix is to pass a plain &[u8] RAM snapshot instead.
// step_with_ram() and decode_brr_block_raw() are the &[u8] variants
// used by Apu.  The original Memory-based step() / decode_brr_block()
// are kept for unit tests.
//
// Long-term the Memory-based variants can be removed once all call
// sites have been migrated.
// ============================================================

/// Read one byte from a raw RAM slice; returns 0 for out-of-range addresses.
#[inline(always)]
fn ram_read8(ram: &[u8], addr: u16) -> u8 {
    ram.get(addr as usize).copied().unwrap_or(0)
}

/// &[u8] variant of decode_brr_block — identical logic, no Memory borrow.
pub fn decode_brr_block_raw(
    ram: &[u8],
    addr: u16,
    prev1: &mut i16,
    prev2: &mut i16,
) -> ([i16; 16], bool, bool) {
    let header = ram_read8(ram, addr);

    let shift  = (header >> 4) & 0x0F;
    let filter = (header >> 2) & 0x03;
    let looop  = (header & 0x02) != 0;
    let end    = (header & 0x01) != 0;

    let mut samples = [0i16; 16];

    for i in 0..8usize {
        let byte = ram_read8(ram, addr + 1 + i as u16);

        let hi_raw = ((byte >> 4) & 0x0F) as i8;
        let hi = if hi_raw & 0x08 != 0 { hi_raw | !0x0F } else { hi_raw };
        let lo_raw = (byte & 0x0F) as i8;
        let lo = if lo_raw & 0x08 != 0 { lo_raw | !0x0F } else { lo_raw };

        let s0 = decode_brr_nibble(hi, shift, filter, *prev1, *prev2);
        *prev2 = *prev1; *prev1 = s0;
        samples[i * 2] = s0;

        let s1 = decode_brr_nibble(lo, shift, filter, *prev1, *prev2);
        *prev2 = *prev1; *prev1 = s1;
        samples[i * 2 + 1] = s1;
    }

    (samples, end, looop)
}

/// Decode all 16 samples from one 9-byte BRR block at `addr` in APU RAM.
///
/// BRR block layout:
///   byte 0   — header: SSSSFFEX
///     bits 7-4 = shift  (S, 0–15)
///     bits 3-2 = filter (F, 0–3)
///     bit  1   = loop   (E: on end, jump to loop point)
///     bit  0   = end    (X: this is the last block)
///   bytes 1-8 — 8 data bytes, each holding 2 nibbles (high then low)
///               = 16 samples total
///
/// `prev1` and `prev2` are updated in place as decoding proceeds,
/// so they carry the history forward to the next block.
///
/// Returns (samples[16], end_flag, loop_flag).
pub fn decode_brr_block(
    mem: &Memory,
    addr: u16,
    prev1: &mut i16,
    prev2: &mut i16,
) -> ([i16; 16], bool, bool) {
    let header = mem.read8(addr);

    // Correct bit layout: SSSSFFEX
    let shift  = (header >> 4) & 0x0F; // bits 7-4: shift amount
    let filter = (header >> 2) & 0x03; // bits 3-2: filter index
    let looop  = (header & 0x02) != 0; // bit  1:   loop flag
    let end    = (header & 0x01) != 0; // bit  0:   end flag

    let mut samples = [0i16; 16];

    for i in 0..8usize {
        let byte = mem.read8(addr + 1 + i as u16);

        // High nibble first (bits 7-4)
        let hi_raw = ((byte >> 4) & 0x0F) as i8;
        let hi = if hi_raw & 0x08 != 0 { hi_raw | !0x0F } else { hi_raw }; // sign-extend from 4 bits

        // Low nibble (bits 3-0)
        let lo_raw = (byte & 0x0F) as i8;
        let lo = if lo_raw & 0x08 != 0 { lo_raw | !0x0F } else { lo_raw };

        // Decode high nibble sample
        let s0 = decode_brr_nibble(hi, shift, filter, *prev1, *prev2);
        *prev2 = *prev1;
        *prev1 = s0;
        samples[i * 2] = s0;

        // Decode low nibble sample (uses the updated prev1/prev2 from s0)
        let s1 = decode_brr_nibble(lo, shift, filter, *prev1, *prev2);
        *prev2 = *prev1;
        *prev1 = s1;
        samples[i * 2 + 1] = s1;
    }

    (samples, end, looop)
}

// ============================================================
// DSP CORE
// ============================================================

/// The SNES DSP: 8 voices, ADSR envelopes, BRR decoding, stereo mix.
pub struct Dsp {
    /// 128 DSP registers (indexed 0x00–0x7F).
    /// Accessed externally via SPC700 I/O ports $F2 (index) / $F3 (data).
    registers: [u8; 128],

    /// The 8 independent audio voices.
    pub voices: [Voice; 8],

    /// DIR register ($5D): high byte of the sample directory base address.
    /// Full address = dir_base * 0x100.
    dir_base: u8,
}

impl Dsp {
    pub fn new() -> Self {
        Self {
            registers: [0u8; 128],
            voices: [Voice::default(); 8],
            dir_base: 0,
        }
    }

    // ----------------------------------------------------------
    // Register I/O
    //
    // DSP register map (7-bit index, 0x00–0x7F):
    //
    // Per-voice block (16 bytes per voice, voice N at offset N*0x10):
    //   +0x0  VOL(L)   signed left volume
    //   +0x1  VOL(R)   signed right volume
    //   +0x2  PITCHL   pitch low byte
    //   +0x3  PITCHH   pitch high byte (6 bits, 13-8)
    //   +0x4  SRCN     sample source number
    //   +0x5  ADSR1    EDDDAAAA
    //   +0x6  ADSR2    SSSRRRRR
    //   +0x7  GAIN     (not yet implemented)
    //   +0x8  ENVX     read-only envelope (7-bit)
    //   +0x9  OUTX     read-only output (8-bit)
    //
    // Global registers (in the upper nibble of each 16-byte block):
    //   $0C  MVOLL    master left volume
    //   $1C  MVOLR    master right volume
    //   $2C  EVOLL    echo left volume
    //   $3C  EVOLR    echo right volume
    //   $4C  KON      key on  (1 bit per voice)
    //   $5C  KOFF     key off (1 bit per voice)
    //   $6C  FLG      flags
    //   $7C  ENDX     end flags (read)
    //   $0D  EFB      echo feedback
    //   $2D  PMON     pitch modulation enable
    //   $3D  NON      noise enable
    //   $4D  EON      echo enable
    //   $5D  DIR      sample directory base page
    //   $6D  ESA      echo start page
    //   $7D  EDL      echo delay
    // ----------------------------------------------------------

    /// Read a DSP register by its 7-bit index.
    pub fn read_reg(&self, index: u8) -> u8 {
        self.registers[(index & 0x7F) as usize]
    }

    /// Write a DSP register by its 7-bit index and update internal state.
    pub fn write_reg(&mut self, index: u8, value: u8) {
        let idx = (index & 0x7F) as usize;
        self.registers[idx] = value;

        let voice_num = idx >> 4;   // high nibble = voice 0–7
        let reg_off   = idx & 0x0F; // low nibble  = register within voice block

        match (voice_num, reg_off) {
            // ---- Per-voice registers (voices 0-7) ----

            // +0: VOL(L) — signed left volume
            (v, 0x0) if v < 8 => self.voices[v].left_vol = value as i8,

            // +1: VOL(R) — signed right volume
            (v, 0x1) if v < 8 => self.voices[v].right_vol = value as i8,

            // +2: PITCH low byte
            (v, 0x2) if v < 8 => {
                let p = &mut self.voices[v].pitch;
                *p = (*p & 0x3F00) | (value as u16);
            }

            // +3: PITCH high byte (only bits 5-0 = pitch bits 13-8)
            (v, 0x3) if v < 8 => {
                let p = &mut self.voices[v].pitch;
                *p = (*p & 0x00FF) | ((value as u16 & 0x3F) << 8);
            }

            // +4: SRCN — sample source number (index into DIR table)
            (v, 0x4) if v < 8 => self.voices[v].srcn = value,

            // +5: ADSR1 = EDDDAAAA
            //   bit 7:    ADSR enable (1=ADSR, 0=GAIN)
            //   bits 6-4: decay rate index (0–7)
            //   bits 3-0: attack rate index (0–15)
            (v, 0x5) if v < 8 => {
                let adsr = &mut self.voices[v].adsr;
                adsr.adsr_mode   = (value & 0x80) != 0;
                adsr.decay_rate  = (value >> 4) & 0x07;
                adsr.attack_rate =  value & 0x0F;
            }

            // +6: ADSR2 = SSSRRRRR
            //   bits 7-5: sustain level (0–7)
            //   bits 4-0: sustain rate index (0–31)
            (v, 0x6) if v < 8 => {
                let adsr = &mut self.voices[v].adsr;
                adsr.sustain_level = (value >> 5) & 0x07;
                adsr.sustain_rate  =  value & 0x1F;
            }

            // +7: GAIN — TODO: implement GAIN mode
            (v, 0x7) if v < 8 => { /* GAIN mode not yet implemented */ }

            // ---- Global registers ----

            // $4C: KON — key on, one bit per voice (bit 0 = voice 0)
            (_, _) if idx == 0x4C => {
                for v in 0..8usize {
                    if value & (1 << v) != 0 {
                        self.key_on_voice(v);
                    }
                }
            }

            // $5C: KOFF — key off, enter release phase
            (_, _) if idx == 0x5C => {
                for v in 0..8usize {
                    if value & (1 << v) != 0 {
                        self.voices[v].key_on = false;
                        self.voices[v].adsr.envelope_phase = EnvelopePhase::Release;
                    }
                }
            }

            // $5D: DIR — sample directory base page
            (_, _) if idx == 0x5D => {
                self.dir_base = value;
            }

            _ => {}
        }
    }

    /// Handle key-on for voice `v`.
    ///
    /// Marks the voice active and resets all playback state.
    /// The actual BRR start/loop addresses are read from the DIR table
    /// on the first call to `step()` after key-on, when we have access
    /// to APU RAM.
    fn key_on_voice(&mut self, v: usize) {
        let voice = &mut self.voices[v];
        voice.key_on = true;

        // Compute the directory entry address for this source number.
        // Each DIR entry is 4 bytes: [start_lo, start_hi, loop_lo, loop_hi].
        // We store the dir entry address in brr.addr temporarily;
        // step() will resolve it to the real BRR address on the first tick.
        let dir_entry = (self.dir_base as u16) * 0x100 + (voice.srcn as u16) * 4;
        voice.brr.addr = dir_entry; // sentinel: will be resolved in step()

        // Reset BRR state
        voice.brr.nibble_idx = 0;
        voice.brr.prev1 = 0;
        voice.brr.prev2 = 0;
        voice.brr.buffer_fill = 0;
        voice.brr.loop_addr = 0;

        // Reset pitch counter
        voice.pitch_counter = 0;

        // Reset envelope to start of attack
        voice.adsr.envelope_phase = EnvelopePhase::Attack;
        voice.adsr.envelope_level = 0;
        voice.adsr.tick_counter = 0;

        voice.current_sample = 0;
    }

    // ----------------------------------------------------------
    // DSP step — two variants
    //
    // step_with_ram(&[u8])  — called by Apu; avoids the Memory borrow conflict.
    // step(&Memory)         — kept for unit tests that construct Memory directly.
    //
    // Both do exactly the same work; only the RAM read primitive differs.
    // ----------------------------------------------------------

    /// Advance the DSP by one output sample tick using a raw RAM slice.
    ///
    /// Called by `Apu::step()` to avoid the simultaneous mutable/immutable
    /// borrow that would arise from passing `&self.memory` while also holding
    /// `&mut self.memory.dsp`.
    pub fn step_with_ram(&mut self, ram: &[u8]) {
        for v in 0..8usize {
            // 1. Envelope update
            if self.voices[v].adsr.envelope_phase != EnvelopePhase::Off {
                self.voices[v].adsr.update_envelope();
            }

            if !self.voices[v].key_on
                && self.voices[v].adsr.envelope_phase == EnvelopePhase::Off
            {
                continue;
            }
            if !self.voices[v].key_on {
                continue;
            }

            // 2. Resolve DIR table on first tick after key-on
            if self.voices[v].brr.buffer_fill == 0 {
                let dir_entry = self.voices[v].brr.addr;

                let start_lo = ram_read8(ram, dir_entry)     as u16;
                let start_hi = ram_read8(ram, dir_entry + 1) as u16;
                let loop_lo  = ram_read8(ram, dir_entry + 2) as u16;
                let loop_hi  = ram_read8(ram, dir_entry + 3) as u16;

                self.voices[v].brr.addr      = (start_hi << 8) | start_lo;
                self.voices[v].brr.loop_addr = (loop_hi  << 8) | loop_lo;

                self.decode_next_block_raw(v, ram);
            }

            // 3. Pitch counter advance
            let pitch = self.voices[v].pitch & 0x3FFF;
            self.voices[v].pitch_counter =
                self.voices[v].pitch_counter.wrapping_add(pitch);

            let samples_to_consume = self.voices[v].pitch_counter / 0x1000;
            self.voices[v].pitch_counter %= 0x1000;

            // 4. Consume decoded samples from buffer
            for _ in 0..samples_to_consume {
                let idx = self.voices[v].brr.nibble_idx as usize;
                if idx < self.voices[v].brr.buffer_fill as usize {
                    self.voices[v].current_sample =
                        self.voices[v].brr.sample_buffer[idx];
                    self.voices[v].brr.nibble_idx += 1;
                }

                if self.voices[v].brr.nibble_idx >= self.voices[v].brr.buffer_fill {
                    self.voices[v].brr.nibble_idx = 0;
                    self.decode_next_block_raw(v, ram);
                    if !self.voices[v].key_on {
                        break;
                    }
                }
            }
        }
    }

    /// &[u8] variant of decode_next_block — used by step_with_ram.
    fn decode_next_block_raw(&mut self, v: usize, ram: &[u8]) {
        let voice = &mut self.voices[v];

        let (samples, end, do_loop) = decode_brr_block_raw(
            ram,
            voice.brr.addr,
            &mut voice.brr.prev1,
            &mut voice.brr.prev2,
        );

        voice.brr.sample_buffer = samples;
        voice.brr.buffer_fill   = 16;
        voice.brr.nibble_idx    = 0;

        if end {
            if do_loop {
                voice.brr.addr = voice.brr.loop_addr;
            } else {
                voice.key_on = false;
                voice.adsr.envelope_phase = EnvelopePhase::Release;
            }
        } else {
            voice.brr.addr = voice.brr.addr.wrapping_add(9);
        }
    }

    // ----------------------------------------------------------
    // DSP step — called once per output sample (32000 Hz)
    // ----------------------------------------------------------

    pub fn step(&mut self, mem: &Memory) {
        for v in 0..8usize {
            // ---- 1. Envelope update ----
            if self.voices[v].adsr.envelope_phase != EnvelopePhase::Off {
                self.voices[v].adsr.update_envelope();
            }

            // Skip voices that are fully silent and not playing
            if !self.voices[v].key_on
                && self.voices[v].adsr.envelope_phase == EnvelopePhase::Off
            {
                continue;
            }

            if !self.voices[v].key_on {
                continue;
            }

            // ---- 2. Resolve DIR table on first tick after key-on ----
            // buffer_fill == 0 means we haven't decoded anything yet.
            if self.voices[v].brr.buffer_fill == 0 {
                let dir_entry = self.voices[v].brr.addr;

                // Read 4-byte directory entry
                let start_lo = mem.read8(dir_entry)     as u16;
                let start_hi = mem.read8(dir_entry + 1) as u16;
                let loop_lo  = mem.read8(dir_entry + 2) as u16;
                let loop_hi  = mem.read8(dir_entry + 3) as u16;

                let start_addr = (start_hi << 8) | start_lo;
                let loop_addr  = (loop_hi  << 8) | loop_lo;

                self.voices[v].brr.addr      = start_addr;
                self.voices[v].brr.loop_addr = loop_addr;

                // Decode the first block immediately
                self.decode_next_block(v, mem);
            }

            // ---- 3. Pitch counter advance ----
            // Add pitch to the counter each output sample.
            // Every 0x1000 units = one BRR sample consumed.
            let pitch = self.voices[v].pitch & 0x3FFF;
            let old_counter = self.voices[v].pitch_counter;
            self.voices[v].pitch_counter = old_counter.wrapping_add(pitch);

            // Number of whole BRR samples to consume this tick
            let samples_to_consume = self.voices[v].pitch_counter / 0x1000;
            self.voices[v].pitch_counter %= 0x1000;

            // ---- 4. Consume decoded samples from buffer ----
            for _ in 0..samples_to_consume {
                let idx = self.voices[v].brr.nibble_idx as usize;

                if idx < self.voices[v].brr.buffer_fill as usize {
                    self.voices[v].current_sample = self.voices[v].brr.sample_buffer[idx];
                    self.voices[v].brr.nibble_idx += 1;
                }

                // If the buffer is exhausted, decode the next block
                if self.voices[v].brr.nibble_idx >= self.voices[v].brr.buffer_fill {
                    self.voices[v].brr.nibble_idx = 0;
                    self.decode_next_block(v, mem);

                    // decode_next_block may have silenced the voice (no-loop end)
                    if !self.voices[v].key_on {
                        break;
                    }
                }
            }
        }
    }

    /// Decode the next 9-byte BRR block for voice `v` and advance the address.
    ///
    /// Handles end/loop flags:
    /// - end=true,  loop=true  → jump to loop_addr and continue
    /// - end=true,  loop=false → silence the voice (enter release)
    /// - end=false             → advance address by 9 bytes
    fn decode_next_block(&mut self, v: usize, mem: &Memory) {
        let voice = &mut self.voices[v];

        let (samples, end, do_loop) = decode_brr_block(
            mem,
            voice.brr.addr,
            &mut voice.brr.prev1,
            &mut voice.brr.prev2,
        );

        voice.brr.sample_buffer = samples;
        voice.brr.buffer_fill   = 16;
        voice.brr.nibble_idx    = 0;

        if end {
            if do_loop {
                // Jump to the loop point from the DIR table
                voice.brr.addr = voice.brr.loop_addr;
                // Note: prev1/prev2 carry forward through the loop boundary
            } else {
                // No loop: silence the voice after the current buffer drains
                voice.key_on = false;
                voice.adsr.envelope_phase = EnvelopePhase::Release;
            }
        } else {
            // Normal advance: next BRR block is 9 bytes later
            voice.brr.addr = voice.brr.addr.wrapping_add(9);
        }
    }

    // ----------------------------------------------------------
    // Stereo mix
    // ----------------------------------------------------------

    /// Mix all active voices into one stereo output sample pair.
    ///
    /// Uses integer arithmetic throughout to match hardware behaviour.
    /// Volumes are signed i8; samples and envelope are 16-bit.
    /// The accumulator is i32 to prevent overflow during summation.
    pub fn render_audio_single(&self) -> (i16, i16) {
        let mut left:  i32 = 0;
        let mut right: i32 = 0;

        for voice in self.voices.iter() {
            if voice.adsr.envelope_phase == EnvelopePhase::Off {
                continue;
            }

            // Scale sample by 11-bit envelope (0–0x7FF) → back to ~16-bit range
            let env    = voice.adsr.envelope_level as i32; // 0–0x7FF
            let sample = voice.current_sample as i32;      // -32768..+32767
            let scaled = (sample * env) >> 11;             // ~16-bit result

            // Apply signed per-voice volumes (i8, -128..+127), shift by 7
            left  += (scaled * voice.left_vol  as i32) >> 7;
            right += (scaled * voice.right_vol as i32) >> 7;
        }

        (
            left .clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            right.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        )
    }
}
