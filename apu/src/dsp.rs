use crate::memory::Memory;

/// ADSR envelope state (moved out of Voice)
#[derive(Debug, Clone, Copy)]
pub struct Adsr {
    pub adsr_mode: bool,     // whether ADSR or gain mode is used
    pub attack_rate: u8,
    pub decay_rate: u8,
    pub sustain_level: u8,
    pub release_rate: u8,
    pub envelope_level: u16, // current volume (0–0x7FF)
    pub envelope_phase: EnvelopePhase,
}

impl Default for Adsr {
    fn default() -> Self {
        Self {
            // ADSR defaults
            adsr_mode: false,
            attack_rate: 0,
            decay_rate: 0,
            sustain_level: 0,
            release_rate: 0,
            envelope_level: 0,
            envelope_phase: EnvelopePhase::Off,
        }
    }
}

impl Adsr {
    /// Update the ADSR envelope each tick
    pub fn update_envelope(&mut self) {
        match self.envelope_phase {
            EnvelopePhase::Attack => {
                self.envelope_level =
                    self.envelope_level.saturating_add(self.attack_rate as u16 * 8);
                if self.envelope_level >= 0x7FF {
                    self.envelope_level = 0x7FF;
                    self.envelope_phase = EnvelopePhase::Decay;
                }
            }

            EnvelopePhase::Decay => {
                let target = (self.sustain_level as u16) * 0x100 / 8;
                if self.envelope_level > target {
                    self.envelope_level =
                        self.envelope_level.saturating_sub(self.decay_rate as u16 * 2);
                } else {
                    self.envelope_phase = EnvelopePhase::Sustain;
                }
            }

            EnvelopePhase::Sustain => {
                // Sustain phase: hold current envelope level
            }

            EnvelopePhase::Release => {
                self.envelope_level =
                    self.envelope_level.saturating_sub(self.release_rate as u16 * 4);
                if self.envelope_level == 0 {
                    self.envelope_phase = EnvelopePhase::Off;
                }
            }

            EnvelopePhase::Off => {
                // Silence, no change
            }
        }
    }
}

/// Voice of the SNES APU DSP
#[derive(Debug, Clone, Copy)]
pub struct Voice {
    pub left_vol: u8,
    pub right_vol: u8,
    pub pitch: u16,
    pub key_on: bool,
    pub sample_start: u16,
    pub sample_end: u16,
    pub current_addr: u16,
    pub frac: u16,          // fractional accumulator for pitch stepping
    pub current_sample: i8, // last fetched sample

    // ADSR moved into sub-structure
    pub adsr: Adsr,
}

#[derive(Copy, Clone,Debug, PartialEq)]
pub enum EnvelopePhase {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            left_vol: 0,
            right_vol: 0,
            pitch: 0,
            key_on: false,
            sample_start: 0,
            sample_end: 0,
            current_addr: 0,
            frac: 0,
            current_sample: 0,

            // ADSR defaults (unchanged comment)
            adsr: Adsr::default(),
        }
    }
}

impl Voice {
    /// Update the ADSR envelope each tick
    pub fn update_envelope(&mut self) {
        self.adsr.update_envelope();
    }
}

pub struct Dsp {
    registers: [u8; 128], // $F200-$F27F
    pub voices: [Voice; 8],
}

impl Dsp {
    pub fn new() -> Self {
        Self {
            registers: [0; 128],
            voices: [Voice::default(); 8],
        }
    }

    /// Read DSP register (memory-mapped)
    pub fn read(&self, addr: u16) -> u8 {
        let index = (addr - 0xF200) as usize;
        self.registers[index]
    }

