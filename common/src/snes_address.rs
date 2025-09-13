/// Common struct used to represent memory addresses in the global
/// SNES adddress space.
///
/// The address space is split in 256 64Ko banks.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct SnesAddress {
    /// The bank number of the address
    pub bank: u8,

    /// The memory address within the bank
    pub addr: u16,
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
        let mut addr = SnesAddress { bank: 3, addr: u16::MAX };
        assert!(addr.increment());

        assert_eq!(addr, SnesAddress { bank: 4, addr: 0 });
    }

    #[test]
    fn test_wrapping_decrement() {
        let mut addr = SnesAddress { bank: 3, addr: 0 };
        assert!(addr.decrement());

        assert_eq!(addr, SnesAddress { bank: 2, addr: u16::MAX });
    }
}
