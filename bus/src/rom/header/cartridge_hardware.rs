use std::fmt;

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
