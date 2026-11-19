//! Types describing the hardware present on the cartridge board

use std::fmt;
use strum_macros::Display;

/// Represents the type of cartridge hardware used by a SNES ROM.
///
/// Contains an HardwareLayout and an optionnal Coprocessor
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CartridgeHardware {
    /// Chips present on the board (ROM, RAM, battery, coprocessor)
    pub layout: HardwareLayout,
    /// Coprocessor on the board, `None` if the layout has none
    pub coprocessor: Option<Coprocessor>,
}

/// Represents which type of hardware a SNES ROM contains.
///
/// Includes combinations of ROM, RAM, Battery and Coprocessor.
#[derive(Display, Debug, Clone, Copy, PartialEq)]
pub enum HardwareLayout {
    /// ROM only
    #[strum(serialize = "Rom")]
    RomOnly,

    /// ROM and RAM
    #[strum(serialize = "Rom + Ram")]
    RomRam,

    /// ROM and battery-backed RAM
    #[strum(serialize = "Rom + Ram + Battery")]
    RomRamBattery,

    /// ROM and coprocessor
    #[strum(serialize = "Rom + Coprocessor")]
    RomCoprocessor,

    /// ROM, coprocessor and RAM
    #[strum(serialize = "Rom + Coprocessor + Ram")]
    RomCoprocessorRam,

    /// ROM, coprocessor and battery-backed RAM
    #[strum(serialize = "Rom + Coprocessor + Ram + Battery")]
    RomCoprocessorRamBattery,

    /// ROM, coprocessor and battery
    #[strum(serialize = "Rom + Coprocessor + Battery")]
    RomCoprocessorBattery,

    /// Low nibble of the chipset byte is not a documented layout
    /// (e.g. `$x9`/`$xA` used by a few coprocessor boards).
    #[strum(serialize = "Unknown")]
    Unknown,
}

/// Represents optional coprocessors present in a SNES cartridge.
///
/// Some coprocessors have additional identifiers (e.g., DSP number).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coprocessor {
    /// DSP math coprocessor, with its version number (DSP-1, DSP-2, ...)
    DSP(u8),
    /// Graphics Support Unit (Super FX)
    GSU,
    /// OBC-1 sprite management chip
    OBC1,
    /// SA-1 accelerator (65C816-based CPU)
    SA1,
    /// S-DD1 decompression chip
    SDD1,
    /// S-RTC real-time clock
    SRTC,
    /// Other coprocessor (header code `0xE`, e.g. Super Game Boy, Satellaview)
    Other,
    /// SPC7110 decompression chip (header code `0xF`, subtype `$00`)
    SPC7110,
    /// ST010 / ST011 (header code `0xF`, subtype `$01`)
    ST010,
    /// ST018 (header code `0xF`, subtype `$02`)
    ST018,
    /// CX4 (header code `0xF`, subtype `$10`)
    CX4,
    /// Custom coprocessor (header code `0xF`) with an unknown subtype at `$FFBF`
    Custom(u8),
    /// Undocumented chipset high nibble
    Unknown(u8),
}

impl HardwareLayout {
    /// Returns true if this cartridge has RAM
    pub fn has_ram(&self) -> bool {
        matches!(
            self,
            HardwareLayout::RomRam
                | HardwareLayout::RomRamBattery
                | HardwareLayout::RomCoprocessorRam
                | HardwareLayout::RomCoprocessorRamBattery
        )
    }

    /// Returns true if this cartridge has a battery
    pub fn has_battery(&self) -> bool {
        matches!(
            self,
            HardwareLayout::RomRamBattery
                | HardwareLayout::RomCoprocessorRamBattery
                | HardwareLayout::RomCoprocessorBattery
        )
    }

    /// Returns true if this cartridge contains a coprocessor
    pub fn has_coprocessor(&self) -> bool {
        matches!(
            self,
            HardwareLayout::RomCoprocessor
                | HardwareLayout::RomCoprocessorRam
                | HardwareLayout::RomCoprocessorRamBattery
                | HardwareLayout::RomCoprocessorBattery
        )
    }
}

