/// Two-write latch used by registers like BG1HOFS, BG1VOFS, CGDATA.
/// Models a hardware flipflop: first access = low byte, second = high byte.
pub struct WriteTwice {
    latch: u8,
    pub phase: BytePhase,
}

/// Helper enum to keep track of the byte phase
#[derive(Debug, PartialEq, Eq)]
pub enum BytePhase {
    /// Next read/write affects the low byte of the addressed word
    Low,

    /// Next read/write affects the high byte of the addressed word
    High,
}

impl BytePhase {
    pub fn flip(&mut self) {
        *self = match self {
            BytePhase::Low => BytePhase::High,
            BytePhase::High => BytePhase::Low,
        };
    }

    pub fn is_high(&self) -> bool {
        *self == BytePhase::High
    }
}

impl WriteTwice {
    pub fn new() -> Self {
        Self {
            latch: 0,
            phase: BytePhase::Low
        }
    }

    /// Feed one byte. Returns `Some((lo, hi))` on the second write, `None` on the first.
    pub fn write(&mut self, value: u8) -> Option<(u8, u8)> {
        match self.phase {
            BytePhase::Low => {
                self.latch = value;
                self.phase = BytePhase::High;
                None
            }
            BytePhase::High => {
                let lo = self.latch;
                self.phase = BytePhase::Low;
                Some((lo, value))
            }
        }
    }

    pub fn reset(&mut self) {
        self.phase = BytePhase::Low;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // WriteTwice::new
    // ============================================================

    /// A freshly created WriteTwice must start in Low phase with latch at 0.
    #[test]
    fn test_new_starts_in_low_phase() {
        let w = WriteTwice::new();
        assert_eq!(w.phase, BytePhase::Low);
    }

    // ============================================================
    // WriteTwice::write
    // ============================================================

    /// First write must return None and store the latch.
    #[test]
    fn test_first_write_returns_none() {
        let mut w = WriteTwice::new();
        assert_eq!(w.write(0xAB), None);
    }

    /// First write must set phase to High.
    #[test]
    fn test_first_write_sets_phase_to_high() {
        let mut w = WriteTwice::new();
        w.write(0xAB);
        assert_eq!(w.phase, BytePhase::High);
    }

    /// Second write must return Some((lo, hi)) with the correct bytes.
    #[test]
    fn test_second_write_returns_lo_hi() {
        let mut w = WriteTwice::new();
        w.write(0xCD);
        assert_eq!(w.write(0x12), Some((0xCD, 0x12)));
    }

    /// Second write must reset phase back to Low.
    #[test]
    fn test_second_write_resets_phase_to_low() {
        let mut w = WriteTwice::new();
        w.write(0xAB);
        w.write(0xCD);
        assert_eq!(w.phase, BytePhase::Low);
    }

    /// After a complete cycle, the third write must start a new latch cycle.
    #[test]
    fn test_third_write_starts_new_cycle() {
        let mut w = WriteTwice::new();
        w.write(0x11);
        w.write(0x22);
        assert_eq!(w.write(0x33), None);
        assert_eq!(w.write(0x44), Some((0x33, 0x44)));
    }

    // ============================================================
    // WriteTwice::reset
    // ============================================================

    /// reset from Low phase must keep phase at Low.
    #[test]
    fn test_reset_from_low_stays_low() {
        let mut w = WriteTwice::new();
        w.reset();
        assert_eq!(w.phase, BytePhase::Low);
    }

    /// reset must cancel a pending first write so the next write starts fresh.
    #[test]
    fn test_reset_from_high_returns_to_low() {
        let mut w = WriteTwice::new();
        w.write(0xFF);
        w.reset();
        assert_eq!(w.phase, BytePhase::Low);
    }

    /// After reset, the write cycle must restart correctly.
    #[test]
    fn test_reset_restarts_write_cycle() {
        let mut w = WriteTwice::new();
        w.write(0xFF);
        w.reset();
        assert_eq!(w.write(0xAB), None);
        assert_eq!(w.write(0x01), Some((0xAB, 0x01)));
    }

    // ============================================================
    // BytePhase::flip
    // ============================================================

    /// flip must toggle Low -> High.
    #[test]
    fn test_byte_phase_flip_low_to_high() {
        let mut phase = BytePhase::Low;
        phase.flip();
        assert_eq!(phase, BytePhase::High);
    }

    /// flip must toggle High -> Low.
    #[test]
    fn test_byte_phase_flip_high_to_low() {
        let mut phase = BytePhase::High;
        phase.flip();
        assert_eq!(phase, BytePhase::Low);
    }

    // ============================================================
    // BytePhase::is_high
    // ============================================================

    /// is_high must return false when phase is Low.
    #[test]
    fn test_is_high_returns_false_when_low() {
        assert!(!BytePhase::Low.is_high());
    }

    /// is_high must return true when phase is High.
    #[test]
    fn test_is_high_returns_true_when_high() {
        assert!(BytePhase::High.is_high());
    }
}
