//! The S-DSP: 8 voices, BRR decoding, ADSR/GAIN envelopes, noise, echo
//! and the final stereo mix.
//!
//! The SPC700 reaches the 128 DSP registers through the $F2 (address)
//! and $F3 (data) ports; see [`Dsp::read_reg`] for the register map.

mod adsr;
mod brr;
mod voice;

// Re-export everything tests and external code need
pub use adsr::{Adsr, EnvelopePhase};
pub use brr::{Brr, decode_brr_block, decode_brr_nibble};
pub use voice::Voice;

use adsr::ENVELOPE_RATE_TABLE;
use brr::ram_read8;
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
    ///   bit 5: disable echo *writes* — reading the echo buffer still
    ///          works either way (matches documented hardware behavior);
    ///          see `tick_echo_buffer`
    ///   bits 4-0: noise clock — index into the shared rate table
    ///             (ENVELOPE_RATE_TABLE; 0 = stopped). See `advance_noise`.
    flg: u8,

    /// $3D NON — one bit per voice; when set, that voice's mixed output
    /// is the shared noise generator instead of its BRR-decoded sample.
    /// BRR decoding keeps running underneath regardless (see `step`), so
    /// clearing NON resumes wherever that voice's sample stream got to.
    non: u8,

    /// $2D PMON — pitch modulation enable, one bit per voice. When bit N
    /// is set (N = 1–7), voice N's pitch is scaled each tick by voice
    /// N-1's post-envelope output (before L/R volume, so a modulator at
    /// volume 0 still modulates). Bit 0 is ignored by hardware — voice 0
    /// has no voice below it — and is masked off on write.
    pmon: u8,

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

    // ---- Echo registers ----
    /// $2C EVOLL / $3C EVOLR — echo output volume, signed (-128..+127).
    /// Scales the FIR-filtered echo before it joins the final mix, the
    /// same way MVOL scales the dry mix. 0 = echo processed (and written
    /// to the buffer) but not heard.
    echo_vol_left: i8,
    echo_vol_right: i8,

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
    /// size = edl * 512 stereo sample pairs (2 KB per unit). EDL=0 is
    /// not "no buffer": the pointer never advances, so the DSP reads and
    /// writes one 4-byte stereo pair at ESA every sample. A new value
    /// only takes effect when the pointer next wraps (see `echo_len`).
    edl: u8,

    /// $0F/$1F/.../$7F — the 8-tap FIR filter coefficients, signed. Real
    /// hardware quirk: these occupy the position that would be "voice N's
    /// GAIN+8" register for each voice 0-7 (reg offset 0xF), but they
    /// aren't per-voice data — together the 8 values form one global
    /// filter applied to the echo buffer. `fir_coeff[N]` is tap N,
    /// stored at register `N*0x10 + 0x0F`. Tap 0 weights the *oldest*
    /// entry of `fir_history`, tap 7 the newest.
    fir_coeff: [i8; 8],

    /// The last 8 stereo pairs read out of the echo buffer, oldest first
    /// (index 7 = the pair read this tick). The FIR filters this history,
    /// not the buffer itself, so every tap sees fully delayed samples.
    /// Stored halved (`>> 1`), as the hardware keeps them; the taps'
    /// `>> 6` makes up for it.
    fir_history: [(i16, i16); 8],

    /// Stage 2: byte offset of the echo buffer's read/write pointer,
    /// relative to the buffer's base (ESA*0x100). Advances by 4 (one
    /// stereo sample pair) each call to `tick_echo_buffer`, wrapping at
    /// `echo_len`. Not reset on ESA/EDL writes — real hardware doesn't
    /// clamp it until it naturally reaches the wraparound check either.
    echo_ptr: u16,

    /// Echo buffer length in bytes as actually in use: EDL*2048, latched
    /// from `edl` each time the pointer is at offset 0. Hardware only
    /// picks up a new EDL at that point, so a mid-pass $7D write lets the
    /// current pass finish at the old length. 0 (EDL=0) makes the pointer
    /// wrap straight back to 0 after every sample — a 4-byte buffer.
    echo_len: u16,

    /// Stage 4: this tick's FIR-filtered echo output, computed by `step`
    /// (which has the mutable RAM access `tick_echo` needs) and read by
    /// `render_audio_single` (which doesn't take RAM at all — it's a
    /// pure read of already-computed state). Split this way so
    /// `render_audio_single`'s signature doesn't have to change.
    echo_out_l: i16,
    echo_out_r: i16,
}

impl Default for Dsp {
    fn default() -> Self {
        Self::new()
    }
}

