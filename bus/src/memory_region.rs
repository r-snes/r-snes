use common::snes_address::SnesAddress;

pub trait MemoryRegion {
    #[allow(dead_code)]
    fn read(&self, addr: SnesAddress) -> u8;

    #[allow(dead_code)]
    fn write(&mut self, addr: SnesAddress, value: u8);
}
