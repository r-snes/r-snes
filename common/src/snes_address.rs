use std::convert::From;

/// Common struct used to represent memory addresses in the global
/// SNES adddress space.
///
/// The address space is split in 256 64Ko banks.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SnesAddress {
    /// The bank number of the address
    pub bank: u8,

    /// The memory address within the bank
    pub addr: u16,
}

impl From<SnesAddress> for usize {
    fn from(addr: SnesAddress) -> usize {
        ((addr.bank as usize) << 16) | (addr.addr as usize)
    }
}

impl From<usize> for SnesAddress {
    /// If value is larger than 24 bits, only the lowest 24 bits are used and
    /// the excess bits are ignored.
    fn from(value: usize) -> SnesAddress {
        SnesAddress {
            bank: ((value >> 16) & 0xFF) as u8,
            addr: (value & 0xFFFF) as u16,
        }
    }
}

impl SnesAddress {
    /// Increment the memory address stored in [`self`]
    ///
    /// Returns whether the increment caused a bank change.
    pub fn increment(&mut self) -> bool {
        self.addr = self.addr.wrapping_add(1);

        if self.addr != 0 {
            return false;
        }
        self.bank = self.bank.wrapping_add(1);
        true
    }

    /// Decrement the memory address stored in [`self`]
    ///
    /// Returns whether the decrement caused a bank change.
    pub fn decrement(&mut self) -> bool {
        self.addr = self.addr.wrapping_sub(1);

        if self.addr != u16::MAX {
            return false;
        }
        self.bank = self.bank.wrapping_sub(1);
        true
    }
}

#[cfg(test)]
mod test {
    use std::u16;

    use super::*;

    #[test]
    fn test_simple_increment() {
        let mut addr = SnesAddress { bank: 0, addr: 1 };
        assert!(!addr.increment());

        assert_eq!(addr, SnesAddress { bank: 0, addr: 2 });
    }

    #[test]
    fn test_simple_decrement() {
        let mut addr = SnesAddress { bank: 0, addr: 1 };
        assert!(!addr.decrement());

        assert_eq!(addr, SnesAddress { bank: 0, addr: 0 });
    }

    #[test]
    fn test_wrapping_increment() {
        let mut addr = SnesAddress {
            bank: 3,
            addr: u16::MAX,
        };
        assert!(addr.increment());

        assert_eq!(addr, SnesAddress { bank: 4, addr: 0 });
    }

    #[test]
    fn test_wrapping_decrement() {
        let mut addr = SnesAddress { bank: 3, addr: 0 };
        assert!(addr.decrement());

        assert_eq!(
            addr,
            SnesAddress {
                bank: 2,
                addr: u16::MAX
            }
        );
    }

    #[test]
    fn test_to_usize() {
        let addr: SnesAddress = SnesAddress {
            bank: (0xE3),
            addr: (0x3F49),
        };
        let expected: usize = 0xE33F49;

        assert_eq!(usize::from(addr), expected);
    }

    #[test]
    fn test_from_usize() {
        let nb: usize = 0x124089;
        let expected: SnesAddress = SnesAddress {
            bank: (0x12),
            addr: (0x4089),
        };

        assert_eq!(SnesAddress::from(nb), expected);
    }

    #[test]
    fn test_from_usize_too_big() {
        let nb: usize = 0x12408993245;
        let expected: SnesAddress = SnesAddress {
            bank: (0x99),
            addr: (0x3245),
        };

        assert_eq!(SnesAddress::from(nb), expected);
    }

    #[test]
    fn test_from_usize_too_small() {
        let nb: usize = 0x124;
        let expected: SnesAddress = SnesAddress {
            bank: (0x00),
            addr: (0x0124),
        };

        assert_eq!(SnesAddress::from(nb), expected);
    }
}
