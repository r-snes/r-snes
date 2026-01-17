use crate::memory::Memory;

/// ------------------------------------------------------------
/// ADSR ENVELOPE
/// Controls how loud a voice is over time
/// ------------------------------------------------------------

/// ADSR envelope state
#[derive(Debug, Clone, Copy)]
pub struct Adsr {
    pub adsr_mode: bool,     // whether ADSR or gain mode is used
    pub attack_rate: u8,     // how fast volume rises
    pub decay_rate: u8,      // how fast volume falls after attack
    pub sustain_level: u8,   // target level for sustain phase
    pub sustain_rate: u8,    // how fast volume falls during sustain
    pub release_rate: u8,    // how fast volume falls on key-off
    pub envelope_level: u16, // current volume (0–0x7FF)
    pub envelope_phase: EnvelopePhase, // current ADSR state
}

impl Default for Adsr {
    fn default() -> Self {
        Self {
            adsr_mode: false,
            attack_rate: 0,
            decay_rate: 0,
            sustain_level: 0,
            sustain_rate: 0,
            release_rate: 0,
            envelope_level: 0,
            envelope_phase: EnvelopePhase::Off,
        }
    }
}

impl Adsr {
    /// Update the ADSR envelope each DSP tick
    /// This simulates how the SNES changes volume over time
    pub fn update_envelope(&mut self) {
        match self.envelope_phase {
            EnvelopePhase::Attack => {
                // Attack phase: ramp volume up
                self.envelope_level =
                    self.envelope_level.saturating_add(self.attack_rate as u16 * 8);

                // When max is reached, move to decay
                if self.envelope_level >= 0x7FF {
                    self.envelope_level = 0x7FF;
                    self.envelope_phase = EnvelopePhase::Decay;
                }
            }

            EnvelopePhase::Decay => {
                // Decay phase: fall toward sustain level
                let target = self.sustain_level as u16 * 32;

                if self.envelope_level > target {
                    self.envelope_level =
                        self.envelope_level.saturating_sub(self.decay_rate as u16 * 2);
                } else {
                    self.envelope_phase = EnvelopePhase::Sustain;
                }
            }

            EnvelopePhase::Sustain => {
                // Sustain phase: slowly decrease volume
                self.envelope_level =
                    self.envelope_level.saturating_sub(self.sustain_rate as u16 * 2);

                // If we reach zero, the voice is effectively silent
                if self.envelope_level == 0 {
                    self.envelope_phase = EnvelopePhase::Off;
                }
            }

            EnvelopePhase::Release => {
                // Release phase: fixed speed fade-out
                const RELEASE_RATE: u16 = 8;

                self.envelope_level =
                    self.envelope_level.saturating_sub(RELEASE_RATE);

                if self.envelope_level == 0 {
                    self.envelope_phase = EnvelopePhase::Off;
                }
            }

            EnvelopePhase::Off => {
                // Do nothing: voice is silent
            }
        }
    }
}

/// Possible ADSR states
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EnvelopePhase {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}

/// ------------------------------------------------------------
/// BRR DECODER STATE (per voice)
/// Tracks where we are inside a compressed BRR sample
/// ------------------------------------------------------------

/// BRR decoding state (per voice)
#[derive(Debug, Clone, Copy)]
pub struct Brr {
    pub addr: u16,   // address of current BRR block in memory
    pub pos: u8,     // nibble index inside the block (0..15)
    pub prev1: i16,  // previous decoded sample
    pub prev2: i16,  // sample before that (needed for filters)
}

impl Default for Brr {
    fn default() -> Self {
        Self {
            addr: 0,
            pos: 0,
            prev1: 0,
            prev2: 0,
        }
    }
}

/// ------------------------------------------------------------
/// VOICE
/// Represents one of the 8 SNES APU channels
/// ------------------------------------------------------------

/// One voice (channel) of the SNES APU DSP
#[derive(Debug, Clone, Copy)]
pub struct Voice {
    pub left_vol: u8,   // left speaker volume (0..127)
    pub right_vol: u8,  // right speaker volume (0..127)
    pub pitch: u16,     // playback speed
    pub key_on: bool,   // whether the voice is active

    pub sample_start: u16, // BRR start address
    pub sample_end: u16,   // BRR end address (not fully used yet)

    pub current_addr: u16, // current memory position (unused for now)
    pub frac: u16,         // fractional accumulator for pitch stepping

    pub current_sample: i8, // last decoded sample (post-BRR, pre-mix)

    // ADSR sub-structure (volume envelope)
    pub adsr: Adsr,

    // BRR sub-structure (compressed audio state)
    pub brr: Brr,
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
            adsr: Adsr::default(),
            brr: Brr::default(),
        }
    }
}

/// ------------------------------------------------------------
/// BRR DECODING
/// ------------------------------------------------------------

/// Decode a single BRR nibble into a 16-bit PCM sample
/// This applies the shift and prediction filter
pub fn decode_brr_nibble(
    nibble: i8,
    shift: u8,
    filter: u8,
    prev1: i16,
    prev2: i16,
) -> i16 {
    // Basic expansion: shift 4-bit nibble to 16-bit range
    let mut sample = (nibble as i16) << shift;

    // Apply SNES BRR prediction filter
    let predicted = match filter {
        0 => 0,
        1 => prev1 - (prev1 >> 4),
        2 => (prev1 * 2) - ((prev1 * 3) >> 5) - prev2 + (prev2 >> 4),
        3 => (prev1 * 2) - ((prev1 * 13) >> 6) - prev2 + ((prev2 * 3) >> 4),
        _ => 0,
    };

    // Add prediction and clamp to valid range
    sample = sample.saturating_add(predicted);
    sample.clamp(-32768, 32767)
}

