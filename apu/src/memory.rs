use crate::dsp::Dsp;

pub struct Memory {
    pub ram: [u8; 64 * 1024], // 64KB APU RAM
    pub dsp: Dsp,             // DSP registers
}

impl Memory {
    pub fn new() -> Self {
        Self {
            ram: [0; 64 * 1024],
            dsp: Dsp::new(),
        }
    }

    pub fn read8(&self, addr: u16) -> u8 {
        if (0xF200..=0xF27F).contains(&addr) {
            self.dsp.read_reg((addr - 0xF200) as u8)  // strip base offset → 0x00–0x7F
        } else {
            self.ram[addr as usize]
        }
    }

    pub fn read16(&self, addr: u16) -> u16 {
        let lo = self.read8(addr) as u16;
        let hi = self.read8(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    pub fn write8(&mut self, addr: u16, val: u8) {
        if (0xF200..=0xF27F).contains(&addr) {
            self.dsp.write_reg((addr - 0xF200) as u8, val);  // strip base offset → 0x00–0x7F
        } else {
            self.ram[addr as usize] = val;
        }
    }

    pub fn write16(&mut self, addr: u16, value: u16) {
        let lo = (value & 0xFF) as u8;
        let hi = (value >> 8) as u8;
        self.write8(addr, lo);
        self.write8(addr.wrapping_add(1), hi);
    }
}
