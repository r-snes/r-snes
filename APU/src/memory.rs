pub struct Memory {
    pub ram: [u8; 64 * 1024], // 64KB APU RAM
}

impl Memory {
    pub fn new() -> Self {
        Self { ram: [0; 64 * 1024] }
    }

    pub fn read8(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    pub fn read16(&self, addr: u16) -> u16 {
        let lo = self.read8(addr) as u16;
        let hi = self.read8(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    pub fn write8(&mut self, addr: u16, val: u8) {
        self.ram[addr as usize] = val;
    }
}