impl CartridgeHardware {
    /// Creates a `CartridgeHardware` value from the chipset bytes of the ROM header.
    ///
    /// Args:
    ///     byte: Chipset byte (`$FFD6`) representing the hardware configuration.
    ///     subtype: Chipset subtype (`$FFBF`), only used when the chipset is `$Fx`.
    ///
    /// Returns:
    ///     A `CartridgeHardware` struct containing the ROM layout and an `Option<Coprocessor>`.
    ///     Never panics: undocumented values map to `Unknown` variants.
    pub fn from_bytes(byte: u8, subtype: u8) -> CartridgeHardware {
        let layout = match byte & 0x0F {
            0x0 => HardwareLayout::RomOnly,
            0x1 => HardwareLayout::RomRam,
            0x2 => HardwareLayout::RomRamBattery,
            0x3 => HardwareLayout::RomCoprocessor,
            0x4 => HardwareLayout::RomCoprocessorRam,
            0x5 => HardwareLayout::RomCoprocessorRamBattery,
            0x6 => HardwareLayout::RomCoprocessorBattery,
            _ => HardwareLayout::Unknown,
        };

        let chip_nibble = (byte & 0xF0) >> 4;
        // An undocumented layout still signals a coprocessor when the
        // high nibble names one, since that nibble is only meaningful with a chip.
        let has_coprocessor = match layout {
            HardwareLayout::Unknown => chip_nibble != 0,
            _ => layout.has_coprocessor(),
        };

        let coprocessor = if has_coprocessor {
            Some(match chip_nibble {
                // DSP-1 to DSP-4 share this code; DSP-1 is by far the most common.
                0x0 => Coprocessor::DSP(1),
                0x1 => Coprocessor::GSU,
                0x2 => Coprocessor::OBC1,
                0x3 => Coprocessor::SA1,
                0x4 => Coprocessor::SDD1,
                0x5 => Coprocessor::SRTC,
                0xE => Coprocessor::Other,
                0xF => match subtype {
                    0x00 => Coprocessor::SPC7110,
                    0x01 => Coprocessor::ST010,
                    0x02 => Coprocessor::ST018,
                    0x10 => Coprocessor::CX4,
                    other => Coprocessor::Custom(other),
                },
                other => Coprocessor::Unknown(other),
            })
        } else {
            None
        };

        CartridgeHardware {
            layout,
            coprocessor,
        }
    }

    /// Returns true if this cartridge has RAM
    pub fn has_ram(&self) -> bool {
        self.layout.has_ram()
    }

    /// Returns true if this cartridge has a battery
    pub fn has_battery(&self) -> bool {
        self.layout.has_battery()
    }

