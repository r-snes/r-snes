use crate::rom::header::RomHeader;

/// Static RAM on the cartridge board.
///
/// On real hardware this chip is battery-backed, which is what makes it save
/// data. At the moment it does not save anywhere and just acts as a basic RAM.
#[derive(PartialEq)]
pub struct Sram {
    data: Vec<u8>,
    mask: usize,
}

impl Sram {
    pub fn new(header: &RomHeader) -> Self {
        let size = header.ram_size_bytes();

        Self {
            // Real S-RAM chips default is undefined but OxFF should be fine.
            data: vec![0xFF; size],
            mask: size.saturating_sub(1),
        }
    }

    pub fn is_present(&self) -> bool {
        !self.data.is_empty()
    }

    /// Reads a byte, `None` if the cartridge has no save RAM.
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

    pub fn write(&mut self, linear: usize, value: u8) {
        if self.is_present() {
            let offset = linear & self.mask;
            self.data[offset] = value;
        }
    }
}
