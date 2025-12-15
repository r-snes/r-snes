use crate::ppu::Ppu;

/// Represents the main communication bus between the CPU and the PPU.
///
/// The bus is responsible for routing CPU read and write operations
/// to the appropriate hardware component based on the accessed address.
/// In this simplified setup, only PPU register accesses are handled.
pub struct Bus {
    /// Picture Processing Unit instance.
    ///
    /// This field exposes the PPU so that the bus can forward
    /// register reads and writes to it.
    pub ppu: Ppu,
}

impl Bus {
    /// Creates a new bus with a freshly initialized PPU.
    ///
    /// All internal components start in their default state.
    pub fn new() -> Self {
        Self { ppu: Ppu::new() }
    }

    /// Handles a CPU write to the given address.
    ///
    /// If the address falls within the PPU register range,
    /// the write is forwarded to the PPU. Writes to other
    /// address ranges are currently ignored.
    pub fn cpu_write(&mut self, addr: u16, value: u8) {
        if (0x2100..=0x213F).contains(&addr) {
            self.ppu.write_register(addr, value);
        }
    }

    /// Handles a CPU read from the given address.
    ///
    /// If the address falls within the PPU register range,
    /// the read is forwarded to the PPU and its result is returned.
    /// Reads from other address ranges return 0.
    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        if (0x2100..=0x213F).contains(&addr) {
            self.ppu.read_register(addr)
        } else {
            0
        }
    }
}