    /// Returns true if this cartridge contains a coprocessor
    pub fn has_coprocessor(&self) -> bool {
        self.coprocessor.is_some()
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
            Coprocessor::SPC7110 => write!(f, "SPC7110"),
            Coprocessor::ST010 => write!(f, "ST010/ST011"),
            Coprocessor::ST018 => write!(f, "ST018"),
            Coprocessor::CX4 => write!(f, "CX4"),
            Coprocessor::Custom(sub) => write!(f, "Custom (subtype ${:02X})", sub),
            Coprocessor::Unknown(code) => write!(f, "Unknown (code ${:X})", code),
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
            assert_eq!(CartridgeHardware::from_bytes(byte, 0).layout, expected);
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_cartridge_components_availability() {
        let mappings = [
            (CartridgeHardware::from_bytes(0x00, 0), false, false, false),
            (CartridgeHardware::from_bytes(0x01, 0), true, false, false),
            (CartridgeHardware::from_bytes(0x02, 0), true, true, false),
            (CartridgeHardware::from_bytes(0x03, 0), false, false, true),
            (CartridgeHardware::from_bytes(0x04, 0), true, false, true),
            (CartridgeHardware::from_bytes(0x05, 0), true, true, true),
            (CartridgeHardware::from_bytes(0x06, 0), false, true, true),
            // Tens digit changed
            (CartridgeHardware::from_bytes(0x10, 0), false, false, false),
            (CartridgeHardware::from_bytes(0x11, 0), true, false, false),
            (CartridgeHardware::from_bytes(0x12, 0), true, true, false),
            (CartridgeHardware::from_bytes(0x13, 0), false, false, true),
            (CartridgeHardware::from_bytes(0x14, 0), true, false, true),
            (CartridgeHardware::from_bytes(0x15, 0), true, true, true),
            (CartridgeHardware::from_bytes(0x16, 0), false, true, true),
        ];

        for (hardware, has_ram, has_battery, has_coprocessor) in mappings {
            assert_eq!(hardware.has_ram(), has_ram);
            assert_eq!(hardware.has_battery(), has_battery);
            assert_eq!(hardware.has_coprocessor(), has_coprocessor);
        }
    }

    #[test]
    fn test_cartridge_hardware_from_byte_unknown_layout_does_not_panic() {
        let hw = CartridgeHardware::from_bytes(0x07, 0);
        assert_eq!(hw.layout, HardwareLayout::Unknown);
        assert_eq!(hw.coprocessor, None);

        // e.g. SPC7110 + RTC boards use $F9
        let hw = CartridgeHardware::from_bytes(0xF9, 0x00);
        assert_eq!(hw.layout, HardwareLayout::Unknown);
        assert_eq!(hw.coprocessor, Some(Coprocessor::SPC7110));
    }

    #[test]
    fn test_custom_coprocessor_subtypes() {
        let mappings = [
            (0x00, Coprocessor::SPC7110),
            (0x01, Coprocessor::ST010),
            (0x02, Coprocessor::ST018),
            (0x10, Coprocessor::CX4),
            (0x42, Coprocessor::Custom(0x42)),
        ];

        for (subtype, expected) in mappings {
            assert_eq!(
                CartridgeHardware::from_bytes(0xF5, subtype).coprocessor,
                Some(expected)
            );
        }
    }

    #[test]
    fn test_subtype_ignored_outside_fx() {
        assert_eq!(
            CartridgeHardware::from_bytes(0x35, 0x10).coprocessor,
            Some(Coprocessor::SA1)
        );
    }

    #[test]
    fn test_coprocessor_from_byte_valid() {
        let mappings = [
            (0x00, None),
            (0x10, None),
            (0x20, None),
            (0x30, None),
            (0x40, None),
            (0x50, None),
            (0xE0, None),
            (0xF0, None),
            // Tens digit changed
            (0x04, Some(Coprocessor::DSP(1))),
            (0x14, Some(Coprocessor::GSU)),
            (0x24, Some(Coprocessor::OBC1)),
            (0x34, Some(Coprocessor::SA1)),
            (0x44, Some(Coprocessor::SDD1)),
            (0x54, Some(Coprocessor::SRTC)),
            (0xE4, Some(Coprocessor::Other)),
            (0xF4, Some(Coprocessor::SPC7110)),
        ];

        for (byte, expected) in mappings {
            assert_eq!(CartridgeHardware::from_bytes(byte, 0).coprocessor, expected);
        }
    }

    #[test]
    fn test_coprocessor_from_byte_none() {
        let no_chip_bytes = [0x60, 0x70, 0x80, 0x90, 0xA0, 0xB0, 0xC0, 0xD0];
        for &byte in &no_chip_bytes {
            assert_eq!(CartridgeHardware::from_bytes(byte, 0).coprocessor, None);
        }
    }

    #[test]
    fn test_coprocessor_from_byte_unknown_code() {
        for code in 0x6..=0xD {
            let byte = (code << 4) | 0x3;
            assert_eq!(
                CartridgeHardware::from_bytes(byte, 0).coprocessor,
                Some(Coprocessor::Unknown(code))
            );
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
            (Coprocessor::SPC7110, "SPC7110"),
            (Coprocessor::ST010, "ST010/ST011"),
            (Coprocessor::ST018, "ST018"),
            (Coprocessor::CX4, "CX4"),
            (Coprocessor::Custom(0x42), "Custom (subtype $42)"),
            (Coprocessor::Unknown(0x7), "Unknown (code $7)"),
        ];

        for (coproc, expected) in mappings {
            assert_eq!(format!("{}", coproc), expected);
        }
    }
}
