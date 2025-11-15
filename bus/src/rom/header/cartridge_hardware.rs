use std::fmt;
use strum_macros::Display;

/// Represents the type of cartridge hardware used by a SNES ROM.
///
/// Contains an HardwareLayout and an optionnal Coprocessor
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CartridgeHardware {
    pub layout: HardwareLayout,
    pub coprocessor: Option<Coprocessor>,
}

/// Represents which type of hardware a SNES ROM contains.
///
/// Includes combinations of ROM, RAM, Battery and Coprocessor.
#[derive(Display, Debug, Clone, Copy, PartialEq)]
pub enum HardwareLayout {
    #[strum(serialize = "Rom")]
    RomOnly,

    #[strum(serialize = "Rom + Ram")]
    RomRam,

    #[strum(serialize = "Rom + Ram + Battery")]
    RomRamBattery,

    #[strum(serialize = "Rom + Coprocessor")]
    RomCoprocessor,

    #[strum(serialize = "Rom + Coprocessor + Ram")]
    RomCoprocessorRam,

    #[strum(serialize = "Rom + Coprocessor + Ram + Battery")]
    RomCoprocessorRamBattery,

    #[strum(serialize = "Rom + Coprocessor + Battery")]
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
        let layout = byte & 0x0F;
        let layout = match layout {
            0x0 => HardwareLayout::RomOnly,
            0x1 => HardwareLayout::RomRam,
            0x2 => HardwareLayout::RomRamBattery,
            0x3 => HardwareLayout::RomCoprocessor,
            0x4 => HardwareLayout::RomCoprocessorRam,
            0x5 => HardwareLayout::RomCoprocessorRamBattery,
            0x6 => HardwareLayout::RomCoprocessorBattery,
            _ => panic!("ERROR: Could not identify hardware of ROM"),
        };

        let coprocessor = (byte & 0xF0) >> 4;
        let coprocessor = match coprocessor {
            0x0 => Some(Coprocessor::DSP(1)),
            0x1 => Some(Coprocessor::GSU),
            0x2 => Some(Coprocessor::OBC1),
            0x3 => Some(Coprocessor::SA1),
            0x4 => Some(Coprocessor::SDD1),
            0x5 => Some(Coprocessor::SRTC),
            0xE => Some(Coprocessor::Other),
            0xF => Some(Coprocessor::Custom),
            _ => None,
        };

        CartridgeHardware {
            layout,
            coprocessor,
        }
    }

    /// Returns true if this cartridge has RAM
    pub fn has_ram(&self) -> bool {
        matches!(
            self.layout,
            HardwareLayout::RomRam
                | HardwareLayout::RomRamBattery
                | HardwareLayout::RomCoprocessorRam
                | HardwareLayout::RomCoprocessorRamBattery
        )
    }

    /// Returns true if this cartridge has a battery
    pub fn has_battery(&self) -> bool {
        matches!(
            self.layout,
            HardwareLayout::RomRamBattery
                | HardwareLayout::RomCoprocessorRamBattery
                | HardwareLayout::RomCoprocessorBattery
        )
    }

    /// Returns true if this cartridge contains a coprocessor
    pub fn has_coprocessor(&self) -> bool {
        matches!(
            self.layout,
            HardwareLayout::RomCoprocessor
                | HardwareLayout::RomCoprocessorRam
                | HardwareLayout::RomCoprocessorRamBattery
                | HardwareLayout::RomCoprocessorBattery
        )
    }
}

impl fmt::Display for Coprocessor {
    // Needed because `strum` can't format enum with parameters
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
            (0x00, HardwareLayout::RomOnly),
            (0x01, HardwareLayout::RomRam),
            (0x02, HardwareLayout::RomRamBattery),
            (0x03, HardwareLayout::RomCoprocessor),
            (0x04, HardwareLayout::RomCoprocessorRam),
            (0x05, HardwareLayout::RomCoprocessorRamBattery),
            (0x06, HardwareLayout::RomCoprocessorBattery),
        ];

        for (byte, expected) in mappings {
            assert_eq!(CartridgeHardware::from_byte(byte).layout, expected);
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_cartridge_components_availability() {
        let mappings = [
            (CartridgeHardware::from_byte(0x00), false, false, false),
            (CartridgeHardware::from_byte(0x01), true, false, false),
            (CartridgeHardware::from_byte(0x02), true, true, false),
            (CartridgeHardware::from_byte(0x03), false, false, true),
            (CartridgeHardware::from_byte(0x04), true, false, true),
            (CartridgeHardware::from_byte(0x05), true, true, true),
            (CartridgeHardware::from_byte(0x06), false, true, true),
            // Tens digit changed
            (CartridgeHardware::from_byte(0x10), false, false, false),
            (CartridgeHardware::from_byte(0x11), true, false, false),
            (CartridgeHardware::from_byte(0x12), true, true, false),
            (CartridgeHardware::from_byte(0x13), false, false, true),
            (CartridgeHardware::from_byte(0x14), true, false, true),
            (CartridgeHardware::from_byte(0x15), true, true, true),
            (CartridgeHardware::from_byte(0x16), false, true, true),
        ];

        for (hardware, has_ram, has_battery, has_coprocessor) in mappings {
            assert_eq!(hardware.has_ram(), has_ram);
            assert_eq!(hardware.has_battery(), has_battery);
            assert_eq!(hardware.has_coprocessor(), has_coprocessor);
        }
    }

    #[test]
    #[should_panic(expected = "ERROR: Could not identify hardware of ROM")]
    fn test_cartridge_hardware_from_byte_invalid() {
        CartridgeHardware::from_byte(0x07);
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
            (0x04, Some(Coprocessor::DSP(1))),
            (0x14, Some(Coprocessor::GSU)),
            (0x24, Some(Coprocessor::OBC1)),
            (0x34, Some(Coprocessor::SA1)),
            (0x44, Some(Coprocessor::SDD1)),
            (0x54, Some(Coprocessor::SRTC)),
            (0xE4, Some(Coprocessor::Other)),
            (0xF4, Some(Coprocessor::Custom)),
        ];

        for (byte, expected) in mappings {
            assert_eq!(CartridgeHardware::from_byte(byte).coprocessor, expected);
        }
    }

    #[test]
    fn test_coprocessor_from_byte_none() {
        let invalid_bytes = [0x60, 0x70, 0x80, 0x90, 0xA0, 0xB0, 0xC0, 0xD0];
        for &byte in &invalid_bytes {
            assert_eq!(CartridgeHardware::from_byte(byte).coprocessor, None);
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_cartridge_hardware_display() {
        let mappings = [
            (HardwareLayout::RomOnly, "Rom"),
            (HardwareLayout::RomRam, "Rom + Ram"),
            (HardwareLayout::RomRamBattery, "Rom + Ram + Battery"),
            (HardwareLayout::RomCoprocessor, "Rom + Coprocessor"),
            (HardwareLayout::RomCoprocessorRam, "Rom + Coprocessor + Ram"),
            (HardwareLayout::RomCoprocessorRamBattery, "Rom + Coprocessor + Ram + Battery"),
            (HardwareLayout::RomCoprocessorBattery, "Rom + Coprocessor + Battery"),
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
