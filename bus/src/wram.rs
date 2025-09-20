use crate::constants::WRAM_SIZE;
use crate::memory_region::MemoryRegion;
use common::snes_address::SnesAddress;

pub struct Wram {
    data: [u8; WRAM_SIZE], // 128 KiB WRAM
}

impl Wram {
    pub fn new() -> Self {
        Self {
            data: [0; WRAM_SIZE],
        }
    }

    fn panic_invalid_addr(addr: SnesAddress) -> ! {
        // TODO: Just print with usize when SnesAddress PR is merged
        panic!(
            "Incorrect access to the WRAM at address: {:02X}{:04X}",
            addr.bank, addr.addr
        );
    }

    fn map_addr(addr: SnesAddress) -> usize {
        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF => {
                if addr.addr < 0x2000 {
                    return addr.addr as usize;
                } else {
                    Self::panic_invalid_addr(addr);
                }
            }
            0x7E => {
                return addr.addr as usize;
            }
            0x7F => {
                // TODO : Assert if it is `+0x10000` or `+0xFFFF`
                return addr.addr as usize + 0x10000;
            }
            _ => {
                Self::panic_invalid_addr(addr);
            }
        }
    }
}

impl MemoryRegion for Wram {
    fn read(&self, addr: SnesAddress) -> u8 {
        let offset = Self::map_addr(addr);

        // TODO: Just print with usize when SnesAddress PR is merged
        return self.data.get(offset as usize).copied().expect(&format!(
            "ERROR: Couldn't extract value from RAM at address: {:02X}{:04X}",
            addr.bank, addr.addr
        ));
    }

    fn write(&mut self, addr: SnesAddress, value: u8) {
        let offset = Self::map_addr(addr);
        if offset < self.data.len() {
            self.data[offset] = value;
        } else {
            Self::panic_invalid_addr(addr);
        }
    }
}

use std::panic;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_good_map_addr() {
        for bank in (0x00..=0x3F).chain(0x80..=0xBF) {
            for offset in 0..0x2000 {
                let addr: SnesAddress = SnesAddress {
                    bank: (bank),
                    addr: (offset),
                };
                assert_eq!(Wram::map_addr(addr), offset as usize);
            }
        }
    }

    #[test]
    #[should_panic]
    fn test_bad_map_addr_panics() {
        Wram::map_addr(SnesAddress {
            bank: (0x00),
            addr: (0x2000),
        });
    }

    #[test]
    #[should_panic]
    fn test_bad_map_addr_panics2() {
        Wram::map_addr(SnesAddress {
            bank: (0x0F),
            addr: (0x2000),
        });
    }
}