impl Dsp {
    /// Create a DSP with every register cleared, all voices silent and the
    /// noise generator seeded.
    ///
    /// Master volume starts at 0, so nothing is audible until the driver
    /// writes MVOLL/MVOLR ($0C/$1C).
    pub fn new() -> Self {
        // Hardware power-on state: FLG = $E0 — soft reset (bit 7: key-ons
        // blocked), mute (bit 6) and echo writes disabled (bit 5). The IPL
        // boot ROM never touches the DSP, so this holds until the sound
        // driver writes $6C itself. Mirrored into `registers` so a
        // read-back of $6C matches the state the DSP is really in.
        let mut registers = [0u8; 128];
        registers[0x6C] = 0xE0;

        Self {
            registers,
            voices: [Voice::default(); 8],
            dir_base: 0,
            // Hardware resets master volume to 0; game code sets it during boot.
            master_vol_left: 0,
            master_vol_right: 0,
            flg: 0xE0,
            non: 0,
            pmon: 0,
            noise_lfsr: 0x4000,
            noise_tick_counter: 0,
            echo_vol_left: 0,
            echo_vol_right: 0,
            efb: 0,
            eon: 0,
            esa: 0,
            edl: 0,
            fir_coeff: [0i8; 8],
            fir_history: [(0, 0); 8],
            echo_ptr: 0,
            echo_len: 0,
            echo_out_l: 0,
            echo_out_r: 0,
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

    /// The echo delay length as actually used (masked to its meaningful
    /// low 4 bits) — distinct from `read_reg(0x7D)`, which always returns
    /// the raw byte last written, mask or no mask. Same relationship as
    /// `voices[N].pitch` vs. `read_reg` for the PITCH-high register: the
    /// raw register readback is never masked, only the processed value
    /// used internally is.
    pub fn edl(&self) -> u8 {
        self.edl
    }

    /// The pitch-modulation enable mask as actually used (bit 0 masked
    /// off) — distinct from `read_reg(0x2D)`, which returns the raw byte
    /// last written. Same relationship as `edl()` vs. `read_reg(0x7D)`.
    pub fn pmon(&self) -> u8 {
        self.pmon
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
                0x4C if self.flg & 0x80 == 0 => {
                    for v in 0..8usize {
                        if value & (1 << v) != 0 {
                            self.key_on_voice(v);
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

                // $2D: PMON — pitch modulation, one bit per voice; see the
                // `pmon` field doc. Bit 0 is masked off (no effect on
                // hardware). The raw byte still lands in `registers`
                // above, so a read-back of $2D returns what was written.
                0x2D => self.pmon = value & 0xFE,

                // $2C/$3C: EVOLL/EVOLR — echo output volume (signed).
                0x2C => self.echo_vol_left = value as i8,
                0x3C => self.echo_vol_right = value as i8,

                // ---- Echo registers ----
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
                // Powers on as $E0 (see `new`). Noise clock changes take
                // effect on the next `advance_noise` call; echo-write-disable
                // (bit 5) is checked in `tick_echo_buffer`. RESET is handled
                // here and in $4C, MUTE in `render_audio_single`.
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

                // Everything else is stored in `registers` only (e.g. the
                // read-only ENVX/OUTX/ENDX slots, unused addresses).
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
    /// `ram` is a direct slice of the 64 KB APU RAM. Mutable as of Stage
    /// 4: the echo buffer (Stage 2/3) needs to write into it. Every
    /// other read (BRR sample data, the DIR table) still only reads.
    ///
    /// Takes `&mut RawARAM` rather than `&mut Memory` so the caller can
    /// pass `&mut memory.ram` without conflicting with the `&mut
    /// memory.dsp` borrow (disjoint fields of the same struct).
    pub fn step(&mut self, ram: &mut RawARAM) {
        self.advance_noise();
        // Real hardware scales the 15-bit LFSR into a signed sample the
        // same way a decoded BRR sample would be: shift left 1 and treat
        // as i16, so values above 0x4000 read as negative.
        let noise_sample = (self.noise_lfsr << 1) as i16;
        let non = self.non;
        let eon = self.eon;
        let pmon = self.pmon;

        // Split borrows so we can pass &mut voice and &mut self.registers
        // into Voice::step() simultaneously — the borrow checker allows
        // borrowing separate struct fields at the same time.
        let (voices, registers) = (&mut self.voices, &mut self.registers);

        let mut echo_in_l: i32 = 0;
        let mut echo_in_r: i32 = 0;

        // Voice i-1's post-envelope output from *this* tick, carried
        // forward through the loop for pitch modulation.
        let mut prev_output: i32 = 0;

        for (i, voice) in voices.iter_mut().enumerate() {
            let pmon_source = if pmon & (1 << i) != 0 {
                Some(prev_output)
            } else {
                None
            };

            // Voice::step only reads RAM; reborrow the mutable reference
            // as shared for the duration of this call.
            voice.step(i, ram, registers, pmon_source);

            if non & (1 << i) != 0 {
                // NON substitutes the noise generator for this voice's
                // decoded-sample source; envelope/volume/pan still apply
                // normally afterward via render_audio_single. BRR decoding
                // keeps running underneath regardless — real hardware
                // doesn't pause it — so clearing NON later resumes
                // wherever that voice's sample stream already got to.
                voice.current_sample = noise_sample;
            }

            // This voice's post-envelope, pre-volume output. It is the
            // value that modulates voice i+1 if PMON selects it, and the
            // value OUTX reports.
            prev_output = if voice.adsr.envelope_phase == EnvelopePhase::Off {
                0
            } else {
                let env = voice.adsr.envelope_level as i32;
                ((voice.current_sample as i32 * env) >> 11) & !1
            };

            // OUTX ($X9): the signed top byte of that output. Written for
            // every voice on every tick, so an idle voice reads back 0.
            registers[(i << 4) | 0x9] = (prev_output >> 8) as u8;

            if eon & (1 << i) != 0 {
                // EON sums this voice's dry output (post-NON, so a
                // voice with both NON and EON set feeds its noise into
                // the echo buffer too, not the BRR audio underneath —
                // both stages see the same "this voice's output this
                // tick" value) into the echo input, same per-voice
                // scaling as the main dry mix.
                let (l, r) = voice_dry_output(voice);
                echo_in_l += l;
                echo_in_r += r;
            }
        }

        let echo_in_l = echo_in_l.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let echo_in_r = echo_in_r.clamp(i16::MIN as i32, i16::MAX as i32) as i16;

        let (echo_out_l, echo_out_r) = self.tick_echo(ram, echo_in_l, echo_in_r);
        self.echo_out_l = echo_out_l;
        self.echo_out_r = echo_out_r;
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

    /// Absolute APU RAM address of a byte offset within the echo buffer.
    /// Wraps at 64 KB like every other APU RAM address does on real
    /// hardware — ESA near the top of the address space plus a large
    /// offset (ESA=$FF with EDL=15 can reach past $FFFF) genuinely
    /// overflows a plain `u16` add, so this must wrap, not panic.
    fn echo_addr(&self, offset: u16) -> u16 {
        (self.esa as u16).wrapping_mul(0x100).wrapping_add(offset)
    }

    /// Advance the echo buffer by one tick: read the stereo sample pair
    /// (L, R) currently at the echo pointer, write `(new_l, new_r)` to
    /// that same position (unless FLG bit 5 disables echo writes —
    /// reading is unaffected either way), then advance the pointer by
    /// one stereo pair (4 bytes), wrapping at the buffer length.
    ///
    /// Returns the pair that was read *before* the write, i.e. the
    /// sample the buffer held from one full trip around ago.
    ///
    /// The length is latched from EDL whenever the pointer is at offset 0,
    /// as on hardware. EDL=0 latches a length of 0, so the pointer wraps
    /// straight back to 0 every tick: the DSP keeps reading and writing
    /// the single pair at ESA. Echo-write-disable (FLG bit 5, set at
    /// power-on) is what keeps that from touching RAM before a driver
    /// has chosen where its buffer goes.
    pub fn tick_echo_buffer(&mut self, ram: &mut RawARAM, new_l: i16, new_r: i16) -> (i16, i16) {
        if self.echo_ptr == 0 {
            self.echo_len = self.edl as u16 * 2048;
        }

        let addr = self.echo_addr(self.echo_ptr);
        let old_l = read_echo_sample(ram, addr);
        let old_r = read_echo_sample(ram, addr.wrapping_add(2));

        // FLG bit 5 set = writes disabled; reading above happens either way.
        if self.flg & 0x20 == 0 {
            write_echo_sample(ram, addr, new_l);
            write_echo_sample(ram, addr.wrapping_add(2), new_r);
        }

        self.echo_ptr += 4;
        if self.echo_ptr >= self.echo_len {
            self.echo_ptr = 0;
        }

        (old_l, old_r)
    }

    /// Filter the FIR history with the 8 coefficients, as the hardware
    /// does: taps 0-6 are summed and truncated to 16 bits (wrapping, not
    /// clamping), tap 7 is added, then the result is clamped and its
    /// lowest bit cleared.
    fn fir_filter(&self) -> (i16, i16) {
        let channel = |pick: fn(&(i16, i16)) -> i16| -> i16 {
            let tap =
                |i: usize| (pick(&self.fir_history[i]) as i32 * self.fir_coeff[i] as i32) >> 6;
            let mut sum: i32 = (0..7).map(tap).sum();
            sum = sum as i16 as i32;
            sum += tap(7) as i16 as i32;
            (sum.clamp(i16::MIN as i32, i16::MAX as i32) as i16) & !1
        };
        (channel(|p| p.0), channel(|p| p.1))
    }

    /// Advance the echo processor by one tick and return the filtered
    /// echo output (before EVOL):
    ///
    /// 1. read the pair at the echo pointer into the FIR history,
    /// 2. filter the history,
    /// 3. write `echo_in + filtered * EFB` back at the same position and
    ///    advance the pointer (`tick_echo_buffer`).
    ///
    /// Because the FIR only ever sees samples read out of the buffer,
    /// every tap is delayed by the full buffer length (EDL * 512 samples).
    pub fn tick_echo(&mut self, ram: &mut RawARAM, echo_in_l: i16, echo_in_r: i16) -> (i16, i16) {
        let addr = self.echo_addr(self.echo_ptr);
        let read_l = read_echo_sample(ram, addr);
        let read_r = read_echo_sample(ram, addr.wrapping_add(2));
        self.fir_history.rotate_left(1);
        self.fir_history[7] = (read_l >> 1, read_r >> 1);

        let (fir_l, fir_r) = self.fir_filter();

        let write_l = echo_feedback(echo_in_l, fir_l, self.efb);
        let write_r = echo_feedback(echo_in_r, fir_r, self.efb);
        self.tick_echo_buffer(ram, write_l, write_r);

        (fir_l, fir_r)
    }

    /// Mix all active voices into one stereo output sample pair.
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
            let (l, r) = voice_dry_output(voice);
            left += l;
            right += r;
        }

        // Apply master volume ($0C/$1C) as a final output stage scaler.
        // Same signed i8 × i32 → >> 7 pattern as per-voice volume.
        left = (left * self.master_vol_left as i32) >> 7;
        right = (right * self.master_vol_right as i32) >> 7;

        // Echo joins the mix scaled by its own volume, EVOL ($2C/$3C),
        // the same signed i8 × i32 → >> 7 pattern as MVOL.
        left += (self.echo_out_l as i32 * self.echo_vol_left as i32) >> 7;
        right += (self.echo_out_r as i32 * self.echo_vol_right as i32) >> 7;

        // A second clamp is required: master vol can amplify the
        // already-summed dry mix past i16 range.
        (
            left.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            right.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        )
    }
}

fn voice_dry_output(voice: &Voice) -> (i32, i32) {
    if voice.adsr.envelope_phase == EnvelopePhase::Off {
        return (0, 0);
    }

    // Scale sample by 11-bit envelope (0–0x7FF) → back to ~16-bit range
    let env = voice.adsr.envelope_level as i32; // 0–0x7FF
    let sample = voice.current_sample as i32; // -32768..+32767
    let scaled = (sample * env) >> 11; // ~16-bit result

    // Apply signed per-voice volumes (i8, -128..+127), shift by 7
    let left = (scaled * voice.left_vol as i32) >> 7;
    let right = (scaled * voice.right_vol as i32) >> 7;
    (left, right)
}

/// The value written back into the echo buffer: the echo input plus the
/// filtered echo scaled by EFB (truncated to 16 bits, as on hardware),
/// clamped, with the lowest bit cleared.
fn echo_feedback(echo_in: i16, fir: i16, efb: i8) -> i16 {
    let feedback = ((fir as i32 * efb as i32) >> 7) as i16 as i32;
    ((echo_in as i32 + feedback).clamp(i16::MIN as i32, i16::MAX as i32) as i16) & !1
}

/// Read a little-endian 16-bit signed sample from APU RAM, matching the
/// echo buffer's on-hardware storage format (same byte order as every
/// other 16-bit quantity in APU RAM — see `Memory::read16`).
fn read_echo_sample(ram: &RawARAM, addr: u16) -> i16 {
    let lo = ram_read8(ram, addr) as u16;
    let hi = ram_read8(ram, addr.wrapping_add(1)) as u16;
    ((hi << 8) | lo) as i16
}

/// Write a little-endian 16-bit signed sample to APU RAM. Every `u16`
/// address is already a valid index into the full 64 KB `RawARAM`, so
/// unlike `ram_read8` this needs no bounds fallback.
fn write_echo_sample(ram: &mut RawARAM, addr: u16, value: i16) {
    let bytes = (value as u16).to_le_bytes();
    ram[addr as usize] = bytes[0];
    ram[addr.wrapping_add(1) as usize] = bytes[1];
}
