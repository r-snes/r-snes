use common::snes_address::SnesAddress;

pub trait MemoryRegion {
    fn read(&self, addr: SnesAddress) -> u8;

    fn write(&mut self, addr: SnesAddress, value: u8);
}
