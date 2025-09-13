use crate::memory::Memory;

/// One voice of the SNES APU DSP
#[derive(Default, Debug, Clone, Copy)]
pub struct Voice {
    pub left_vol: u8,
    pub right_vol: u8,
    pub pitch: u16,
    pub key_on: bool,
    pub sample_start: u16,
    pub sample_end: u16,
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

    pub fn step(&mut self) {
        // Here we would process voices (ADSR, echo, sample position, etc.)
    }

    pub fn render_audio(&self, num_samples: usize) -> Vec<i16> {
        vec![0; num_samples]
    }
}
