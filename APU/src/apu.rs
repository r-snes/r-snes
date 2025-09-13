use crate::{cpu::Spc700, dsp::Dsp, memory::Memory, timers::Timers};

pub struct Apu {
    pub cpu: Spc700,
    pub dsp: Dsp,
    pub memory: Memory,
    pub timers: Timers,
    pub cycles: u64,
}


impl Apu {
    pub fn new() -> Self {
        Self {
            cpu: Spc700::new(),
            dsp: Dsp::new(),
            memory: Memory::new(),
            timers: Timers::new(),
            cycles: 0,
        }
    }

    /// Step the APU forward by a given number of cycles
    pub fn step(&mut self, cycles: u32) {
        for _ in 0..cycles {
            self.cpu.step(&mut self.memory);
            self.timers.step(&mut self.memory);
            self.dsp.step();
            self.cycles += 1;
        }
    }

    /// Generate audio samples (stub for now)
    pub fn render_audio(&mut self, _num_samples: usize) -> Vec<i16> {
        // TODO: pull from DSP’s audio buffer
        vec![0; _num_samples]
    }
}
