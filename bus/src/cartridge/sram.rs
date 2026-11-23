//! Provides `Sram`, the static RAM of the cartridge

use crate::cartridge::header::RomHeader;

/// Static RAM on the cartridge board.
///
/// On real hardware this chip is battery-backed, which is what makes it save
/// data. At the moment it does not save anywhere and just acts as a basic RAM.
#[derive(PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sram {
    data: Vec<u8>,
    mask: usize,
}

impl Sram {
    /// Creates an S-RAM of the size declared in the ROM header
    pub fn new(header: &RomHeader) -> Self {
        let size = header.ram_size_bytes();

        Self {
            // Real S-RAM chips default is undefined but 0xFF should be fine.
            data: vec![0xFF; size],
            mask: size.saturating_sub(1),
        }
    }

    /// Whether `other` has the same size and mirroring as this S-RAM, so
    /// that it can take its place, e.g. when loading a save state.
    pub fn same_layout(&self, other: &Sram) -> bool {
        self.data.len() == other.data.len() && self.mask == other.mask
    }

    /// Whether the cartridge has an S-RAM chip
    pub fn is_present(&self) -> bool {
        !self.data.is_empty()
    }

    /// Reads a byte, `None` if the cartridge has no S-RAM.
    ///
    /// `linear` is the address on the chip's side of the board; masking here
    /// reproduces the mirroring caused by the chip's unconnected address lines.
    pub fn read(&self, linear: usize) -> Option<u8> {
        if self.is_present() {
            Some(self.data[linear & self.mask])
        } else {
            None
        }
    }

    /// Writes a byte, ignored if the cartridge has no S-RAM.
    ///
    /// `linear` is mirrored the same way as in `Sram::read`.
    pub fn write(&mut self, linear: usize, value: u8) {
        if self.is_present() {
            let offset = linear & self.mask;
            self.data[offset] = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sram(size: usize, mask: usize) -> Sram {
        Sram {
            data: vec![0; size],
            mask,
        }
    }

    #[test]
    fn an_sram_of_the_same_size_and_mirroring_has_the_same_layout() {
        assert!(sram(0x2000, 0x1FFF).same_layout(&sram(0x2000, 0x1FFF)));
        assert!(sram(0, 0).same_layout(&sram(0, 0)));
    }

    #[test]
    fn an_sram_of_another_size_or_mirroring_has_another_layout() {
        let sram_ = sram(0x2000, 0x1FFF);

        assert!(!sram_.same_layout(&sram(0x800, 0x7FF)));
        assert!(!sram_.same_layout(&sram(0, 0)));
        assert!(!sram_.same_layout(&sram(0x800, 0x1FFF)));
        assert!(!sram_.same_layout(&sram(0x2000, 0x7FF)));
    }
}
