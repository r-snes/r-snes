use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Country {
    International,
    Japan,
    USA,
    Europe,
    Scandinavia,
    Finland,
    Denmark,
    France,
    Holland,
    Spain,
    Germany,
    Italy,
    China,
    Indonesia,
    SouthKorea,
    Common,
    Canada,
    Brazil,
    Australia,
    OtherX,
    OtherY,
    OtherZ,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoStandard {
    NTSC,
    PAL,
    Other,
}

impl Country {
    pub fn from_byte(byte: u8) -> Country {
        match byte {
            0x00 => Country::Japan, // "0x00" sometimes means Japan or "International"
            0x01 => Country::USA,
            0x02 => Country::Europe,
            0x03 => Country::Scandinavia,
            0x04 => Country::Finland,
            0x05 => Country::Denmark,
            0x06 => Country::France,
            0x07 => Country::Holland,
            0x08 => Country::Spain,
            0x09 => Country::Germany,
            0x0A => Country::Italy,
            0x0B => Country::China,
            0x0C => Country::Indonesia,
            0x0D => Country::SouthKorea,
            0x0E => Country::Common,
            0x0F => Country::Canada,
            0x10 => Country::Brazil,
            0x11 => Country::Australia,
            0x12 => Country::OtherX,
            0x13 => Country::OtherY,
            0x14 => Country::OtherZ,
            _ => panic!("ERROR: Could not identify country of ROM"),
        }
    }
}

impl VideoStandard {
    pub fn from_country(country: Country) -> VideoStandard {
        match country {
            Country::Japan
            | Country::USA
            | Country::SouthKorea
            | Country::Canada
            | Country::Brazil => VideoStandard::NTSC,

            Country::Europe
            | Country::Scandinavia
            | Country::Finland
            | Country::Denmark
            | Country::France
            | Country::Holland
            | Country::Spain
            | Country::Germany
            | Country::Italy
            | Country::China
            | Country::Indonesia
            | Country::Australia => VideoStandard::PAL,

            _ => VideoStandard::Other,
        }
    }
}

impl fmt::Display for Country {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Country::International => write!(f, "International"),
            Country::Japan => write!(f, "Japan"),
            Country::USA => write!(f, "USA"),
            Country::Europe => write!(f, "Europe"),
            Country::Scandinavia => write!(f, "Scandinavia"),
            Country::Finland => write!(f, "Finland"),
            Country::Denmark => write!(f, "Denmark"),
            Country::France => write!(f, "France"),
            Country::Holland => write!(f, "Holland"),
            Country::Spain => write!(f, "Spain"),
            Country::Germany => write!(f, "Germany"),
            Country::Italy => write!(f, "Italy"),
            Country::China => write!(f, "China"),
            Country::Indonesia => write!(f, "Indonesia"),
            Country::SouthKorea => write!(f, "SouthKorea"),
            Country::Common => write!(f, "Common"),
            Country::Canada => write!(f, "Canada"),
            Country::Brazil => write!(f, "Brazil"),
            Country::Australia => write!(f, "Australia"),
            Country::OtherX => write!(f, "OtherX"),
            Country::OtherY => write!(f, "OtherY"),
            Country::OtherZ => write!(f, "OtherZ"),
        }
    }
}

impl fmt::Display for VideoStandard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VideoStandard::NTSC => write!(f, "NTSC"),
            VideoStandard::PAL => write!(f, "PAL"),
            VideoStandard::Other => write!(f, "Other"),
        }
    }
}
