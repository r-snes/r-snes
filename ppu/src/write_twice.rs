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

    /// First write must return None and store the latch.
    #[test]
    fn test_first_write_returns_none() {
        let mut w = WriteTwice::new();
        assert_eq!(w.write(0xAB), None);
    }

    /// Second write must return Some((lo, hi)) with the correct bytes.
    #[test]
    fn test_second_write_returns_lo_hi() {
        let mut w = WriteTwice::new();
        w.write(0xCD);
        assert_eq!(w.write(0x12), Some((0xCD, 0x12)));
    }

    /// After a complete cycle, the third write must start a new latch cycle.
    #[test]
    fn test_third_write_starts_new_cycle() {
        let mut w = WriteTwice::new();
        w.write(0x11);
        w.write(0x22);
        assert_eq!(w.write(0x33), None); // new lo
        assert_eq!(w.write(0x44), Some((0x33, 0x44)));
    }

    /// reset must cancel a pending first write so the next write starts fresh.
    #[test]
    fn test_reset_cancels_pending_latch() {
        let mut w = WriteTwice::new();
        w.write(0xFF); // first write - pending
        w.reset();
        assert_eq!(w.write(0xAB), None); // treated as first write again
        assert_eq!(w.write(0x01), Some((0xAB, 0x01)));
    }
}
