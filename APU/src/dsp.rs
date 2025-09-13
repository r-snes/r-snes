/// One voice of the SNES APU DSP
#[derive(Default, Debug, Clone, Copy)]
pub struct Voice {
    pub left_vol: u8,
    pub right_vol: u8,
    pub pitch: u16,
    pub key_on: bool,
    pub sample_start: u16,
    pub current_addr: u16,
    pub sample_end: u16,
    pub current_sample: i16,
}

pub struct Dsp {
    registers: [u8; 128], // $F200-$F27F
    pub voices: [Voice; 8],
}

impl Dsp {
    pub fn new() -> Self {
        Self {
            registers: [0; 128],
            voices: [Voice {
                key_on: false,
                sample_start: 0,
                sample_end: 0,
                current_addr: 0,
                pitch: 0,
                left_vol: 0,
                right_vol: 0,
                current_sample: 0,
            }; 8],
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

        // Map registers to voices
        match index {
            0x00..=0x07 => {
                // Left volume
                self.voices[index as usize].left_vol = value;
            }
            0x08..=0x0F => {
                // Right volume
                let voice_idx = index - 0x08;
                self.voices[voice_idx].right_vol = value;
            }
            0x10..=0x17 => {
                // Pitch low byte
                let voice_idx = index - 0x10;
                let pitch = (self.voices[voice_idx].pitch & 0xFF00) | value as u16;
                self.voices[voice_idx].pitch = pitch;
            }
            0x18..=0x1F => {
                // Pitch high byte
                let voice_idx = index - 0x18;
                let pitch = (value as u16) << 8 | (self.voices[voice_idx].pitch & 0x00FF);
                self.voices[voice_idx].pitch = pitch;
            }
            0x20..=0x27 => {
                // Key on/off
                let voice_idx = index - 0x20;
                self.voices[voice_idx].key_on = value != 0;
            }
            0x30..=0x37 => {
                // Sample start
                let voice_idx = index - 0x30;
                self.voices[voice_idx].sample_start = (self.voices[voice_idx].sample_start & 0xFF00) | value as u16;
            }
            0x38..=0x3F => {
                // Sample end
                let voice_idx = index - 0x38;
                self.voices[voice_idx].sample_end = (self.voices[voice_idx].sample_end & 0xFF00) | value as u16;
            }
            _ => {
                // Other registers (echo, ADSR, etc.) not implemented yet
            }
        }
    }

    pub fn step(&mut self, memory: &crate::memory::Memory) {
        for voice in self.voices.iter_mut() {
            if voice.key_on {
                // Fetch the sample from memory (8-bit unsigned -> i16 signed)
                let sample_byte = memory.read8(voice.current_addr);
                voice.current_sample = (sample_byte as i16) << 8; // simple conversion to signed
    
                // Advance current address by pitch
                voice.current_addr = voice.current_addr.wrapping_add(voice.pitch);
    
                // Stop if we reach end of sample
                if voice.current_addr >= voice.sample_end {
                    voice.key_on = false;
                }
            }
        }
    }

    pub fn render_audio(&self, num_samples: usize) -> Vec<i16> {
        let mut buffer = vec![0i16; num_samples * 2]; // stereo interleaved: L/R
        for voice in self.voices.iter() {
            if voice.key_on {
                let l = voice.left_vol as i16;
                let r = voice.right_vol as i16;
                for i in 0..num_samples {
                    buffer[i*2] = buffer[i*2].saturating_add(l * (voice.current_sample >> 8));
                    buffer[i*2+1] = buffer[i*2+1].saturating_add(r * (voice.current_sample >> 8));
                }
            }
        }
        buffer
    }
    
}
