use crate::vram::VRAM;
use crate::registers::PPURegisters;

pub struct PPU {
    pub regs: PPURegisters,
    pub vram: VRAM,
    pub cgram: CGRAM,
}

impl PPU {
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x2115 => {
                self.regs.vmain = value;
                self.vram.write_vmain(value);
            }
            0x2116 => self.vram.write_vmadd_low(value),
            0x2117 => self.vram.write_vmadd_high(value),
            0x2118 => self.vram.write_vmdatal(value),
            0x2119 => self.vram.write_vmdatah(value),
            0x2139 => self.vram.write_vmdatah(value),
            0x213A => self.vram.write_vmdatah(value),
            _ => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (unimplemented register)",
                    addr, value
                );
            }
        }
    }
}
