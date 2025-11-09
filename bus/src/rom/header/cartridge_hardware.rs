use std::fmt;

/// Represents the type of cartridge hardware used by a SNES ROM.
///
/// Includes combinations of ROM, RAM, Battery backup, and Coprocessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartridgeHardware {
    RomOnly,
    RomRam,
    RomRamBattery,
    RomCoprocessor,
    RomCoprocessorRam,
    RomCoprocessorRamBattery,
    RomCoprocessorBattery,
}

/// Represents optional coprocessors present in a SNES cartridge.
///
/// Some coprocessors have additional identifiers (e.g., DSP number).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coprocessor {
    DSP(u8),
    GSU,
    OBC1,
    SA1,
    SDD1,
    SRTC,
    Other,
    Custom,
}

impl CartridgeHardware {
    /// Creates a `CartridgeHardware` value from a byte extracted from the ROM header.
    ///
    /// Args:
    ///     byte: Byte from the ROM header representing hardware configuration.
    ///
    /// Returns:
    ///     A `CartridgeHardware` enum corresponding to the ROM's hardware.
    pub fn from_byte(byte: u8) -> CartridgeHardware {
        let hardware_value = byte & 0x0F;

        match hardware_value {
            0x0 => CartridgeHardware::RomOnly,
            0x1 => CartridgeHardware::RomRam,
            0x2 => CartridgeHardware::RomRamBattery,
            0x3 => CartridgeHardware::RomCoprocessor,
            0x4 => CartridgeHardware::RomCoprocessorRam,
            0x5 => CartridgeHardware::RomCoprocessorRamBattery,
            0x6 => CartridgeHardware::RomCoprocessorBattery,
            _ => panic!("ERROR: Could not identify hardware of ROM"),
        }
    }
}

impl Coprocessor {
    /// Creates an optional `Coprocessor` from a byte extracted from the ROM header.
    ///
    /// Args:
    ///     byte: Byte from the ROM header representing coprocessor configuration.
    ///
    /// Returns:
    ///     `Some(Coprocessor)` if the coprocessor can be identified.
    ///     `None` if value is unrecognized.
    pub fn from_byte(byte: u8) -> Option<Coprocessor> {
        let coprocessor = (byte & 0xF0) >> 4;

        match coprocessor {
            0x0 => Some(Coprocessor::DSP(1)),
            0x1 => Some(Coprocessor::GSU),
            0x2 => Some(Coprocessor::OBC1),
            0x3 => Some(Coprocessor::SA1),
            0x4 => Some(Coprocessor::SDD1),
            0x5 => Some(Coprocessor::SRTC),
            0xE => Some(Coprocessor::Other),
            0xF => Some(Coprocessor::Custom),
            _ => None,
        }
    }
}

impl fmt::Display for CartridgeHardware {
    /// Formats the cartridge hardware as a human-readable string.
    ///
    /// Examples:
    /// - `RomOnly` -> "Rom"
    /// - `RomCoprocessorRamBattery` -> "Rom + Coprocessor + Ram + Battery"
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CartridgeHardware::RomOnly => write!(f, "Rom"),
            CartridgeHardware::RomRam => write!(f, "Rom + Ram"),
            CartridgeHardware::RomRamBattery => write!(f, "Rom + Ram + Battery"),
            CartridgeHardware::RomCoprocessor => write!(f, "Rom + Coprocessor"),
            CartridgeHardware::RomCoprocessorRam => write!(f, "Rom + Coprocessor + Ram"),
            CartridgeHardware::RomCoprocessorRamBattery => write!(f, "Rom + Coprocessor + Ram + Battery"),
            CartridgeHardware::RomCoprocessorBattery => write!(f, "Rom + Coprocessor + Battery"),
        }
    }
}

