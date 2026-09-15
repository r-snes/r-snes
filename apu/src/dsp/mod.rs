mod adsr;
mod brr;
mod voice;

// Re-export everything tests and external code need
pub use adsr::{Adsr, EnvelopePhase};
pub use brr::{Brr, decode_brr_block, decode_brr_nibble};
pub use voice::Voice;

use adsr::ENVELOPE_RATE_TABLE;
use common::u16_split::U16Split;

use crate::memory::RawARAM;

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

    /// $0C MVOLL — master left  volume, signed (-128..+127).
    /// Applied to the final summed mix as a global output scaler.
    /// Initialised to 0: game code must write a non-zero value to hear output.
    master_vol_left: i8,

    /// $1C MVOLR — master right volume, signed (-128..+127).
    master_vol_right: i8,

    /// $6C FLG — global DSP flags:
    ///   bit 7: soft RESET (also forces mute per hardware; silences every
    ///          voice immediately, clears ENDX, and blocks new key-ons
    ///          for as long as this bit stays set)
    ///   bit 6: MUTE — forces final output to silence without touching
    ///          voice/envelope state (unlike RESET, playback keeps running
    ///          underneath, it just isn't heard)
    ///   bit 5: disable echo writes (no effect yet — echo isn't implemented)
    ///   bits 4-0: noise clock — index into the shared rate table
    ///             (ENVELOPE_RATE_TABLE; 0 = stopped). See `advance_noise`.
    flg: u8,

    /// $3D NON — one bit per voice; when set, that voice's mixed output
    /// is the shared noise generator instead of its BRR-decoded sample.
    /// BRR decoding keeps running underneath regardless (see `step`), so
    /// clearing NON resumes wherever that voice's sample stream got to.
    non: u8,

    /// Shared 15-bit noise LFSR (bits 0-14; bit 15 always 0). Advanced by
    /// `advance_noise` at the rate selected by FLG bits 0-4. Seeded
    /// non-zero — an LFSR seeded with 0 would XOR itself into permanent
    /// silence. The exact real-hardware power-on seed isn't verified here;
    /// any non-zero seed produces the same statistical noise.
    noise_lfsr: u16,

    /// Ticks elapsed since the last noise LFSR advance, compared against
    /// the period selected by FLG bits 0-4 (same tick-gating pattern as
    /// `Adsr::tick_due`).
    noise_tick_counter: u16,

    // ---- Echo registers (Stage 1: storage/roundtrip only — the actual
    // echo buffer, FIR filtering, and voice routing land in later stages;
    // none of these affect audio output yet). ----
    /// $0D EFB — echo feedback, signed (-128..+127). Scales the echo
    /// buffer's own most recent output before it's written back into the
    /// buffer, so echoes decay (or grow, or invert) over repetitions.
    efb: i8,

    /// $4D EON — one bit per voice; when set, that voice's dry output is
    /// also summed into the echo buffer's input, in addition to the
    /// normal dry mix.
    eon: u8,

    /// $6D ESA — echo buffer base page in APU RAM. Full base address =
    /// esa * 0x100.
    esa: u8,

    /// $7D EDL — echo delay length, 0-15 (only the low nibble is
    /// meaningful; upper bits are ignored, matching hardware). Buffer
    /// size = edl * 512 stereo sample pairs (2 KB per unit). EDL=0 means
    /// no delay buffer at all — echo is effectively bypassed.
    edl: u8,

    /// $0F/$1F/.../$7F — the 8-tap FIR filter coefficients, signed. Real
    /// hardware quirk: these occupy the position that would be "voice N's
    /// GAIN+8" register for each voice 0-7 (reg offset 0xF), but they
    /// aren't per-voice data — together the 8 values form one global
    /// filter applied to the echo buffer. `fir_coeff[N]` is tap N,
    /// stored at register `N*0x10 + 0x0F`.
    fir_coeff: [i8; 8],
}

impl Default for Dsp {
    fn default() -> Self {
        Self::new()
    }
}

