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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rom_speed_from_byte_slow() {
        let bytes = [0x00, 0x01, 0x02, 0x0F];
        for &b in &bytes {
            assert_eq!(RomSpeed::from_byte(b), RomSpeed::Slow);
        }
    }

    #[test]
    fn test_rom_speed_from_byte_fast() {
        let bytes = [0x10, 0x11, 0x12, 0x1F];
        for &b in &bytes {
            assert_eq!(RomSpeed::from_byte(b), RomSpeed::Fast);
        }
    }

    #[test]
    fn test_rom_speed_display() {
        let mappings = [(RomSpeed::Slow, "Slow"), (RomSpeed::Fast, "Fast")];

        for (speed, expected) in mappings {
            assert_eq!(format!("{}", speed), expected);
        }
    }

    #[test]
    fn test_rom_speed_bits_ignored() {
        // Make sure only the 5th bit is used
        for b in 0x00..=0xFF {
            let expected = if (b >> 4) & 1 == 0 {
                RomSpeed::Slow
            } else {
                RomSpeed::Fast
            };
            assert_eq!(RomSpeed::from_byte(b), expected);
        }
    }
}
