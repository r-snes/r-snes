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
    ///     `None` if no coprocessor is present or the value is unrecognized.
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
