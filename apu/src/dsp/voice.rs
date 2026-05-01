use super::adsr::Adsr;
use super::brr::Brr;

// ============================================================
// VOICE
// Represents one of the 8 independent audio channels.
// ============================================================

/// One voice (channel) of the SNES APU DSP.
#[derive(Debug, Clone, Copy, Default)]
pub struct Voice {
    /// Left channel volume, signed (-128..+127).
    pub left_vol: i8,

    /// Right channel volume, signed (-128..+127).
    pub right_vol: i8,

    /// 14-bit pitch value (0x0000–0x3FFF).
    /// 0x1000 = playback at the native 32 kHz sample rate.
    pub pitch: u16,

    /// Sample source number: index into the DIR table in APU RAM.
    pub srcn: u8,

    /// Whether this voice is currently keyed on (actively playing).
    pub key_on: bool,

    /// 16-bit pitch counter used to pace sample consumption.
    /// Conceptually a fixed-point accumulator; every 0x1000 units = 1 sample.
    pub pitch_counter: u16,

    /// Most recently output sample (16-bit, pre-envelope).
    pub current_sample: i16,

    /// ADSR envelope sub-state.
    pub adsr: Adsr,

    /// BRR decoder sub-state.
    pub brr: Brr,
}
