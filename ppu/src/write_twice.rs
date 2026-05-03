/// Two-write latch used by registers like BG1HOFS, BG1VOFS, CGDATA.
/// First write stores the low byte; second write commits lo+hi to the target.
pub struct WriteTwice {
    latch: u8,
    written: bool,
}

impl WriteTwice {
    pub fn new() -> Self {
        Self { latch: 0, written: false }
    }

    /// Feed one byte. Returns `Some((lo, hi))` on the second write, `None` on the first.
    pub fn write(&mut self, value: u8) -> Option<(u8, u8)> {
        if !self.written {
            self.latch = value;
            self.written = true;
            None
        } else {
            let lo = self.latch;
            self.written = false;
            Some((lo, value))
        }
    }

    pub fn reset(&mut self) {
        self.written = false;
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
        w.write(0xFF); // first write — pending
        w.reset();
        assert_eq!(w.write(0xAB), None); // treated as first write again
        assert_eq!(w.write(0x01), Some((0xAB, 0x01)));
    }
}
