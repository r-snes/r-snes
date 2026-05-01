/// Ticks between envelope updates for each rate index (0–31).
/// Rate 31 = update every tick; all other values are tick counts.
///
/// The real DSP uses a 32-entry lookup table to determine how many ticks
/// pass between each envelope step. Index 0 is a special sentinel meaning
/// infinite hold — the envelope never steps while this rate is active.
pub(super) const ENVELOPE_RATE_TABLE: [u16; 32] = [
    0,    // 0: never (infinite)
    2048, 1536, 1280, 1024, 768,
    640,  512,  384,  320,  256,
    192,  160,  128,  96,   80,
    64,   48,   40,   32,   24,
    20,   16,   12,   10,   8,
    6,    5,    4,    3,    2,
    1,    // 31: every tick
];

// ============================================================
// ADSR ENVELOPE
// Controls how loud a voice is over time using a 4-phase model.
// Envelope level is 11 bits wide (0x000–0x7FF).
// ============================================================

/// Current phase of the ADSR envelope state machine.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EnvelopePhase {
    /// Rise linearly from 0 to 0x7FF. Rate index comes from the 4-bit
    /// `attack_rate` field. Special case: `attack_rate == 15` uses a fixed
    /// +1024 step for a near-instant attack.
    Attack,

    /// Fall exponentially toward the sustain target. Rate index =
    /// `decay_rate * 2 + 16` (upper half of the rate table). Step size =
    /// `-(level >> 8) - 1`. Transitions to Sustain when the target is reached.
    Decay,

    /// Continue falling exponentially at the sustain rate. Rate 0 = infinite
    /// hold (envelope never steps). Transitions to Off when level reaches 0.
    Sustain,

    /// Fixed linear fade of -8 per tick, entered on key-off. No rate table
    /// gating. Transitions to Off when level reaches 0.
    Release,

    /// Voice is silent; envelope processing is skipped entirely.
    Off,
}

/// ADSR envelope for one voice.
#[derive(Debug, Clone, Copy)]
pub struct Adsr {
    /// true = ADSR mode, false = GAIN mode (GAIN not yet implemented)
    pub adsr_mode: bool,

    /// Attack rate index (0–15). Maps into the rate table.
    pub attack_rate: u8,

    /// Decay rate index (0–7). Maps into rate table as (rate*2 + 16).
    pub decay_rate: u8,

    /// Sustain level (0–7). Sustain target = (level + 1) * 0x100.
    pub sustain_level: u8,

    /// Sustain rate index (0–31). Direct index into rate table.
    pub sustain_rate: u8,

    /// Current 11-bit envelope volume (0x000–0x7FF).
    pub envelope_level: u16,

    /// Current phase of the envelope.
    pub envelope_phase: EnvelopePhase,

    /// Internal tick counter used to pace envelope updates.
    pub tick_counter: u16,
}

impl Default for Adsr {
    fn default() -> Self {
        Self {
            adsr_mode: false,
            attack_rate: 0,
            decay_rate: 0,
            sustain_level: 0,
            sustain_rate: 0,
            envelope_level: 0,
            envelope_phase: EnvelopePhase::Off,
            tick_counter: 0,
        }
    }
}

impl Adsr {
    /// Advance the envelope by one DSP tick (called once per output sample).
    ///
    /// The hardware only steps the envelope every N ticks, where N is
    /// determined by the rate table. Each phase has its own rate source.
    pub fn update_envelope(&mut self) {
        match self.envelope_phase {
            EnvelopePhase::Attack => {
                if self.attack_rate == 15 {
                    // Fast attack: fixed +1024 step, no rate gating
                    self.envelope_level = (self.envelope_level + 1024).min(0x7FF);
                } else {
                    // Normal attack: table-driven tick gating, +32 per step
                    let rate_idx = (self.attack_rate * 2 + 1) as usize;
                    let period = ENVELOPE_RATE_TABLE[rate_idx.min(31)];
                    if !self.tick_due(period) {
                        return;
                    }
                    self.envelope_level = (self.envelope_level + 32).min(0x7FF);
                }

                // Both branches above clamp with .min(0x7FF), so == is sufficient
                // and the redundant re-assignment of 0x7FF can be omitted.
                if self.envelope_level == 0x7FF {
                    self.envelope_phase = EnvelopePhase::Decay;
                    self.tick_counter = 0;
                }
            }

            EnvelopePhase::Decay => {
                let rate_idx = (self.decay_rate * 2 + 16) as usize;
                let period = ENVELOPE_RATE_TABLE[rate_idx.min(31)];
                if !self.tick_due(period) {
                    return;
                }

                // Exponential step proportional to current level
                let step = (self.envelope_level >> 8) + 1;
                self.envelope_level = self.envelope_level.saturating_sub(step);

                // Sustain target: (sustain_level + 1) * 0x100
                let target = (self.sustain_level as u16 + 1) * 0x100;
                if self.envelope_level <= target {
                    self.envelope_level = target;
                    self.envelope_phase = EnvelopePhase::Sustain;
                    self.tick_counter = 0;
                }
            }

            EnvelopePhase::Sustain => {
                let rate_idx = self.sustain_rate as usize;
                let period = ENVELOPE_RATE_TABLE[rate_idx.min(31)];
                // period == 0 means hold forever
                if !self.tick_due(period) {
                    return;
                }

                let step = (self.envelope_level >> 8) + 1;
                self.envelope_level = self.envelope_level.saturating_sub(step);

                if self.envelope_level == 0 {
                    self.envelope_phase = EnvelopePhase::Off;
                }
            }

            EnvelopePhase::Release => {
                self.envelope_level = self.envelope_level.saturating_sub(8);
                if self.envelope_level == 0 {
                    self.envelope_phase = EnvelopePhase::Off;
                }
            }

            EnvelopePhase::Off => {}
        }
    }

    /// Returns true if enough ticks have elapsed for an envelope step.
    /// `period` == 0 means never, so always returns false in that case.
    pub(super) fn tick_due(&mut self, period: u16) -> bool {
        if period == 0 {
            return false;
        }
        self.tick_counter += 1;
        if self.tick_counter >= period {
            self.tick_counter = 0;
            true
        } else {
            false
        }
    }
}
