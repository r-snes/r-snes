use common::snes_address::SnesAddress;

pub trait MemoryRegion {
    fn read(&mut self, addr: SnesAddress) -> u8;

    fn write(&mut self, addr: SnesAddress, value: u8);
}
