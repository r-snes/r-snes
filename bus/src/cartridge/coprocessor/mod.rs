//! Interface between the cartridge and the coprocessors that can sit on its board
//!
//! Each emulated chip implements [`CoprocessorChip`] and gets a match arm in
//! [`create`]. The cartridge owns the chip, gives it the first chance to answer
//! every bus access, and the core steps it with the master clock.

pub mod obc1;
pub mod srtc;
pub mod test_chip;

use crate::bus::AccessSpeed;
use crate::cartridge::header::RomHeader;
use crate::cartridge::header::cartridge_hardware::Coprocessor;
use crate::cartridge::header::mapping_mode::MappingMode;
use crate::cartridge::sram::Sram;
use common::snes_address::SnesAddress;

/// The parts of the cartridge board a coprocessor can reach
///
/// Many chips read the game ROM (DSP data tables, decompression chips) or
/// use the cartridge S-RAM as their working memory (OBC1, Super FX, SA-1),
/// so they get access to both on every bus access.
pub struct CartridgeMemory<'a> {
    /// Game ROM, without the copier header
    pub rom: &'a [u8],
    /// S-RAM of the cartridge (empty if the board has none)
    pub sram: &'a mut Sram,
    /// Mapping mode of the cartridge, as some chips move their registers with it
    pub map: MappingMode,
}

/// A coprocessor emulated on the cartridge board
///
/// Only `read` and `write` are required: a chip that answers instantly
/// (e.g. high-level emulated DSP-1, OBC1, S-RTC) can ignore the clock and IRQ.
pub trait CoprocessorChip {
    /// Reads a byte at `addr`, or `None` if the chip does not respond there
    ///
    /// Called before the rest of the cartridge, and before the I/O registers
    /// for `$2000-$5FFF` in banks `$00-$3F` / `$80-$BF`, so a chip only
    /// has to claim its own addresses and return `None` for everything else.
    /// Takes `&mut self` because reading a chip register can change its state.
    fn read(&mut self, addr: SnesAddress, mem: CartridgeMemory) -> Option<(u8, AccessSpeed)>;

    /// Writes a byte at `addr`, or returns `None` if the chip does not respond there
    fn write(&mut self, addr: SnesAddress, value: u8, mem: CartridgeMemory) -> Option<AccessSpeed>;

    /// Advances the chip by `master_cycles` master clock cycles
    fn step(&mut self, _master_cycles: u32) {}

    /// State of the chip's IRQ line toward the main CPU (`true` = asserted)
    fn irq(&self) -> bool {
        false
    }

    /// Resets the chip, called when the console is reset
    fn reset(&mut self) {}
}

/// Result of setting up the coprocessor declared in the ROM header
pub enum ChipSetup {
    /// The cartridge has no coprocessor
    NoChip,
    /// The coprocessor is emulated and ready
    Ready {
        /// The emulated chip
        chip: Box<dyn CoprocessorChip>,
        /// Caveat about how it is emulated, for the user (e.g. approximate emulation)
        note: Option<String>,
    },
    /// The coprocessor is not emulated: the message explains it to the user
    Unsupported(String),
}

/// Sets up the emulated chip for the coprocessor declared in the ROM header
pub fn create(header: &RomHeader) -> ChipSetup {
    match header.hardware.coprocessor {
        None => ChipSetup::NoChip,
        // One arm per emulated chip
        Some(Coprocessor::OBC1) => ChipSetup::Ready {
            chip: Box::new(obc1::Obc1::new()),
            note: None,
        },
        Some(Coprocessor::SRTC) => ChipSetup::Ready {
            chip: Box::new(srtc::Srtc::new()),
            note: None,
        },
        Some(chip) => ChipSetup::Unsupported(unsupported_message(chip)),
    }
}

/// Message shown when a ROM needs a coprocessor that is not emulated
pub fn unsupported_message(chip: Coprocessor) -> String {
    format!(
        "This ROM needs the {chip} coprocessor, which is not emulated yet. \
         It may not run correctly."
    )
}
