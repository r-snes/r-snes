use crate::constants::WRAM_SIZE;
use crate::memory_region::MemoryRegion;

pub struct Wram {
    data: [u8; WRAM_SIZE], // 128 KiB WRAM
}

impl Wram {
    pub fn new() -> Self {
        Self {
            data: [0; WRAM_SIZE],
        }
    }

    fn panic_invalid_addr(addr: u32) -> ! {
        panic!("Incorrect access to the WRAM at address: {:06X}", addr);
    }

    fn map_addr(addr: u32) -> usize {
        let bank = (addr >> 16) as u8;
        let offset = addr & 0xFFFF;

        match bank {
            0x00..=0x3F | 0x80..=0xBF => {
                if offset < 0x2000 {
                    return offset as usize;
                } else {
                    Self::panic_invalid_addr(addr);
                }
            }
            0x7E => {
                return offset as usize;
            }
            0x7F => {
                // TODO : Assert if it is `+0x10000` or `+0xFFFF`
                return (offset + 0x10000) as usize;
            }
            _ => {
                Self::panic_invalid_addr(addr);
            }
        }
    }
}

impl MemoryRegion for Wram {
    fn read(&self, addr: u32) -> u8 {
        let offset = Self::map_addr(addr);

        return self.data.get(offset as usize).copied().expect(&format!(
            "ERROR: Couldn't extract value from RAM at address: {:06X}",
            addr
        ));
    }

    fn write(&mut self, addr: u32, value: u8) {
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
                let addr: u32 = ((bank as u32) << 16) | (offset as u32);
                // eprintln!("{:X}", addr);
                assert_eq!(Wram::map_addr(addr), offset);
            }
        }
    }

    #[test]
    #[should_panic]
    fn test_bad_map_addr_panics() {
        Wram::map_addr(0x2000);
    }

    #[test]
    #[should_panic]
    fn test_bad_map_addr_panics2() {
        Wram::map_addr(0x0F2000);
    }
}