    /// Write DSP register (memory-mapped)
    pub fn write(&mut self, addr: u16, value: u8) {
        // Make sure addr is in the valid DSP register range
        if addr < 0xF200 || (addr as usize) >= 0xF200 + self.registers.len() {
            // return could be used instead of panicking if more verif is added
            panic!("Invalid DSP write address: {:#X}", addr);
        }

        let index = (addr - 0xF200) as usize;
        self.registers[index] = value;

        match index {
            // 0x00..=0x07: Left volume for each voice
            // Registers 0x00..0x07 directly map to the `left_vol` field of each voice.
            0x00..=0x07 => self.voices[index].left_vol = value,

            // 0x08..=0x0F: Right volume for each voice
            // Same idea as left volume, but offset by 8.
            0x08..=0x0F => self.voices[index - 0x08].right_vol = value,

            // 0x10..=0x17: Pitch low byte
            // Sets the low 8 bits of the pitch value.
            // The high 8 bits are updated separately at 0x18..=0x1F.
            0x10..=0x17 => {
                let voice_idx = index - 0x10;
                let pitch = (self.voices[voice_idx].pitch & 0xFF00) | value as u16;
                self.voices[voice_idx].pitch = pitch;
            }

            // 0x18..=0x1F: Pitch high byte
            // Sets the high 8 bits of the pitch value.
            0x18..=0x1F => {
                let voice_idx = index - 0x18;
                let pitch = ((value as u16) << 8) | (self.voices[voice_idx].pitch & 0x00FF);
                self.voices[voice_idx].pitch = pitch;
            }

            // 0x20..=0x27: Key On
            // Writing a nonzero value starts playback for the corresponding voice.
            // This also resets the current address to `sample_start`
            // and clears the fractional accumulator.
            // Writing zero turns the voice off.
            0x20..=0x27 => {
                let voice_idx = index - 0x20;
                self.voices[voice_idx].key_on = value != 0;
                if value != 0 {
                    let v = &mut self.voices[voice_idx];
                    v.current_addr = v.sample_start;
                    v.frac = 0;
                    v.adsr.envelope_phase = EnvelopePhase::Attack;
                    v.adsr.envelope_level = 0;
                }
            }

            // 0x28..=0x2F: Key Off
            // Writing a nonzero value releases the corresponding voice.
            0x28..=0x2F => {
                let voice_idx = index - 0x28;
                if value != 0 {
                    let v = &mut self.voices[voice_idx];
                    v.adsr.envelope_phase = EnvelopePhase::Release;
                }
            }

            // 0x30..=0x37: Sample Start (low byte)
            // Sets the low 8 bits of the sample start address.
            0x30..=0x37 => {
                let voice_idx = index - 0x30;
                self.voices[voice_idx].sample_start =
                    (self.voices[voice_idx].sample_start & 0xFF00) | value as u16;
            }

            // 0x38..=0x3F: Sample End (low byte)
            // Sets the low 8 bits of the sample end address.
            0x38..=0x3F => {
                let voice_idx = index - 0x38;
                self.voices[voice_idx].sample_end =
                    (self.voices[voice_idx].sample_end & 0xFF00) | value as u16;
            }

            // 0x50..=0x57: ADSR1 (Attack, Decay, ADSR enable)
            // Bits:
            // 7 - ADSR enable
            // 6–4 - Attack rate (0–15)
            // 3–0 - Decay rate (0–15)
            0x50..=0x57 => {
                let voice_idx = index - 0x50;
                let v = &mut self.voices[voice_idx];
                v.adsr.adsr_mode = (value & 0x80) != 0; // Bit 7 enables ADSR
                v.adsr.attack_rate   = (value >> 4) & 0x07;  // mask 3 bits
                v.adsr.decay_rate    = value & 0x0F;
            }

            // 0x60..=0x67: ADSR2 (Sustain level + Release rate)
            // Bits:
            // 7–5 - Sustain level (0–7)
            // 4–0 - Release rate (0–31)
            0x60..=0x67 => {
                let voice_idx = index - 0x60;
                let v = &mut self.voices[voice_idx];
                v.adsr.sustain_level = (value >> 5) & 0x07; // Bits 7–5
                v.adsr.release_rate = value & 0x1F;         // Bits 4–0
            }

            _ => {} // Other registers (echo, gain, FIR, etc.) not implemented yet
        }
    }

    /// Step DSP one tick (process voices + ADSR)
    pub fn step(&mut self, mem: &Memory) {
        for voice in self.voices.iter_mut() {
            // FIRST: Update the envelope (as long as the voice is active or releasing)
            if voice.adsr.envelope_phase != EnvelopePhase::Off {
                voice.adsr.update_envelope();
            }

            // If key is not active AND envelope is Off — voice is fully silent
            if !voice.key_on && voice.adsr.envelope_phase == EnvelopePhase::Off {
                continue;
            }

            // Only advance sample position if key_on is true
            if voice.key_on {
                // Advance fractional accumulator
                voice.frac = voice.frac.wrapping_add(voice.pitch);
                let step = voice.frac >> 8; // integer increment
                voice.frac &= 0xFF;         // keep fractional

                // Advance sample address
                voice.current_addr = voice.current_addr.wrapping_add(step);

                // Reached end of sample?
                if voice.current_addr >= voice.sample_end {
                    // Trigger release
                    voice.key_on = false;
                    voice.adsr.envelope_phase = EnvelopePhase::Release;
                } else {
                    // Fetch new sample
                    voice.current_sample = mem.read8(voice.current_addr) as i8;
                }
            }
        }
    }

    /// Mix all voices into a stereo output buffer
    pub fn render_audio(&mut self, out: &mut [(i16, i16)]) {
        for sample in out.iter_mut() {
            let mut left_mix: f32 = 0.0;
            let mut right_mix: f32 = 0.0;

            for voice in self.voices.iter() {
                if voice.adsr.envelope_phase == EnvelopePhase::Off {
                    continue;
                }

                // Envelope fraction 0.0 – 1.0
                let env = voice.adsr.envelope_level as f32 / 0x7FF as f32;

                // Raw sample -128..127 → convert to float
                let base = voice.current_sample as f32;

                // Apply envelope
                let amp = base * env;

                // Apply left/right volumes (0..127)
                left_mix  += amp * (voice.left_vol  as f32 / 127.0);
                right_mix += amp * (voice.right_vol as f32 / 127.0);
            }

            // Convert float → i16 with clamping
            sample.0 = left_mix.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            sample.1 = right_mix.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        }
    }
}
