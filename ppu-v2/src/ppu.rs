use crate::vram::VRAM;
use crate::registers::PPURegisters;

pub struct PPU {
    pub regs: PPURegisters,
    pub vram: VRAM,
}

impl PPU {
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x2115 => {
                self.regs.vmain = value;
                self.vram.write_vmain(value);
            }
            _ => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (unimplemented register)",
                    addr, value
                );
            }
        }
    }
}