impl Dsp {
    pub fn new() -> Self {
        Self {
            registers: [0u8; 128],
            voices: [Voice::default(); 8],
            dir_base: 0,
            // Hardware resets master volume to 0; game code sets it during boot.
            master_vol_left: 0,
            master_vol_right: 0,
            // Real hardware powers up with RESET set; our HLE boot skips
            // the IPL's own register init, so start "already booted"
            // (not reset/muted) like the rest of the zero-initialized
            // register file — the driver writes $6C itself during setup.
            flg: 0,
            non: 0,
            noise_lfsr: 0x4000,
            noise_tick_counter: 0,
            efb: 0,
            eon: 0,
            esa: 0,
            edl: 0,
            fir_coeff: [0i8; 8],
        }
    }

    /// Read a DSP register by its 7-bit index.
    ///
    /// DSP register map (7-bit index `0x00–0x7F`):
    ///
    /// Per-voice block — voice N at offset `N * 0x10`:
    /// ```text
    /// +0x0 VOL(L)  +0x1 VOL(R)  +0x2 PITCHL  +0x3 PITCHH
    /// +0x4 SRCN    +0x5 ADSR1   +0x6 ADSR2   +0x7 GAIN
    /// +0x8 ENVX    +0x9 OUTX
    /// ```
    /// Global registers:
    /// ```text
    /// $0C MVOLL  $1C MVOLR  $4C KON   $5C KOFF  $5D DIR
    /// $6C FLG    $7C ENDX   $0D EFB   $2D PMON  $3D NON
    /// $4D EON    $6D ESA    $7D EDL
    /// ```
    pub fn read_reg(&self, index: u8) -> u8 {
        self.registers[(index & 0x7F) as usize]
    }

    pub fn edl(&self) -> u8 {
        self.edl
    }

