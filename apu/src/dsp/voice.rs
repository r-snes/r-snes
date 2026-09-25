use crate::memory::RawARAM;

use super::adsr::{Adsr, EnvelopePhase};
use super::brr::{Brr, GAUSS, decode_brr_block, ram_read8};

/// One voice (channel) of the SNES APU DSP.
#[derive(Debug, Clone, Copy, Default)]
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

    /// The DSP's interpolation position (hardware's `interp_pos`),
    /// 0..=0x7FFF, in units of 1/0x1000 of a sample. The low 12 bits are
    /// the fractional position used by `interpolate`. The top bits count
    /// whole samples ahead of the current window, and hardware drops 4 of
    /// those each tick once they reach 4 (see `step`). Keeping the whole
    /// value, not just the fraction, is what lets the 0x7FFF cap behave
    /// exactly as on hardware under strong pitch modulation.
    pub pitch_counter: u16,

    /// Most recently output sample (16-bit, pre-envelope). This is the
    /// *interpolated* output — see `interpolate` — not a raw decoded
    /// BRR sample.
    pub current_sample: i16,

    /// The 4 most recently decoded raw BRR samples, oldest to newest
    /// (`history[3]` is the newest). Gaussian-interpolated every tick
    /// against the fractional pitch position to produce `current_sample`.
    /// Reset on key-on (see `Dsp::key_on_voice`) so a new note doesn't
    /// interpolate against the previous note's tail.
    pub history: [i16; 4],

    /// ADSR envelope sub-state.
    pub adsr: Adsr,

    /// BRR decoder sub-state.
    pub brr: Brr,
}

impl Voice {
    /// Advance this voice by one DSP tick.
    ///
    /// `i` is the voice index (0–7), used to compute the ENVX register
    /// offsets and the ENDX bitmask.
    /// `registers` is the DSP register file; ENVX and ENDX are written
    /// here so the CPU can read them back via `$F3` (OUTX is written by
    /// `Dsp::step`).
    /// `pmon_source` is the pitch-modulation input for this tick.
    pub fn step(&mut self, i: usize, ram: &RawARAM, registers: &mut [u8; 128], pmon_source: Option<i32>,) {
        // 1. Envelope update
        if self.adsr.envelope_phase != EnvelopePhase::Off {
            self.adsr.update_envelope();
        }

        // A voice only goes fully idle once its envelope has actually
        // reached Off. `key_on == false` alone (i.e. right after KOFF)
        // is not enough to stop here: the voice is in Release and must
        // keep decoding/consuming BRR samples while the envelope fades
        // it out underneath, the same way real hardware keeps playing
        // through a release instead of freezing on the last sample.
        if !self.key_on && self.adsr.envelope_phase == EnvelopePhase::Off {
            return;
        }

        // 2. Resolve DIR table on first tick after key-on.
        // buffer_fill == 0 means no block has been decoded yet.
        if self.brr.buffer_fill == 0 {
            let dir_entry = self.brr.addr;

            let start_lo = ram_read8(ram, dir_entry) as u16;
            let start_hi = ram_read8(ram, dir_entry + 1) as u16;
            let loop_lo = ram_read8(ram, dir_entry + 2) as u16;
            let loop_hi = ram_read8(ram, dir_entry + 3) as u16;

            self.brr.addr = (start_hi << 8) | start_lo;
            self.brr.loop_addr = (loop_hi << 8) | loop_lo;

            self.decode_next_block(i, ram, registers);
        }

        // 3. Pitch counter advance.
        // Every 0x1000 units = one BRR sample consumed.
        //
        // PMON formula
        let base_pitch = (self.pitch & 0x3FFF) as i32;
        let pitch = match pmon_source {
            Some(x) => base_pitch + (((x >> 5) * base_pitch) >> 10),
            None => base_pitch,
        };

        // Update of the interpolation position:
        let prev_pos = self.pitch_counter & 0x3FFF;
        let new_pos = (prev_pos as i32 + pitch).min(0x7FFF) as u16;
        self.pitch_counter = new_pos;

        // Whole samples crossed this tick. Unmodulated pitch is at most
        // 0x3FFF, so new_pos <= 0x7FFE: the cap never applies.
        let samples_to_consume = (new_pos >> 12) - (prev_pos >> 12);

        // 4. Shift each newly-reached raw decoded sample into the
        // 4-sample interpolation history as the pitch counter crosses it.
        for _ in 0..samples_to_consume {
            let idx = self.brr.nibble_idx as usize;
            if idx < self.brr.buffer_fill as usize {
                self.push_history(self.brr.sample_buffer[idx]);
                self.brr.nibble_idx += 1;
            }

            if self.brr.nibble_idx >= self.brr.buffer_fill {
                self.brr.nibble_idx = 0;
                self.decode_next_block(i, ram, registers);
                if !self.key_on {
                    break;
                }
            }
        }

        // 5. Gaussian-interpolate this tick's output sample from the
        // 4-sample history and the fractional part of the pitch counter
        // (how far between the last and next raw sample we currently are).
        self.current_sample = self.interpolate();

        // 6. Update the read-only ENVX ($X8) register.
        //   ENVX = envelope_level >> 4  (11-bit → 7-bit)
        // OUTX ($X9) is written by `Dsp::step` instead: it is the
        // post-envelope output, which is only final after NON may have
        // substituted noise for this voice's sample.
        registers[(i << 4) | 0x8] = (self.adsr.envelope_level >> 4) as u8;
    }

