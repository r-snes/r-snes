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
        let index = (addr - 0xF200) as usize;
        self.registers[index] = value;

        match index {
            0x00..=0x07 => self.voices[index].left_vol = value,
            0x08..=0x0F => self.voices[index - 0x08].right_vol = value,
            0x10..=0x17 => {
                let voice_idx = index - 0x10;
                let pitch = (self.voices[voice_idx].pitch & 0xFF00) | value as u16;
                self.voices[voice_idx].pitch = pitch;
            }
            0x18..=0x1F => {
                let voice_idx = index - 0x18;
                let pitch = ((value as u16) << 8) | (self.voices[voice_idx].pitch & 0x00FF);
                self.voices[voice_idx].pitch = pitch;
            }
            0x20..=0x27 => {
                let voice_idx = index - 0x20;
                self.voices[voice_idx].key_on = value != 0;
                if value != 0 {
                    self.voices[voice_idx].current_addr = self.voices[voice_idx].sample_start;
                    self.voices[voice_idx].frac = 0;
                }
            }
            0x30..=0x37 => {
                let voice_idx = index - 0x30;
                self.voices[voice_idx].sample_start =
                    (self.voices[voice_idx].sample_start & 0xFF00) | value as u16;
            }
            0x38..=0x3F => {
                let voice_idx = index - 0x38;
                self.voices[voice_idx].sample_end =
                    (self.voices[voice_idx].sample_end & 0xFF00) | value as u16;
            }
            _ => {} // Other registers (echo, ADSR) not implemented yet
        }
    }

    /// Step DSP one tick (process voices)
    pub fn step(&mut self, mem: &Memory) {
        for voice in self.voices.iter_mut() {
            if !voice.key_on {
                continue;
            }
    
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
                        .clamp(-32768, 32767) as i16;
                    sample.1 = (sample.1 as i32 + sample_val * right_vol)
                        .clamp(-32768, 32767) as i16;
                }
            }
        }
        buffer
    }
}
