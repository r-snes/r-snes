use crate::constants::WRAM_SIZE;
use crate::memory_region::MemoryRegion;
use common::snes_address::SnesAddress;

/// WRAM (Work RAM) - 128 KiB (2 full banks)
///
/// - Located in banks 0x7E and 0x7F (64 KiB each).  
/// - A portion of bank 0x7E (`0x0000–0x1FFF`) is mirrored across all banks  
///   0x00–0x3F and 0x80–0xBF.
///
/// For example, the addresses `0x001000`, `0x7E1000`, and `0x9E1000` all refer to
/// the same memory location.
///
/// Warning: bank 0x7F is not mirrored, so `0x7F1000` is independent.
pub struct Wram {
    data: [u8; WRAM_SIZE],
}

impl Wram {
    pub fn new() -> Self {
        Self {
            data: [0; WRAM_SIZE],
        }
    }

    fn panic_invalid_addr(addr: SnesAddress) -> ! {
        panic!(
            "ERROR: Couldn't extract value from RAM at address: {:06X}",
            usize::from(addr)
        );
    }

    /// Converts a `SnesAddress` into an internal WRAM offset.
    ///
    /// Handles the mirroring of the lower 0x2000 bytes across banks 0x00–0x3F and 0x80–0xBF,
    /// and maps banks 0x7E and 0x7F to the actual 128 KiB WRAM.
    ///
    /// # Panics
    /// Panics if the given address does not correspond to a valid WRAM location.
    fn to_offset(addr: SnesAddress) -> usize {
        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF if addr.addr < 0x2000 => addr.addr as usize,
            0x7E => addr.addr as usize,
            0x7F => addr.addr as usize + 0x10000,
            _ => Self::panic_invalid_addr(addr),
        }
    }
}

impl MemoryRegion for Wram {
    /// Reads a byte from WRAM at the given `SnesAddress`.
    ///
    /// The address is first translated to an internal WRAM offset using `to_offset`.
    ///
    /// # Panics
    /// Panics if the address is invalid or out of bounds.
    fn read(&self, addr: SnesAddress) -> u8 {
        let offset = Self::to_offset(addr);

        return *self.data.get(offset).expect(&format!(
            "ERROR: Couldn't extract value from RAM at address: {:06X}",
            usize::from(addr)
        ));
    }

    /// Writes a byte to WRAM at the given `SnesAddress`.
    ///
    /// The address is first translated to an internal WRAM offset using `to_offset`.
    ///
    /// # Panics
    /// Panics if the address is invalid or out of bounds.
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
            for addr in 0..0x2000 {
                let address: SnesAddress = SnesAddress {
                    bank: bank,
                    addr: addr,
                };
                assert_eq!(Wram::to_offset(address), addr as usize);
            }
        }
    }

    #[test]
    #[should_panic]
    fn test_bad_map_addr_panics() {
        Wram::to_offset(SnesAddress {
            bank: (0x00),
            addr: (0x2000),
        });
    }

    #[test]
    #[should_panic]
    fn test_bad_map_addr_panics2() {
        Wram::to_offset(SnesAddress {
            bank: 0x0F,
            addr: 0x2000,
        });
    }

    #[test]
    #[should_panic(expected = "Incorrect access to the WRAM at address: E32345")]
    fn test_bad_map_addr_panic_message_read() {
        let wram = Wram::new();

        wram.read(SnesAddress {
            bank: 0xE3,
            addr: 0x2345,
        });
    }

    #[test]
    #[should_panic(expected = "Incorrect access to the WRAM at address: E32345")]
    fn test_bad_map_addr_panic_message_write() {
        let mut wram = Wram::new();

        wram.write(
            SnesAddress {
                bank: 0xE3,
                addr: 0x2345,
            },
            0x43,
        );
    }

    #[test]
    fn test_simple_read_write() {
        let mut wram = Wram::new();
        let mirrored_addr = SnesAddress {
            bank: 0x20,
            addr: 0x1456,
        };
        let first_full_bank_addr = SnesAddress {
            bank: 0x7E,
            addr: 0x4444,
        };
        let second_full_bank_addr = SnesAddress {
            bank: 0x7F,
            addr: 0x3E58,
        };

        wram.write(mirrored_addr, 0x43);
        assert_eq!(wram.read(mirrored_addr), 0x43);

        wram.write(first_full_bank_addr, 0xF3);
        assert_eq!(wram.read(first_full_bank_addr), 0xF3);

        wram.write(second_full_bank_addr, 0x2E);
        assert_eq!(wram.read(second_full_bank_addr), 0x2E);
    }

    #[test]
    fn test_full_bank_edges() {
        let mut wram = Wram::new();
        let first_bank_end = SnesAddress {
            bank: 0x7E,
            addr: 0xFFFF,
        };
        let second_bank_start = SnesAddress {
            bank: 0x7F,
            addr: 0x0000,
        };
        let second_bank_end = SnesAddress {
            bank: 0x7F,
            addr: 0x0000,
        };

        wram.write(first_bank_end, 0xF3);
        assert_eq!(wram.read(first_bank_end), 0xF3);

        assert_eq!(wram.read(second_bank_start), 0x00); // To check if first bank don't override

        wram.write(second_bank_start, 0x2E);
        assert_eq!(wram.read(second_bank_start), 0x2E);

        assert_eq!(wram.read(first_bank_end), 0xF3); // To check if second bank don't override first bank

        wram.write(second_bank_end, 0x45);
        assert_eq!(wram.read(second_bank_end), 0x45);
    }
}
