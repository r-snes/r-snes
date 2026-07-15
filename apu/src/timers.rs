use crate::memory::Memory;

/// The SPC700's three hardware timers ($FA-$FF).
///
/// Clocking (all derived from the 1.024 MHz CPU clock):
///   - Timers 0 and 1 tick at   8 kHz (once every 128 CPU cycles)
///   - Timer  2       ticks at 64 kHz (once every  16 CPU cycles)
///
/// Each timer has an internal stage counter that increments on its base
/// clock while its CONTROL enable bit (bits 0-2 of $F1) is set. When the
/// stage counter reaches the divisor ($FA-$FC; a divisor of 0 counts as
/// 256), it resets and the 4-bit output counter ($FD-$FF) increments,
/// wrapping 15 -> 0. Reading an output counter clears it (handled by
/// `Memory::read8_mut`).
///
/// A 0 -> 1 transition of an enable bit resets that timer's stage and
/// output counters, matching hardware: drivers enable a timer and expect
/// the first tick a full period later, not a leftover partial count.
pub struct Timers {
    /// Position within the 128-cycle base period (0..=127). Timer 2's
    /// 16-cycle clock is derived from the same counter (every 16th cycle),
    /// so all three timers stay phase-locked like the real chip's shared
    /// prescaler chain.
    cycle: u8,

    /// Internal stage counters compared against the divisors. u16 because
    /// a divisor of 0 means 256, which a u8 stage could never reach.
    stage: [u16; 3],

    /// CONTROL enable bits (0-2) seen on the previous step, for detecting
    /// the 0 -> 1 edges that reset a timer.
    prev_enable: u8,
}

impl Default for Timers {
    fn default() -> Self {
        Self::new()
    }
}

impl Timers {
    pub fn new() -> Self {
        Self {
            cycle:       0,
            stage:       [0; 3],
            prev_enable: 0,
        }
    }

    /// Advance the timers by one CPU cycle (1.024 MHz).
    pub fn step(&mut self, mem: &mut Memory) {
        let enables = mem.control & 0x07;

        // Enable rising edge: reset stage and output counter.
        let rising = enables & !self.prev_enable;
        self.prev_enable = enables;
        for t in 0..3 {
            if rising & (1 << t) != 0 {
                self.stage[t] = 0;
                mem.timer_out[t] = 0;
            }
        }

        self.cycle = (self.cycle + 1) & 127;

        // 64 kHz clock: every 16th CPU cycle.
        if self.cycle & 15 == 0 {
            self.tick(mem, 2, enables);
        }
        // 8 kHz clock: every 128th CPU cycle.
        if self.cycle == 0 {
            self.tick(mem, 0, enables);
            self.tick(mem, 1, enables);
        }
    }

    /// One base-clock tick for timer `t`: advance the stage counter and,
    /// on divisor match, increment the 4-bit output counter.
    fn tick(&mut self, mem: &mut Memory, t: usize, enables: u8) {
        if enables & (1 << t) == 0 {
            return;
        }

        let target = match mem.timer_div[t] {
            0 => 256u16, // hardware treats divisor 0 as 256
            d => d as u16,
        };

        self.stage[t] += 1;
        if self.stage[t] >= target {
            self.stage[t] = 0;
            mem.timer_out[t] = (mem.timer_out[t] + 1) & 0x0F;
        }
    }
}
