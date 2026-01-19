use crate::vram::Vram;

/// Represents the Picture Processing Unit (PPU).
///
/// The PPU exposes a set of memory-mapped registers that control
/// access to internal video-related resources such as VRAM.
/// This structure acts as an interface between the bus and VRAM.
pub struct Ppu {
    /// Video RAM instance used for storing graphical data.
    pub vram: Vram,
}

impl Ppu {
    /// Creates a new PPU instance with an initialized VRAM.
    pub fn new() -> Self {
        Self {
            vram: Vram::new(),
        }
    }

    /// Writes a value to a PPU register.
    ///
    /// The meaning of the write depends on the register address.
    /// Some registers control the VRAM address, while others
    /// write data through the VRAM data port.
    ///
    /// Writes to unsupported registers are ignored.
    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            // OAM data port
            0x2104 => self.oam.write_data(value),

            // VRAM address
            0x2115 => self.vram.write_vmain(value),
            0x2116 => self.vram.write_addr_low(value),
            0x2117 => self.vram.write_addr_high(value),
            // VRAM data port
            0x2118 => self.vram.write_data_low(value),
            0x2119 => self.vram.write_data_high(value),

            // Unhandled registers
            _ => {}
        }
    }

    /// Reads a value from a PPU register.
    ///
    /// Some registers return data from internal buffers,
    /// such as the VRAM data read ports.
    ///
    /// Reads from unsupported registers return 0.
    pub fn read_register(&mut self, addr: u16) -> u8 {
        match addr {
            // OAM data port
            0x2105 => self.oam.read_data(),

            // VRAM data ports
            0x2139 => self.vram.read_data_low(),
            0x213A => self.vram.read_data_high(),

            // Unhandled registers
            _ => 0,
        }
    }
}
