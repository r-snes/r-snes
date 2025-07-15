pub trait MemoryRegion {
    fn read(&self, addr: u32) -> u8;
    fn write(&mut self, addr: u32, value: u8);
}