    /// Write a DSP register by its 7-bit index and update internal state.
    pub fn write_reg(&mut self, index: u8, value: u8) {
        let idx = (index & 0x7F) as usize;
        self.registers[idx] = value;

        let voice_num = idx >> 4; // high nibble = voice 0–7
        let reg_off = idx & 0x0F; // low nibble  = register within voice block

        // voice_num = idx >> 4, idx = index & 0x7F, so voice_num <= 7 always.
        // The `if v < 8` guards are therefore redundant and omitted.
        match (voice_num, reg_off) {
            // ---- Per-voice registers ----

            // +0: VOL(L) — signed left volume
            (v, 0x0) => self.voices[v].left_vol = value as i8,

            // +1: VOL(R) — signed right volume
            (v, 0x1) => self.voices[v].right_vol = value as i8,

            // +2: PITCH low byte
            (v, 0x2) => {
                let p = &mut self.voices[v].pitch;
                *p.lo_mut() = value;
            }

            // +3: PITCH high byte (only bits 5-0 = pitch bits 13-8)
            (v, 0x3) => {
                let p = &mut self.voices[v].pitch;
                *p.hi_mut() = value & 0x3F;
            }

            // +4: SRCN — sample source number (index into DIR table)
            (v, 0x4) => self.voices[v].srcn = value,

            // +5: ADSR1 = EDDDAAAA
            //   bit 7:    ADSR enable (1=ADSR, 0=GAIN)
            //   bits 6-4: decay rate index (0–7)
            //   bits 3-0: attack rate index (0–15)
            (v, 0x5) => {
                let adsr = &mut self.voices[v].adsr;
                adsr.adsr_mode = (value & 0x80) != 0;
                adsr.decay_rate = (value >> 4) & 0x07;
                adsr.attack_rate = value & 0x0F;
            }

            // +6: ADSR2 = SSSRRRRR
            //   bits 7-5: sustain level (0–7)
            //   bits 4-0: sustain rate index (0–31)
            (v, 0x6) => {
                let adsr = &mut self.voices[v].adsr;
                adsr.sustain_level = (value >> 5) & 0x07;
                adsr.sustain_rate = value & 0x1F;
            }

            // +7: GAIN — envelope control when ADSR1 bit 7 is clear.
            // Stored raw; Adsr::update_gain interprets it per tick.
            // Writable at any time (drivers sweep it for hand-rolled
            // envelopes and fades)
            (v, 0x7) => self.voices[v].adsr.gain_param = value,

            // +0xF: FIR coefficient tap `v` — NOT per-voice data. This is
            // the well-documented hardware quirk of sharing register
            // space with the per-voice block; see `fir_coeff`'s doc.
            (v, 0xF) => self.fir_coeff[v] = value as i8,

            // ---- Global registers ----
            _ => match idx {
                // $4C: KON — key on, one bit per voice (bit 0 = voice 0).
                // Ignored while FLG's RESET bit is set — real hardware
                // blocks new key-ons for as long as the DSP is held in
                // reset.
                0x4C => {
                    if self.flg & 0x80 == 0 {
                        for v in 0..8usize {
                            if value & (1 << v) != 0 {
                                self.key_on_voice(v);
                            }
                        }
                    }
                }

                // $5C: KOFF — key off, enter release phase
                0x5C => {
                    for v in 0..8usize {
                        if value & (1 << v) != 0 {
                            self.voices[v].key_on = false;
                            self.voices[v].adsr.envelope_phase = EnvelopePhase::Release;
                        }
                    }
                }

                // $0C: MVOLL — master left  volume (signed)
                0x0C => self.master_vol_left = value as i8,

                // $1C: MVOLR — master right volume (signed)
                0x1C => self.master_vol_right = value as i8,

                // $5D: DIR — sample directory base page
                0x5D => self.dir_base = value,

                // $3D: NON — one bit per voice; see the `non` field doc.
                0x3D => self.non = value,

                // ---- Echo registers (Stage 1: stored, no audio effect
                // yet — the buffer/FIR/routing land in later stages) ----
                // $0D: EFB — echo feedback, signed.
                0x0D => self.efb = value as i8,
                // $4D: EON — one bit per voice; see the `eon` field doc.
                0x4D => self.eon = value,
                // $6D: ESA — echo buffer base page.
                0x6D => self.esa = value,
                // $7D: EDL — echo delay length; only the low nibble is
                // meaningful (see the `edl` field doc).
                0x7D => self.edl = value & 0x0F,

                // $6C: FLG — noise clock / echo-write-disable / mute / reset.
                // Noise clock changes take effect on the next `advance_noise`
                // call. Echo-write-disable (bit 5) is stored but has no
                // effect yet — the echo buffer itself isn't implemented
                // until a later stage. RESET and MUTE are handled here
                // and in render_audio_single/$4C respectively.
                0x6C => {
                    self.flg = value;
                    if value & 0x80 != 0 {
                        // RESET: hardware silences every voice immediately
                        // (not a Release fade — a hard cut) and clears
                        // ENDX. New key-ons stay blocked (see $4C) for as
                        // long as this bit remains set.
                        self.registers[0x7C] = 0;
                        for voice in self.voices.iter_mut() {
                            voice.key_on = false;
                            voice.adsr.envelope_phase = EnvelopePhase::Off;
                            voice.adsr.envelope_level = 0;
                        }
                    }
                }

                // Only pitch modulation (PMON, $2D) remains genuinely
                // unhandled — echo's registers are now stored above (see
                // the Stage 1 block), pitch mod isn't started yet.
                _ => {}
            },
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
        voice.history = [0; 4];

        // Clear this voice's bit in ENDX ($7C) so the CPU sees the new
        // key-on cleanly and doesn't mistake a leftover end flag for
        // the new sample having already finished.
        self.registers[0x7C] &= !(1u8 << v);
    }

    /// Advance the DSP by one output sample tick.
    ///
    /// `ram` is a direct slice of the 64 KB APU RAM. The DSP only reads
    /// from RAM (BRR sample data and the DIR table); it never writes to it.
    ///
    /// Takes `&RawARAM` rather than `&Memory` so the caller can pass
    /// `&memory.ram` without conflicting with the `&mut memory.dsp` borrow.
    pub fn step(&mut self, ram: &RawARAM) {
        self.advance_noise();
        // Real hardware scales the 15-bit LFSR into a signed sample the
        // same way a decoded BRR sample would be: shift left 1 and treat
        // as i16, so values above 0x4000 read as negative.
        let noise_sample = (self.noise_lfsr << 1) as i16;
        let non = self.non;

        // Split borrows so we can pass &mut voice and &mut self.registers
        // into Voice::step() simultaneously — the borrow checker allows
        // borrowing separate struct fields at the same time.
        let (voices, registers) = (&mut self.voices, &mut self.registers);

        for (i, voice) in voices.iter_mut().enumerate() {
            voice.step(i, ram, registers);

            if non & (1 << i) != 0 {
                // NON substitutes the noise generator for this voice's
                // decoded-sample source; envelope/volume/pan still apply
                // normally afterward via render_audio_single. BRR decoding
                // keeps running underneath regardless — real hardware
                // doesn't pause it — so clearing NON later resumes
                // wherever that voice's sample stream already got to.
                voice.current_sample = noise_sample;
                registers[(i << 4) | 0x9] = (noise_sample >> 8) as u8;
            }
        }
    }

    /// Advance the shared noise LFSR by one DSP tick, if the noise clock
    /// (FLG bits 0-4) is due to fire this tick. Gated by the same rate
    /// table and tick-counting pattern as ADSR envelope rates — index 0
    /// means "stopped," matching FLG's documented "0 = noise off."
    ///
    /// 15-bit LFSR, taps at bit0 and bit1: each fire, the new bit
    /// (bit0 XOR bit1 of the current value) is fed in at bit14 and the
    /// whole register shifts right by 1.
    fn advance_noise(&mut self) {
        let period = ENVELOPE_RATE_TABLE[(self.flg & 0x1F) as usize];
        if period == 0 {
            return;
        }

        self.noise_tick_counter += 1;
        if self.noise_tick_counter < period {
            return;
        }
        self.noise_tick_counter = 0;

        let feedback = ((self.noise_lfsr << 13) ^ (self.noise_lfsr << 14)) & 0x4000;
        self.noise_lfsr = feedback | (self.noise_lfsr >> 1);
    }

    /// Mix all active voices into one stereo output sample pair.
    ///
    /// Uses integer arithmetic throughout to match hardware behaviour.
    /// Volumes are signed i8; samples and envelope are 16-bit.
    /// The accumulator is i32 to prevent overflow during summation.
    pub fn render_audio_single(&self) -> (i16, i16) {
        // MUTE (bit6) and RESET (bit7, which forces mute too) silence the
        // final output stage only — voices keep decoding/enveloping
        // underneath (see `step`), they just aren't heard. This matches
        // hardware: unmuting mid-note resumes wherever playback got to,
        // it doesn't restart it.
        if self.flg & 0xC0 != 0 {
            return (0, 0);
        }

        let mut left: i32 = 0;
        let mut right: i32 = 0;

        for voice in self.voices.iter() {
            if voice.adsr.envelope_phase == EnvelopePhase::Off {
                continue;
            }

            // Scale sample by 11-bit envelope (0–0x7FF) → back to ~16-bit range
            let env = voice.adsr.envelope_level as i32; // 0–0x7FF
            let sample = voice.current_sample as i32; // -32768..+32767
            let scaled = (sample * env) >> 11; // ~16-bit result

            // Apply signed per-voice volumes (i8, -128..+127), shift by 7
            left += (scaled * voice.left_vol as i32) >> 7;
            right += (scaled * voice.right_vol as i32) >> 7;
        }

        // Apply master volume ($0C/$1C) as a final output stage scaler.
        // Same signed i8 × i32 → >> 7 pattern as per-voice volume.
        // A second clamp is required because master vol can amplify the
        // already-summed mix past i16 range again.
        left = (left * self.master_vol_left as i32) >> 7;
        right = (right * self.master_vol_right as i32) >> 7;

        (
            left.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            right.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        )
    }
}
