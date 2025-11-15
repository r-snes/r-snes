use std::cell::RefCell;
use std::rc::Rc;

use crate::constants::{IO_END_ADDRESS, IO_START_ADDRESS};
use crate::memory_region::MemoryRegion;
use apu::Apu;
use common::snes_address::SnesAddress;
use cpu::cpu::CPU;
use ppu::ppu::PPU;

/// I/O Registers – 0x4000 bytes (mirrored)
///
/// - Memory area for various hardware components (CPU, APU, PPU, etc.).
/// - Accessible in banks 0x00–0x3F and 0x80–0xBF, within the address
///   range 0x2000–0x5FFF.
/// - Fully mirrored across all these banks.
///
/// For example, the addresses `0x004000` and `0x9E4000` both refer to the
/// same memory location.
pub struct Io {
    pub cpu: Rc<RefCell<CPU>>,
    pub ppu: Rc<RefCell<PPU>>,
    pub apu: Rc<RefCell<Apu>>,
}

impl Io {
    pub fn new(cpu: Rc<RefCell<CPU>>, ppu: Rc<RefCell<PPU>>, apu: Rc<RefCell<Apu>>) -> Self {
        Self { cpu, ppu, apu }
    }

    fn panic_invalid_addr(addr: SnesAddress) -> ! {
        panic!(
            "Incorrect access to the IO at address: {:06X}",
            usize::from(addr)
        );
    }
}

impl MemoryRegion for Io {
    /// Reads a byte from the I/O memory zone at the given `SnesAddress`.
    ///
    /// The address is translated to an internal I/O offset using `to_offset`.
    ///
    /// # Panics
    /// Panics if the address does not map to a valid I/O memory location.
    fn read(&self, addr: SnesAddress) -> u8 {
        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF
                if addr.addr >= IO_START_ADDRESS && addr.addr < IO_END_ADDRESS =>
            {
                match addr.addr {
                    0x2100..=0x213F => Self::handle_ppu_read(addr),
                    0x2140..=0x421F => Self::handle_others_read(addr),
                    0x4300..=0x437F => Self::handle_dma_read(addr),
                    _ => 0, // TODO : Check if it's open bus (I believe it is)
                }
            }
            _ => Self::panic_invalid_addr(addr),
        }
    }

    /// Writes a byte to the I/O memory zone at the given `SnesAddress`.
    ///
    /// The address is translated to an internal I/O offset using `to_offset`.
    ///
    /// # Panics
    /// Panics if the address does not map to a valid I/O memory location.
    fn write(&mut self, addr: SnesAddress, value: u8) {
        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF
                if addr.addr >= IO_START_ADDRESS && addr.addr < IO_END_ADDRESS =>
            {
                match addr.addr {
                    0x2100..=0x213F => Self::handle_ppu_write(addr, value),
                    0x2140..=0x421F => Self::handle_others_write(addr, value),
                    0x4300..=0x437F => Self::handle_dma_write(addr, value),
                    _ => Self::panic_invalid_addr(addr), // TODO : Check if we should do nothing
                }
            }
            _ => Self::panic_invalid_addr(addr),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::snes_address::snes_addr;
}
