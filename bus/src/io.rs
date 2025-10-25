use common::snes_address::SnesAddress;

use crate::constants::{IO_END_ADDRESS, IO_SIZE, IO_START_ADDRESS};
use crate::memory_region::MemoryRegion;

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
    // TODO : Implement real CPU, PPU, APU, etc... memoriy behaviors.
    data: [u8; IO_SIZE],
}

impl Io {
    pub fn new() -> Self {
        Self { data: [0; IO_SIZE] }
    }

    fn panic_invalid_addr(addr: SnesAddress) -> ! {
        // TODO: Just print with usize when SnesAddress PR is merged
        panic!(
            "Incorrect access to the IO at address: {:02X}{:04X}",
            addr.bank, addr.addr
        );
    }

    /// Converts a `SnesAddress` into an internal I/O offset.
    ///
    /// Maps addresses between 0x2000–0x5FFF and mirrored across banks
    /// 0x00–0x3F and 0x80–0xBF.
    ///
    /// # Panics
    /// Panics if the address is outside the valid I/O memory zone range.
    fn to_offset(addr: SnesAddress) -> usize {
        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF
                if addr.addr >= IO_START_ADDRESS && addr.addr < IO_END_ADDRESS =>
            {
                addr.addr as usize
            }
            _ => Self::panic_invalid_addr(addr),
        }
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
        let offset = Self::to_offset(addr);

        return self.data.get(offset).copied().expect(&format!(
            "ERROR: Couldn't extract value from IO at address: {:06X}",
            usize::from(addr)
        ));
    }

    /// Writes a byte to the I/O memory zone at the given `SnesAddress`.
    ///
    /// The address is translated to an internal I/O offset using `to_offset`.
    ///
    /// # Panics
    /// Panics if the address does not map to a valid I/O memory location.
    fn write(&mut self, addr: SnesAddress, value: u8) {
        let offset = Self::to_offset(addr);

        if offset < self.data.len() {
            self.data[offset] = value;
        } else {
            // Shouldn't come here, panics just in case
            Self::panic_invalid_addr(addr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_good_map_addr() {
        for bank in (0x00..=0x3F).chain(0x80..=0xBF) {
            for addr in IO_START_ADDRESS..IO_END_ADDRESS {
                let address: SnesAddress = SnesAddress {
                    bank: bank,
                    addr: addr,
                };
                assert_eq!(Io::to_offset(address), addr as usize);
            }
        }
    }

    #[test]
    #[should_panic]
    fn test_bad_map_addr_panics() {
        Io::to_offset(SnesAddress {
            bank: 0x00,
            addr: IO_START_ADDRESS - 0x0321,
        });
    }

    #[test]
    #[should_panic]
    fn test_bad_map_addr_panics2() {
        Io::to_offset(SnesAddress {
            bank: 0x0F,
            addr: IO_END_ADDRESS + 0x34EF,
        });
    }

    #[test]
    #[should_panic(expected = "Incorrect access to the IO at address: E32345")]
    fn test_bad_map_addr_panic_message_read() {
        let io = Io::new();

        io.read(SnesAddress {
            bank: 0xE3,
            addr: 0x2345,
        });
    }

    #[test]
    #[should_panic(expected = "Incorrect access to the IO at address: E32345")]
    fn test_bad_map_addr_panic_message_write() {
        let mut io = Io::new();

        io.write(
            SnesAddress {
                bank: 0xE3,
                addr: 0x2345,
            },
            0x43,
        );
    }

    #[test]
    fn test_simple_read_write() {
        let mut wram = Io::new();
        let first_addr = SnesAddress {
            bank: 0x00,
            addr: IO_START_ADDRESS,
        };
        let second_addr = SnesAddress {
            bank: 0x9F,
            addr: IO_START_ADDRESS,
        };

        wram.write(first_addr, 0x43);
        assert_eq!(wram.read(first_addr), 0x43);

        wram.write(second_addr, 0x43);
        assert_eq!(wram.read(second_addr), 0x43);
    }
}
