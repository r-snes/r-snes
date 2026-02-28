use crate::registers::PPURegisters;
use crate::vram::VRAM;
use crate::cgram::CGRAM;

pub struct PPU {
    pub regs: PPURegisters,
    pub vram: VRAM,
    pub cgram: CGRAM,

    // Timing
    pub scanline: u16,
    pub frame_ready: bool,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            regs: PPURegisters::new(),
            vram: VRAM::new(),
            cgram: CGRAM::new(),
            scanline: 0,
            frame_ready: false,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // ==========================
            // DISPLAY
            // ==========================
            0x2100 => self.regs.inidisp = value,

            // ==========================
            // VRAM
            // ==========================
            0x2115 => {
                self.regs.vmain = value;
                self.vram.write_vmain(value);
            }
            0x2116 => self.vram.write_vmadd_low(value),
            0x2117 => self.vram.write_vmadd_high(value),
            0x2118 => self.vram.write_vmdatal(value),
            0x2119 => self.vram.write_vmdatah(value),

            // ==========================
            // CGRAM
            // ==========================
            0x2121 => self.cgram.write_addr(value),
            0x2122 => self.cgram.write_data(value),

            _ => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (unimplemented register)",
                    addr, value
                );
            }
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            // ==========================
            // CGRAM
            // ==========================
            0x213B => self.cgram.read_data(),

            _ => {
                println!(
                    "PPU READ IGNORED: ${:04X} (unimplemented register)",
                    addr
                );
                0
            }
        }
    }

    pub fn step_scanline(&mut self) {
        self.scanline += 1;

        if self.scanline >= 262 {
            self.scanline = 0;
            self.frame_ready = true;
        }
    }

    pub fn force_blank(&self) -> bool {
        (self.regs.inidisp & 0x80) != 0
    }

    pub fn brightness(&self) -> u8 {
        self.regs.inidisp & 0x0F
    }
}
