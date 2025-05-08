pub trait MemoryRegion {
    fn read(&self, addr: u32) -> u8;
    fn write(&mut self, addr: u32, value: u8);
}

pub struct Wram {
    data: [u8; 128 * 1024], // 128 KiB WRAM
}

impl Wram {
    pub fn new() -> Self {
        Self {
            data: [0; 128 * 1024],
        }
    }
}

impl MemoryRegion for Wram {
    fn read(&self, addr: u32) -> u8 {
        let offset = (addr & 0x1FFFF) as usize; // Wraps within 128KiB
        self.data.get(offset).copied().unwrap_or(0xFF)
    }

    fn write(&mut self, addr: u32, value: u8) {
        let offset = (addr & 0x1FFFF) as usize;
        if offset < self.data.len() {
            self.data[offset] = value;
        }
    }
}

pub struct Cartridge {
    rom: Vec<u8>,
    pub is_hirom: bool,
}

impl Cartridge {
    pub fn new(rom: Vec<u8>, is_hirom: bool) -> Self {
        Self { rom, is_hirom }
    }
}

impl MemoryRegion for Cartridge {
    fn read(&self, addr: u32) -> u8 {
        let offset = if self.is_hirom {
            // HiROM maps full banks
            ((addr & 0x3FFFFF) as usize).min(self.rom.len())
        } else {
            // LoROM maps only 32KB per bank
            let bank = (addr >> 16) & 0x7F;
            let lo_offset = addr & 0x7FFF;
            ((bank * 0x8000) + lo_offset) as usize
        };
        self.rom.get(offset).copied().unwrap_or(0xFF)
    }

    fn write(&mut self, _addr: u32, _value: u8) {
        // ROM = Read Only Memory duh...
    }
}

pub struct Bus {
    wram: Wram,
    cartridge: Cartridge,
    // Add other peripherals here later
}

impl Bus {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            wram: Wram::new(),
            cartridge,
        }
    }

    pub fn read(&self, addr: u32) -> u8 {
        let bank = (addr >> 16) as u8;
        let offset = addr & 0xFFFF;

        match bank {
            0x00..=0x3F | 0x80..=0xBF => {
                if offset < 0x2000 {
                    self.wram.read(addr & 0x1FFFF)
                } else if offset >= 0x8000 {
                    self.cartridge.read(addr)
                } else {
                    0xFF // I/O or open bus
                }
            }
            0x7E..=0x7F => self.wram.read(addr),
            0x40..=0x7D | 0xC0..=0xFF => self.cartridge.read(addr),
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u32, value: u8) {
        let bank = (addr >> 16) as u8;
        let offset = addr & 0xFFFF;

        match bank {
            0x00..=0x3F | 0x80..=0xBF => {
                if offset < 0x2000 {
                    self.wram.write((addr & 0x1FFFF), value);
                } else {
                    // Ignore or log I/O
                }
            }
            0x7E..=0x7F => self.wram.write(addr, value),
            _ => {}
        }
    }
}
