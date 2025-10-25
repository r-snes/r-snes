use crate::memory::Memory;

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
    pub adsr_mode: bool,     // whether ADSR or gain mode is used
    pub attack_rate: u8,
    pub decay_rate: u8,
    pub sustain_level: u8,
    pub release_rate: u8,
    pub envelope_level: u16, // current volume (0–0x7FF)
    pub envelope_phase: EnvelopePhase,
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

impl Voice {
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
                v.envelope_phase = EnvelopePhase::Attack;
                v.envelope_level = 0;
            }
        }

        // 0x28..=0x2F: Key Off
        // Writing a nonzero value releases the corresponding voice.
        0x28..=0x2F => {
            let voice_idx = index - 0x28;
            if value != 0 {
                let v = &mut self.voices[voice_idx];
                v.envelope_phase = EnvelopePhase::Release;
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
            v.adsr_mode = (value & 0x80) != 0; // Bit 7 enables ADSR
            v.attack_rate = (value >> 4) & 0x0F; // Bits 6–4
            v.decay_rate = value & 0x0F;         // Bits 3–0
        }

        // 0x60..=0x67: ADSR2 (Sustain level + Release rate)
        // Bits:
        // 7–5 - Sustain level (0–7)
        // 4–0 - Release rate (0–31)
        0x60..=0x67 => {
            let voice_idx = index - 0x60;
            let v = &mut self.voices[voice_idx];
            v.sustain_level = (value >> 5) & 0x07; // Bits 7–5
            v.release_rate = value & 0x1F;         // Bits 4–0
        }

        _ => {} // Other registers (echo, gain, FIR, etc.) not implemented yet
    }
}

    /// Step DSP one tick (process voices)
    pub fn step(&mut self, mem: &Memory) {
        for voice in self.voices.iter_mut() {
            if !voice.key_on {
                continue;
            }
    
            // Update the ADSR envelope each tick
            voice.update_envelope();

            // Advance fractional accumulator
            voice.frac = voice.frac.wrapping_add(voice.pitch);
            let step = voice.frac >> 8; // integer increment
            voice.frac &= 0xFF;         // keep fractional
    
            // Advance current address
            voice.current_addr = voice.current_addr.wrapping_add(step);
    
            // Stop if we reach the end of the sample
            if voice.current_addr >= voice.sample_end {
                voice.key_on = false;
            } else {
                // Fetch sample from memory after stepping
                voice.current_sample = mem.read8(voice.current_addr) as i8;
            }
        }
    }    

    /// Mix all voices into a stereo output buffer
    pub fn render_audio(&self, num_samples: usize) -> Vec<(i16, i16)> {
        let mut buffer = vec![(0i16, 0i16); num_samples];

        for voice in &self.voices {
            if voice.key_on {
                let left_vol = voice.left_vol as i32;
                let right_vol = voice.right_vol as i32;
                let sample_val = voice.current_sample as i32;

                for sample in &mut buffer {
                    sample.0 = (sample.0 as i32 + sample_val * left_vol)
                        .clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                    sample.1 = (sample.1 as i32 + sample_val * right_vol)
                        .clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                }
            }
        }
        buffer
    }
}
