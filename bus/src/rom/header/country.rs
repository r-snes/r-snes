use strum_macros::Display;

/// Represents the country or region code of a SNES ROM.
///
/// Covers official regions and some miscellaneous/other codes.
#[derive(Display, Debug, Clone, Copy, PartialEq)]
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

/// Represents the video standard used by a SNES ROM.
///
/// Mainly NTSC or PAL, with an "Other" option for unknown/unsupported regions.
#[derive(Display, Debug, Clone, Copy, PartialEq)]
pub enum VideoStandard {
    NTSC,
    PAL,
    Other,
}

impl Country {
    /// Creates a `Country` value from a byte extracted from the ROM header.
    ///
    /// Args:
    ///     byte: Byte from the ROM header representing the country/region code.
    ///
    /// Returns:
    ///     A `Country` enum corresponding to the ROM's region.
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
    /// Determines the video standard (NTSC/PAL/Other) based on a given `Country`.
    ///
    /// Args:
    ///     country: The country/region of the ROM.
    ///
    /// Returns:
    ///     A `VideoStandard` enum corresponding to the country's standard.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_country_from_byte_valid() {
        let mappings = [
            (0x00, Country::Japan),
            (0x01, Country::USA),
            (0x02, Country::Europe),
            (0x03, Country::Scandinavia),
            (0x04, Country::Finland),
            (0x05, Country::Denmark),
            (0x06, Country::France),
            (0x07, Country::Holland),
            (0x08, Country::Spain),
            (0x09, Country::Germany),
            (0x0A, Country::Italy),
            (0x0B, Country::China),
            (0x0C, Country::Indonesia),
            (0x0D, Country::SouthKorea),
            (0x0E, Country::Common),
            (0x0F, Country::Canada),
            (0x10, Country::Brazil),
            (0x11, Country::Australia),
            (0x12, Country::OtherX),
            (0x13, Country::OtherY),
            (0x14, Country::OtherZ),
        ];

        for (byte, expected) in mappings {
            assert_eq!(Country::from_byte(byte), expected);
        }
    }

    #[test]
    #[should_panic(expected = "ERROR: Could not identify country of ROM")]
    fn test_country_from_byte_invalid() {
        Country::from_byte(0xFF);
    }

    #[test]
    fn test_video_standard_from_country() {
        let ntsc_countries = [
            Country::Japan,
            Country::USA,
            Country::SouthKorea,
            Country::Canada,
            Country::Brazil,
        ];
        let pal_countries = [
            Country::Europe,
            Country::Scandinavia,
            Country::Finland,
            Country::Denmark,
            Country::France,
            Country::Holland,
            Country::Spain,
            Country::Germany,
            Country::Italy,
            Country::China,
            Country::Indonesia,
            Country::Australia,
        ];
        let other_countries = [
            Country::International,
            Country::Common,
            Country::OtherX,
            Country::OtherY,
            Country::OtherZ,
        ];

        for &c in &ntsc_countries {
            assert_eq!(VideoStandard::from_country(c), VideoStandard::NTSC);
        }
        for &c in &pal_countries {
            assert_eq!(VideoStandard::from_country(c), VideoStandard::PAL);
        }
        for &c in &other_countries {
            assert_eq!(VideoStandard::from_country(c), VideoStandard::Other);
        }
    }

    #[test]
    fn test_country_display() {
        let mappings = [
            (Country::International, "International"),
            (Country::Japan, "Japan"),
            (Country::USA, "USA"),
            (Country::Europe, "Europe"),
            (Country::Scandinavia, "Scandinavia"),
            (Country::Finland, "Finland"),
            (Country::Denmark, "Denmark"),
            (Country::France, "France"),
            (Country::Holland, "Holland"),
            (Country::Spain, "Spain"),
            (Country::Germany, "Germany"),
            (Country::Italy, "Italy"),
            (Country::China, "China"),
            (Country::Indonesia, "Indonesia"),
            (Country::SouthKorea, "SouthKorea"),
            (Country::Common, "Common"),
            (Country::Canada, "Canada"),
            (Country::Brazil, "Brazil"),
            (Country::Australia, "Australia"),
            (Country::OtherX, "OtherX"),
            (Country::OtherY, "OtherY"),
            (Country::OtherZ, "OtherZ"),
        ];

        for (country, expected) in mappings {
            assert_eq!(format!("{}", country), expected);
        }
    }

    #[test]
    fn test_video_standard_display() {
        let mappings = [
            (VideoStandard::NTSC, "NTSC"),
            (VideoStandard::PAL, "PAL"),
            (VideoStandard::Other, "Other"),
        ];

        for (standard, expected) in mappings {
            assert_eq!(format!("{}", standard), expected);
        }
    }
}