/// ------------------------------------------------------------
/// DSP CORE
/// ------------------------------------------------------------

pub struct Dsp {
    registers: [u8; 128], // $F200-$F27F
    pub voices: [Voice; 8], // 8 audio channels
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
        if addr < 0xF200 || (addr as usize) >= 0xF200 + self.registers.len() {
            panic!("Invalid DSP write address: {:#X}", addr);
        }

        let index = (addr - 0xF200) as usize;
        self.registers[index] = value;

        match index {
            // Left volume for voices 0-7
            0x00..=0x07 => self.voices[index].left_vol = value,

            // Right volume for voices 0-7
            0x08..=0x0F => self.voices[index - 0x08].right_vol = value,

            // Low byte of pitch
            0x10..=0x17 => {
                let v = &mut self.voices[index - 0x10];
                v.pitch = (v.pitch & 0xFF00) | value as u16;
            }

            // High byte of pitch
            0x18..=0x1F => {
                let v = &mut self.voices[index - 0x18];
                v.pitch = ((value as u16) << 8) | (v.pitch & 0x00FF);
            }

            // Key ON
            0x20..=0x27 => {
                let v = &mut self.voices[index - 0x20];
                v.key_on = value != 0;
                if value != 0 {
                    // Reset playback state when key-on
                    v.current_addr = v.sample_start;
                    v.frac = 0;
                    v.brr.addr = v.sample_start;
                    v.brr.pos = 0;
                    v.brr.prev1 = 0;
                    v.brr.prev2 = 0;
                    v.adsr.envelope_phase = EnvelopePhase::Attack;
                    v.adsr.envelope_level = 0;
                }
            }

            // Key OFF -> enter release phase
            0x28..=0x2F => {
                if value != 0 {
                    self.voices[index - 0x28].adsr.envelope_phase = EnvelopePhase::Release;
                }
            }

            // Sample start low byte
            0x30..=0x37 => {
                let v = &mut self.voices[index - 0x30];
                v.sample_start = (v.sample_start & 0xFF00) | value as u16;
            }

            // Sample end low byte
            0x38..=0x3F => {
                let v = &mut self.voices[index - 0x38];
                v.sample_end = (v.sample_end & 0xFF00) | value as u16;
            }

            // ADSR attack/decay config
            0x50..=0x57 => {
                let v = &mut self.voices[index - 0x50];
                v.adsr.adsr_mode   = (value & 0x80) != 0;
                v.adsr.attack_rate = (value >> 4) & 0x07;
                v.adsr.decay_rate  = value & 0x0F;
            }

            // ADSR sustain config
            0x60..=0x67 => {
                let v = &mut self.voices[index - 0x60];
                v.adsr.sustain_level = (value >> 5) & 0x07;
                v.adsr.sustain_rate  = value & 0x1F;
            }

            _ => {}
        }
    }

    /// Step DSP one tick
    /// - Updates ADSR
    /// - Advances BRR decoding if key_on
    pub fn step(&mut self, mem: &Memory) {
        for voice in self.voices.iter_mut() {
            // Update volume envelope
            if voice.adsr.envelope_phase != EnvelopePhase::Off {
                voice.adsr.update_envelope();
            }

            // Skip fully silent voices
            if !voice.key_on && voice.adsr.envelope_phase == EnvelopePhase::Off {
                continue;
            }

            // If playing, advance BRR decoding
            if voice.key_on {
                voice.frac = voice.frac.wrapping_add(voice.pitch);
                let step = voice.frac >> 8;
                voice.frac &= 0xFF;

                for _ in 0..step {
                    let header = mem.read8(voice.brr.addr);
                    let shift = header & 0x0F;
                    let filter = (header >> 4) & 0x03;

                    let byte = mem.read8(voice.brr.addr + 1 + (voice.brr.pos / 2) as u16);

                    let mut nibble = if voice.brr.pos & 1 == 0 {
                        ((byte >> 4) & 0x0F) as i8
                    } else {
                        (byte & 0x0F) as i8
                    };

                    if nibble & 0x08 != 0 {
                        nibble |= !0x0F;
                    }

                    let sample = decode_brr_nibble(
                        nibble,
                        shift,
                        filter,
                        voice.brr.prev1,
                        voice.brr.prev2,
                    );

                    // Update BRR history
                    voice.brr.prev2 = voice.brr.prev1;
                    voice.brr.prev1 = sample;

                    // Store sample (scaled down to 8-bit for mixing)
                    voice.current_sample = (sample >> 8) as i8;

                    // Advance nibble position
                    voice.brr.pos += 1;
                    if voice.brr.pos >= 16 {
                        voice.brr.pos = 0;
                        voice.brr.addr += 9; // next BRR block
                    }
                }
            }
        }
    }

    /// Mix all voices and return one stereo sample (left, right).
    pub fn render_audio_single(&self) -> (i16, i16) {
        let mut left_mix: f32 = 0.0;
        let mut right_mix: f32 = 0.0;

        for voice in self.voices.iter() {
            if voice.adsr.envelope_phase == EnvelopePhase::Off {
                continue;
            }

            let env = voice.adsr.envelope_level as f32 / 0x7FF as f32;
            let base = voice.current_sample as f32;
            let amp = base * env;

            left_mix  += amp * (voice.left_vol  as f32 / 127.0);
            right_mix += amp * (voice.right_vol as f32 / 127.0);
        }

        (
            left_mix.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
            right_mix.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
        )
    }
}
