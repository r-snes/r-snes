use crate::{cpu::Spc700, memory::Memory, timers::Timers};

// The SPC700 CPU runs at 1.024 MHz.
// The DSP produces one output sample every 32 CPU cycles (32 kHz).
// We count CPU cycles and only tick the DSP when this threshold is reached.
const DSP_CYCLES_PER_SAMPLE: u32 = 32;

pub struct Apu {
    pub cpu:    Spc700,
    pub memory: Memory,   // Memory::dsp is the *only* Dsp — there is no separate field
    pub timers: Timers,

    /// Total CPU cycles elapsed since APU creation.
    pub cycles: u64,

    /// Counts CPU cycles since the last DSP tick.
    /// Resets to 0 every DSP_CYCLES_PER_SAMPLE cycles.
    dsp_cycles: u32,
}

impl Apu {
    pub fn new() -> Self {
        let mut apu = Self {
            cpu:        Spc700::new(),
            memory:     Memory::new(),
            timers:     Timers::new(),
            cycles:     0,
            dsp_cycles: 0,
        };

        // Load the reset vector and initialise SP so the CPU starts correctly.
        apu.cpu.reset(&mut apu.memory);

        apu
    }

    /// Step the APU forward by `cycles` CPU cycles.
    ///
    /// Each call ticks:
    ///   - The SPC700 CPU  (every cycle)
    ///   - The timers      (every cycle)
    ///   - The DSP         (once every 32 cycles → 32 kHz)
    ///
    /// All DSP access goes through `self.memory.dsp`; there is no
    /// separate Dsp field on Apu.
    pub fn step(&mut self, cycles: u32) {
        for _ in 0..cycles {
            self.cpu.step(&mut self.memory);
            self.timers.step(&mut self.memory);

            self.dsp_cycles += 1;
            if self.dsp_cycles >= DSP_CYCLES_PER_SAMPLE {
                self.dsp_cycles = 0;

                // dsp.step() needs to read BRR sample data from APU RAM, but
                // Memory owns the Dsp, so we cannot pass &self.memory while
                // also holding &mut self.memory.dsp.  We resolve this by
                // building a read-only RAM view.  This will be eliminated when
                // dsp.step() is refactored to take a &[u8] RAM slice directly
                // (see issue tracker — "borrow conflict" item).
                let ram_snapshot = self.memory.ram;
                self.memory.dsp.step_with_ram(&ram_snapshot);
            }

            self.cycles += 1;
        }
    }

    /// Generate `num_samples` stereo output samples (left, right interleaved).
    ///
    /// Steps the APU internally for each sample so that CPU, timers, and DSP
    /// all advance in lock-step.  Returns a Vec of length `num_samples * 2`.
    pub fn render_audio(&mut self, num_samples: usize) -> Vec<i16> {
        let mut buf = Vec::with_capacity(num_samples * 2);

        for _ in 0..num_samples {
            // Advance the full APU by one DSP period (32 CPU cycles = 1 sample).
            self.step(DSP_CYCLES_PER_SAMPLE);

            // Collect the stereo output from the DSP.
            let (l, r) = self.memory.dsp.render_audio_single();
            buf.push(l);
            buf.push(r);
        }

        buf
    }
}
