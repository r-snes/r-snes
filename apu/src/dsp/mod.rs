mod adsr;
mod brr;
mod voice;

// Re-export everything tests and external code need
pub use adsr::{Adsr, EnvelopePhase};
pub use brr::{Brr, decode_brr_nibble, decode_brr_block};
pub use voice::Voice;

use adsr::ENVELOPE_RATE_TABLE;
use brr::ram_read8;

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
}

impl Dsp {
    pub fn new() -> Self {
        Self {
            registers: [0u8; 128],
            voices: [Voice::default(); 8],
            dir_base: 0,
            // Hardware resets master volume to 0; game code sets it during boot.
            master_vol_left:  0,
            master_vol_right: 0,
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

    /// Write a DSP register by its 7-bit index and update internal state.
    pub fn write_reg(&mut self, index: u8, value: u8) {
        let idx = (index & 0x7F) as usize;
        self.registers[idx] = value;

        let voice_num = idx >> 4;   // high nibble = voice 0–7
        let reg_off   = idx & 0x0F; // low nibble  = register within voice block

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
                *p = (*p & 0x3F00) | (value as u16);
            }

            // +3: PITCH high byte (only bits 5-0 = pitch bits 13-8)
            (v, 0x3) => {
                let p = &mut self.voices[v].pitch;
                *p = (*p & 0x00FF) | ((value as u16 & 0x3F) << 8);
            }

            // +4: SRCN — sample source number (index into DIR table)
            (v, 0x4) => self.voices[v].srcn = value,

            // +5: ADSR1 = EDDDAAAA
            //   bit 7:    ADSR enable (1=ADSR, 0=GAIN)
            //   bits 6-4: decay rate index (0–7)
            //   bits 3-0: attack rate index (0–15)
            (v, 0x5) => {
                let adsr = &mut self.voices[v].adsr;
                adsr.adsr_mode   = (value & 0x80) != 0;
                adsr.decay_rate  = (value >> 4) & 0x07;
                adsr.attack_rate =  value & 0x0F;
            }

            // +6: ADSR2 = SSSRRRRR
            //   bits 7-5: sustain level (0–7)
            //   bits 4-0: sustain rate index (0–31)
            (v, 0x6) => {
                let adsr = &mut self.voices[v].adsr;
                adsr.sustain_level = (value >> 5) & 0x07;
                adsr.sustain_rate  =  value & 0x1F;
            }

            // +7: GAIN — TODO: implement GAIN mode
            (v, 0x7) => todo!("GAIN mode"),

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

            // $0C: MVOLL — master left  volume (signed)
            (_, _) if idx == 0x0C => self.master_vol_left  = value as i8,

            // $1C: MVOLR — master right volume (signed)
            (_, _) if idx == 0x1C => self.master_vol_right = value as i8,

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
        voice.brr.nibble_idx  = 0;
        voice.brr.prev1       = 0;
        voice.brr.prev2       = 0;
        voice.brr.buffer_fill = 0;
        voice.brr.loop_addr   = 0;

        // Reset pitch counter
        voice.pitch_counter = 0;

        // Reset envelope to start of attack
        voice.adsr.envelope_phase = EnvelopePhase::Attack;
        voice.adsr.envelope_level = 0;
        voice.adsr.tick_counter   = 0;

        voice.current_sample = 0;

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
    /// Takes `&[u8]` rather than `&Memory` so the caller can pass
    /// `&memory.ram` without conflicting with the `&mut memory.dsp` borrow.
    pub fn step(&mut self, ram: &[u8]) {
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

            // 2. Resolve DIR table on first tick after key-on.
            // buffer_fill == 0 means we haven't decoded anything yet.
            if self.voices[v].brr.buffer_fill == 0 {
                let dir_entry = self.voices[v].brr.addr;

                let start_lo = ram_read8(ram, dir_entry)     as u16;
                let start_hi = ram_read8(ram, dir_entry + 1) as u16;
                let loop_lo  = ram_read8(ram, dir_entry + 2) as u16;
                let loop_hi  = ram_read8(ram, dir_entry + 3) as u16;

                self.voices[v].brr.addr      = (start_hi << 8) | start_lo;
                self.voices[v].brr.loop_addr = (loop_hi  << 8) | loop_lo;

                self.decode_next_block(v, ram);
            }

            // 3. Pitch counter advance.
            // Every 0x1000 units = one BRR sample consumed.
            let pitch = self.voices[v].pitch & 0x3FFF;
            self.voices[v].pitch_counter =
                self.voices[v].pitch_counter.wrapping_add(pitch);

            let samples_to_consume = self.voices[v].pitch_counter / 0x1000;
            self.voices[v].pitch_counter %= 0x1000;

            // 4. Consume decoded samples from buffer.
            for _ in 0..samples_to_consume {
                let idx = self.voices[v].brr.nibble_idx as usize;
                if idx < self.voices[v].brr.buffer_fill as usize {
                    self.voices[v].current_sample =
                        self.voices[v].brr.sample_buffer[idx];
                    self.voices[v].brr.nibble_idx += 1;
                }

                if self.voices[v].brr.nibble_idx >= self.voices[v].brr.buffer_fill {
                    self.voices[v].brr.nibble_idx = 0;
                    self.decode_next_block(v, ram);
                    if !self.voices[v].key_on {
                        break;
                    }
                }
            }

            // 5. Update read-only ENVX ($X8) and OUTX ($X9) registers.
            //   ENVX = envelope_level >> 4  (11-bit → 7-bit)
            //   OUTX = current_sample  >> 8 (signed top byte)
            let envx_idx = (v << 4) | 0x8;
            let outx_idx = (v << 4) | 0x9;
            self.registers[envx_idx] =
                (self.voices[v].adsr.envelope_level >> 4) as u8;
            self.registers[outx_idx] =
                (self.voices[v].current_sample >> 8) as u8;
        }
    }

    /// Decode the next 9-byte BRR block for voice `v` and advance the address.
    ///
    /// Handles end/loop flags:
    /// - end=true,  loop=true  → jump to loop_addr and continue
    /// - end=true,  loop=false → silence the voice (enter release)
    /// - end=false             → advance address by 9 bytes
    fn decode_next_block(&mut self, v: usize, ram: &[u8]) {
        let voice = &mut self.voices[v];

        let (samples, end, do_loop) = decode_brr_block(
            ram,
            voice.brr.addr,
            &mut voice.brr.prev1,
            &mut voice.brr.prev2,
        );

        voice.brr.sample_buffer = samples;
        voice.brr.buffer_fill   = 16;
        voice.brr.nibble_idx    = 0;

        if end {
            // Set ENDX bit so the CPU can detect sample completion.
            self.registers[0x7C] |= 1u8 << v;

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

        // Apply master volume ($0C/$1C) as a final output stage scaler.
        // Same signed i8 × i32 → >> 7 pattern as per-voice volume.
        // A second clamp is required because master vol can amplify the
        // already-summed mix past i16 range again.
        left  = (left  * self.master_vol_left  as i32) >> 7;
        right = (right * self.master_vol_right as i32) >> 7;

        (
            left .clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            right.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        )
    }
}
