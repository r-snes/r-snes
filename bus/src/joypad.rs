//! SNES controller ports.
//!
//! A standard pad is a 16-bit parallel-in / serial-out shift register. The
//! CPU talks to it through three lines shared by both ports:
//!
//! - Latch (JOYOUT bit 0, `$4016` write, or the auto-read strobe): while
//!   high the register keeps reloading the current button state; the value is
//!   frozen on the falling edge.
//! - Clock (pulsed by reading `$4016` for port 1, `$4017` for port 2, or by
//!   the auto-read): shifts the next bit onto the data line.
//! - Data: the current bit
//!
//! # Reference
//! [SNESdev Wiki - Standard controller](https://snes.nesdev.org/wiki/Standard_controller)

/// One controller port and the device plugged into it.
///
/// Button states use the JOY1 layout:
///
/// | bit | 15 | 14 | 13     | 12    | 11 | 10   | 9    | 8     | 7 | 6 | 5 | 4 | 3–0 |
/// |-----|----|----|--------|-------|----|------|------|-------|---|---|---|---|-----|
/// |     | B  | Y  | Select | Start | Up | Down | Left | Right | A | X | L | R | ID  |
///
/// A set bit means "pressed". The 4 ID bits are always `0` for a standard pad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControllerPort {
    /// Whether a standard pad is plugged in. An empty port reads `0` forever.
    pub connected: bool,
    /// Live button state, supplied by the frontend.
    buttons: u16,
    /// Shift register contents; the MSB is the bit currently on the data line.
    shift: u16,
    /// Current level of the latch line.
    latch: bool,
}

impl ControllerPort {
    const ID_MASK: u16 = 0x000F;

    pub fn new() -> Self {
        Self {
            connected: true,
            buttons: 0,
            shift: 0,
            latch: false,
        }
    }

    /// Replace the live button state.
    /// The ID bits are forced to `0`.
    pub fn set_buttons(&mut self, buttons: u16) {
        self.buttons = buttons & !Self::ID_MASK;
    }

    /// Live button state.
    pub fn buttons(&self) -> u16 {
        self.buttons
    }

    /// Drive the latch line. The button state is captured on the falling edge.
    pub fn set_latch(&mut self, on: bool) {
        if self.latch && !on {
            self.shift = self.buttons;
        }
        self.latch = on;
    }

    /// Level of data line 1, without clocking.
    ///
    /// While latched, the pad keeps reloading, so this reports B continuously.
    pub fn data1(&self) -> u8 {
        if !self.connected {
            return 0;
        }
        let word = if self.latch { self.buttons } else { self.shift };
        ((word >> 15) & 1) as u8
    }

    /// Clock pulse: advance to the next bit. Official pads shift in `1`s, so
    /// every read after the 16th returns `1` (games use this to detect a pad).
    pub fn clock(&mut self) {
        if !self.latch {
            self.shift = (self.shift << 1) | 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_bits(port: &mut ControllerPort, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| {
                let bit = port.data1();
                port.clock();
                bit
            })
            .collect()
    }

    #[test]
    fn serial_order_is_msb_first_then_ones() {
        let mut pad = ControllerPort::new();
        pad.set_buttons(0xA000);
        pad.set_latch(true);
        pad.set_latch(false);

        let bits = read_bits(&mut pad, 18);
        assert_eq!(&bits[..4], &[1, 0, 1, 0]);
        assert!(bits[4..16].iter().all(|&b| b == 0));
        assert_eq!(&bits[16..], &[1, 1]);
    }

    #[test]
    fn latched_pad_keeps_reporting_b() {
        let mut pad = ControllerPort::new();
        pad.set_buttons(0x8000);
        pad.set_latch(true);
        assert_eq!(read_bits(&mut pad, 4), vec![1, 1, 1, 1]);
    }

    #[test]
    fn state_is_frozen_on_latch_falling_edge() {
        let mut pad = ControllerPort::new();
        pad.set_buttons(0x8000);
        pad.set_latch(true);
        pad.set_latch(false);
        pad.set_buttons(0);
        assert_eq!(pad.data1(), 1);
    }

    #[test]
    fn id_bits_are_forced_to_zero() {
        let mut pad = ControllerPort::new();
        pad.set_buttons(0xFFFF);
        assert_eq!(pad.buttons(), 0xFFF0);
    }

    #[test]
    fn empty_port_reads_zero() {
        let mut pad = ControllerPort::new();
        pad.connected = false;
        pad.set_buttons(0xFFFF);
        pad.set_latch(true);
        pad.set_latch(false);
        assert!(read_bits(&mut pad, 20).iter().all(|&b| b == 0));
    }
}
