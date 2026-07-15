use crate::memory::Memory;

/// The SPC700's three hardware timers ($FA-$FF).
///
/// NOT YET IMPLEMENTED: `step` is a no-op. The register plumbing exists
/// on the Memory side (CONTROL enable bits 0-2, divisors at $FA-$FC,
/// counters at $FD-$FF with clear-on-read), but nothing increments the
/// counters yet. Sound drivers rely on timer 0/1 for tempo, so this is
/// a prerequisite for music playback.
pub struct Timers;

impl Default for Timers {
    fn default() -> Self {
        Self::new()
    }
}

impl Timers {
    pub fn new() -> Self {
        Self
    }

    pub fn step(&mut self, _mem: &mut Memory) {
        // TODO: increment timer counters per CONTROL enables and divisors.
    }
}
