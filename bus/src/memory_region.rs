pub trait MemoryRegion {
    #[allow(dead_code)]
    fn read(&self, addr: u32) -> u8;
    #[allow(dead_code)]
    fn write(&mut self, addr: u32, value: u8);
}