impl fmt::Display for Coprocessor {
    /// Formats the coprocessor as a human-readable string.
    ///
    /// Examples:
    /// - `DSP(1)` -> "DSP-1"
    /// - `GSU` -> "GSU"
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Coprocessor::DSP(nb) => write!(f, "DSP-{}", nb),
            Coprocessor::GSU => write!(f, "GSU"),
            Coprocessor::OBC1 => write!(f, "OBC1"),
            Coprocessor::SA1 => write!(f, "SA1"),
            Coprocessor::SDD1 => write!(f, "SDD1"),
            Coprocessor::SRTC => write!(f, "SRTC"),
            Coprocessor::Other => write!(f, "Other"),
            Coprocessor::Custom => write!(f, "Custom"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cartridge_hardware_from_byte_valid() {
        let mappings = [
            (0x0, CartridgeHardware::RomOnly),
            (0x1, CartridgeHardware::RomRam),
            (0x2, CartridgeHardware::RomRamBattery),
            (0x3, CartridgeHardware::RomCoprocessor),
            (0x4, CartridgeHardware::RomCoprocessorRam),
            (0x5, CartridgeHardware::RomCoprocessorRamBattery),
            (0x6, CartridgeHardware::RomCoprocessorBattery),
        ];

        for (byte, expected) in mappings {
            assert_eq!(CartridgeHardware::from_byte(byte), expected);
        }
    }

    #[test]
    #[should_panic(expected = "ERROR: Could not identify hardware of ROM")]
    fn test_cartridge_hardware_from_byte_invalid() {
        CartridgeHardware::from_byte(0x7);
    }

    #[test]
    fn test_coprocessor_from_byte_valid() {
        let mappings = [
            (0x00, Some(Coprocessor::DSP(1))),
            (0x10, Some(Coprocessor::GSU)),
            (0x20, Some(Coprocessor::OBC1)),
            (0x30, Some(Coprocessor::SA1)),
            (0x40, Some(Coprocessor::SDD1)),
            (0x50, Some(Coprocessor::SRTC)),
            (0xE0, Some(Coprocessor::Other)),
            (0xF0, Some(Coprocessor::Custom)),
            // Tens digit changed
            (0x07, Some(Coprocessor::DSP(1))),
            (0x17, Some(Coprocessor::GSU)),
            (0x27, Some(Coprocessor::OBC1)),
            (0x37, Some(Coprocessor::SA1)),
            (0x47, Some(Coprocessor::SDD1)),
            (0x57, Some(Coprocessor::SRTC)),
            (0xE7, Some(Coprocessor::Other)),
            (0xF7, Some(Coprocessor::Custom)),
        ];

        for (byte, expected) in mappings {
            assert_eq!(Coprocessor::from_byte(byte), expected);
        }
    }

    #[test]
    fn test_coprocessor_from_byte_none() {
        let invalid_bytes = [0x60, 0x70, 0x80, 0x90, 0xA0, 0xB0, 0xC0, 0xD0];
        for &byte in &invalid_bytes {
            assert_eq!(Coprocessor::from_byte(byte), None);
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_cartridge_hardware_display() {
        let mappings = [
            (CartridgeHardware::RomOnly, "Rom"),
            (CartridgeHardware::RomRam, "Rom + Ram"),
            (CartridgeHardware::RomRamBattery, "Rom + Ram + Battery"),
            (CartridgeHardware::RomCoprocessor, "Rom + Coprocessor"),
            (CartridgeHardware::RomCoprocessorRam, "Rom + Coprocessor + Ram"),
            (CartridgeHardware::RomCoprocessorRamBattery, "Rom + Coprocessor + Ram + Battery"),
            (CartridgeHardware::RomCoprocessorBattery, "Rom + Coprocessor + Battery"),
        ];

        for (hardware, expected) in mappings {
            assert_eq!(format!("{}", hardware), expected);
        }
    }

    #[test]
    fn test_coprocessor_display() {
        let mappings = [
            (Coprocessor::DSP(1), "DSP-1"),
            (Coprocessor::GSU, "GSU"),
            (Coprocessor::OBC1, "OBC1"),
            (Coprocessor::SA1, "SA1"),
            (Coprocessor::SDD1, "SDD1"),
            (Coprocessor::SRTC, "SRTC"),
            (Coprocessor::Other, "Other"),
            (Coprocessor::Custom, "Custom"),
        ];

        for (coproc, expected) in mappings {
            assert_eq!(format!("{}", coproc), expected);
        }
    }
}