    /// Shift a newly decoded raw sample into the 4-sample history,
    /// oldest-to-newest (`history[3]` is always the most recent).
    pub fn push_history(&mut self, sample: i16) {
        self.history[0] = self.history[1];
        self.history[1] = self.history[2];
        self.history[2] = self.history[3];
        self.history[3] = sample;
    }

    /// Gaussian-interpolate the current output sample from the 4-sample
    /// history and the fractional part of the pitch counter.
    ///
    /// The intermediate cast to i16 after the first three taps reproduces
    /// a documented quirk of the real DSP's interpolator (it truncates to
    /// 16 bits there before adding the fourth tap); this is required for
    /// bit-accurate output, not a mistake.
    pub fn interpolate(&self) -> i16 {
        let index = ((self.pitch_counter >> 4) & 0xFF) as usize;
        let h = &self.history;

        let mut out: i32 = (GAUSS[255 - index] as i32 * h[0] as i32) >> 11;
        out += (GAUSS[511 - index] as i32 * h[1] as i32) >> 11;
        out += (GAUSS[index + 256] as i32 * h[2] as i32) >> 11;
        out = out as i16 as i32; // hardware quirk: truncate before the 4th tap
        out += (GAUSS[index] as i32 * h[3] as i32) >> 11;

        out.clamp(i16::MIN as i32, i16::MAX as i32) as i16
    }

    /// Decode the next 9-byte BRR block and advance the BRR address.
    ///
    /// Handles end/loop flags:
    /// - end=true,  loop=true  → jump to loop_addr and continue
    /// - end=true,  loop=false → mute the voice immediately
    /// - end=false             → advance address by 9 bytes
    ///
    /// Sets bit `i` of `registers[0x7C]` (ENDX) when an end block is reached.
    fn decode_next_block(&mut self, i: usize, ram: &RawARAM, registers: &mut [u8; 128]) {
        let (samples, end, do_loop) =
            decode_brr_block(ram, self.brr.addr, &mut self.brr.prev1, &mut self.brr.prev2);

        self.brr.sample_buffer = samples;
        self.brr.buffer_fill = 16;
        self.brr.nibble_idx = 0;

        if end {
            registers[0x7C] |= 1u8 << i;

            if do_loop {
                self.brr.addr = self.brr.loop_addr;
            } else {
                // Real hardware does NOT fade this out via a normal
                // release: hitting an end block with the loop flag clear
                // forces the envelope to 0 immediately (a hard mute), not
                // a -8/tick ramp. Setting the phase straight to `Off`
                // (rather than `Release`) also stops `step()`'s
                // `!key_on && phase == Off` guard from letting the voice
                // keep running afterwards — otherwise it would go on
                // re-decoding and replaying this same terminal block for
                // the rest of the fade instead of going silent.
                self.key_on = false;
                self.adsr.envelope_level = 0;
                self.adsr.envelope_phase = EnvelopePhase::Off;
            }
        } else {
            self.brr.addr = self.brr.addr.wrapping_add(9);
        }
    }
}
