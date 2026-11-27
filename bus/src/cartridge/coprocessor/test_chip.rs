//! Fake coprocessor used by unit tests to check the cartridge/bus plumbing

use super::{CartridgeMemory, CoprocessorChip};
use crate::bus::AccessSpeed;
use common::snes_address::SnesAddress;
use std::cell::Cell;
use std::rc::Rc;

/// Fake chip responding at two addresses of banks `$00-$3F` / `$80-$BF`:
/// - `TestChip::IO_REG` (`$2800`), inside the I/O range: a read/write register.
///   Reading it also returns the first ROM byte when the register holds `$FF`,
///   to check that the chip can see the cartridge memory.
/// - `TestChip::CART_REG` (`$6000`), inside the cartridge range: reads
///   return the number of master cycles stepped so far (low byte).
///
/// The IRQ line is asserted once `irq_after` master cycles were stepped.
/// `cycles` is shared so tests can watch it after the chip was moved into a cartridge.
#[cfg(not(tarpaulin_include))]
pub struct TestChip {
    /// Value of the register at `TestChip::IO_REG`
    pub reg: u8,
    /// Total master cycles stepped since the last reset
    pub cycles: Rc<Cell<u64>>,
    /// Number of stepped cycles after which the IRQ line is asserted
    pub irq_after: u64,
}

#[cfg(not(tarpaulin_include))]
impl TestChip {
    /// Address of the register inside the I/O range
    pub const IO_REG: u16 = 0x2800;
    /// Address of the register inside the cartridge range
    pub const CART_REG: u16 = 0x6000;

    /// Creates a test chip, returned along with a handle on its cycle counter
    pub fn new(irq_after: u64) -> (Self, Rc<Cell<u64>>) {
        let cycles = Rc::new(Cell::new(0));
        (
            Self {
                reg: 0,
                cycles: cycles.clone(),
                irq_after,
            },
            cycles,
        )
    }

    fn claims(addr: SnesAddress, local: u16) -> bool {
        matches!(addr.bank, 0x00..=0x3F | 0x80..=0xBF) && addr.addr == local
    }
}

#[cfg(not(tarpaulin_include))]
impl CoprocessorChip for TestChip {
    fn read(&mut self, addr: SnesAddress, mem: CartridgeMemory) -> Option<(u8, AccessSpeed)> {
        if Self::claims(addr, Self::IO_REG) {
            let value = if self.reg == 0xFF {
                mem.rom[0]
            } else {
                self.reg
            };
            Some((value, AccessSpeed::Fast))
        } else if Self::claims(addr, Self::CART_REG) {
            Some((self.cycles.get() as u8, AccessSpeed::Slow))
        } else {
            None
        }
    }

    fn write(
        &mut self,
        addr: SnesAddress,
        value: u8,
        _mem: CartridgeMemory,
    ) -> Option<AccessSpeed> {
        if Self::claims(addr, Self::IO_REG) {
            self.reg = value;
            Some(AccessSpeed::Fast)
        } else {
            None
        }
    }

    fn step(&mut self, master_cycles: u32) {
        self.cycles.set(self.cycles.get() + master_cycles as u64);
    }

    fn irq(&self) -> bool {
        self.cycles.get() >= self.irq_after
    }

    fn reset(&mut self) {
        self.reg = 0;
        self.cycles.set(0);
    }
}
