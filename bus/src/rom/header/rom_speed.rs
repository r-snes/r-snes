use std::fmt;

/// Represents the speed of a SNES ROM.
///
/// Can be either Slow or Fast
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomSpeed {
    Slow,
    Fast,
}

impl RomSpeed {
    /// Creates a `RomSpeed` value from a byte extracted from the ROM header.
    ///
    /// Args:
    ///     byte: Byte from the ROM header representing the ROM speed.
    ///
    /// Returns:
    ///     A `RomSpeed` enum corresponding to the ROM's speed.
    pub fn from_byte(byte: u8) -> RomSpeed {
        let speed_bit = (byte >> 4) & 1;

        match speed_bit {
            // TODO : check if better way to represent a single bit
            0 => RomSpeed::Slow,
            1 => RomSpeed::Fast,
            _ => panic!("ERROR: Could not identify speed of ROM"),
        }
    }
}

impl fmt::Display for RomSpeed {
    /// Formats the ROM speed as a human-readable string.
    ///
    /// Examples:
    /// - `Slow` -> "Slow"
    /// - `Fast` -> "Fast"
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RomSpeed::Slow => write!(f, "Slow"),
            RomSpeed::Fast => write!(f, "Fast"),
        }
    }
}
