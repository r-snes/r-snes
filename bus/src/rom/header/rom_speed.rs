use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomSpeed {
    Slow,
    Fast,
}

impl RomSpeed {
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RomSpeed::Slow => write!(f, "Slow"),
            RomSpeed::Fast => write!(f, "Fast"),
        }
    }
}
