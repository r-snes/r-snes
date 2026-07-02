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
    /// The SPC700 runs at a fixed 1.024 MHz, derived from its own
    /// independent 24.576 MHz crystal -- it is NOT phase-locked to the
    /// SNES master clock. Real hardware has two unsynchronized oscillators;
    /// RSnes approximates the average ratio with an integer cycle-debt
    /// accumulator (see RSnes::update_apu_cycles).
    pub const CLOCK_HZ: u64 = 1_024_000;

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
            // TEMP DEBUG — confirms the APU is actually being stepped and
            // executing something (currently NOP forever at PC 0, since
            // there's no IPL boot ROM yet to give it real code to run).
            // Throttled to the first few cycles + then every ~1M cycles so
            // it doesn't flood stdout. Remove once IPL boot is wired up.
            if self.cycles < 5 || self.cycles % 1_000_000 == 0 {
                eprintln!(
                    "[apu debug] cycle={} pc={:#06x} opcode={:#04x}",
                    self.cycles,
                    self.cpu.regs.pc,
                    self.memory.read8(self.cpu.regs.pc),
                );
            }

            self.cpu.step(&mut self.memory);
            self.timers.step(&mut self.memory);

            self.dsp_cycles += 1;
            if self.dsp_cycles >= DSP_CYCLES_PER_SAMPLE {
                self.dsp_cycles = 0;
                self.memory.dsp.step(&self.memory.ram);
            }

            self.cycles += 1;
        }
    }

    /// Generate `num_samples` stereo output samples.
    ///
    /// Steps the APU internally for each sample so that CPU, timers, and DSP
    /// all advance in lock-step.  Returns a `Vec` of `(left, right)` pairs.
    pub fn render_audio(&mut self, num_samples: usize) -> Vec<(i16, i16)> {
        let mut buf = Vec::with_capacity(num_samples);

        for _ in 0..num_samples {
            // Advance the full APU by one DSP period (32 CPU cycles = 1 sample).
            self.step(DSP_CYCLES_PER_SAMPLE);

            // Collect the stereo output from the DSP as an explicit (L, R) pair.
            buf.push(self.memory.dsp.render_audio_single());
        }

        buf
    }
}
