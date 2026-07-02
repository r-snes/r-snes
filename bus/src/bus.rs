use crate::io::Io;
use crate::memory_region::MemoryRegion;
use crate::rom::Rom;
use crate::wram::Wram;
use apu::Apu;
use common::snes_address::SnesAddress;
use std::error::Error;
use std::path::Path;

/// The main CPU sees the APU's 4 communication ports at these addresses.
/// They mirror the SPC700's own $F4-$F7 (see memory.rs on the apu side).
const APU_PORT_START: u16 = 0x2140;
const APU_PORT_END: u16 = 0x2143;

pub struct Bus {
    pub wram: Wram,
    pub rom: Rom,
    pub io: Io,
}

impl Bus {
    pub fn new<P: AsRef<Path>>(rom_path: P) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            rom: Rom::load_from_file(rom_path)?,
            wram: Wram::new(),
            io: Io::new(),
        })
    }

    /// True for the banks where mirrored low-page I/O registers (including
    /// the APU ports) live. Same bank set already used below for
    /// wram/io/rom, just named so the APU check can reuse it.
    fn is_io_mirror_bank(bank: u8) -> bool {
        matches!(bank, 0x00..=0x3F | 0x80..=0xBF)
    }

    /// Read a byte as the main 65816 CPU would see it.
    ///
    /// `apu` is threaded in as a parameter rather than owned by `Bus`
    /// because `RSnes` already owns the one real `Apu` instance — passing
    /// it in avoids a second APU existing or awkward shared ownership.
    pub fn read(&self, addr: SnesAddress, apu: &mut Apu) -> u8 {
        // $2140-$2143, mirrored across banks: main CPU reads what the
        // SPC700 most recently wrote to its own $F4-$F7 (port_out).
        if Self::is_io_mirror_bank(addr.bank)
            && (APU_PORT_START..=APU_PORT_END).contains(&addr.addr)
        {
            let port = (addr.addr - APU_PORT_START) as usize;
            return apu.memory.cpu_port_read(port);
        }

        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF => match addr.addr {
                0x0000..0x2000 => self.wram.read(addr),
                0x2000..0x6000 => self.io.read(addr),
                0x6000..0x8000 => self.rom.read(addr), // TODO : Expansion port
                0x8000..=0xFFFF => self.rom.read(addr),
            },
            0x7E..=0x7F => self.wram.read(addr),
            0x40..=0x7D | 0xC0..=0xFF => self.rom.read(addr),
        }
    }

    /// Write a byte as the main 65816 CPU would.
    pub fn write(&mut self, addr: SnesAddress, value: u8, apu: &mut Apu) {
        // $2140-$2143, mirrored across banks: main CPU writes land in
        // port_in, which the SPC700 will read back at its own $F4-$F7.
        if Self::is_io_mirror_bank(addr.bank)
            && (APU_PORT_START..=APU_PORT_END).contains(&addr.addr)
        {
            let port = (addr.addr - APU_PORT_START) as usize;
            apu.memory.cpu_port_write(port, value);
            return;
        }

        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF => match addr.addr {
                0x0000..0x2000 => self.wram.write(addr, value),
                0x2000..0x6000 => self.io.write(addr, value),
                0x6000..0x8000 => self.rom.write(addr, value), // TODO : Expansion port
                0x8000..=0xFFFF => self.rom.write(addr, value),
            },
            0x7E..=0x7F => self.wram.write(addr, value),
            0x40..=0x7D | 0xC0..=0xFF => self.rom.write(addr, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom::test_rom::*;
    use common::snes_address::snes_addr;

    fn dummy_apu() -> Apu {
        Apu::new()
    }

    #[test]
    fn test_wram_read_write_through_bus() {
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();
        let mut apu = dummy_apu();

        let addr = snes_addr!(0:0x0010);
        bus.write(addr, 0x42, &mut apu);
        assert_eq!(bus.read(addr, &mut apu), 0x42);

        let addr_mirror = snes_addr!(0x80:0x0010);
        assert_eq!(bus.read(addr, &mut apu), 0x42);
        assert_eq!(bus.read(addr_mirror, &mut apu), 0x42);

        let real_addr = snes_addr!(0x7E:0x0010);
        assert_eq!(bus.read(real_addr, &mut apu), 0x42);

        bus.write(real_addr, 0x21, &mut apu);
        assert_eq!(bus.read(real_addr, &mut apu), 0x21);
        assert_eq!(bus.read(addr, &mut apu), 0x21);
        assert_eq!(bus.read(addr_mirror, &mut apu), 0x21);
    }

    #[test]
    fn test_io_read_write_through_bus() {
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();
        let mut apu = dummy_apu();

        let addr = snes_addr!(0:0x2345);
        bus.write(addr, 0x77, &mut apu);
        assert_eq!(bus.read(addr, &mut apu), 0x77);

        let addr_mirror = snes_addr!(0x9E:0x2345);
        assert_eq!(bus.read(addr, &mut apu), 0x77);
        assert_eq!(bus.read(addr_mirror, &mut apu), 0x77);
    }

    #[test]
    fn test_rom_read_write_through_bus() {
        let mut rom_data = create_valid_lorom(0x100000 * 0x40);
        rom_data[0x0001] = 0x42;
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();
        let mut apu = dummy_apu();

        let addr = snes_addr!(0:0x8001);
        assert_eq!(bus.read(addr, &mut apu), 0x42);
        bus.write(addr, 0x21, &mut apu);
        assert_eq!(bus.read(addr, &mut apu), 0x42);

        let other_addr = snes_addr!(0x40:0x8001);
        assert_eq!(bus.read(other_addr, &mut apu), 0);
        bus.write(other_addr, 0x21, &mut apu);
        assert_eq!(bus.read(other_addr, &mut apu), 0);
    }

    #[test]
    #[should_panic(expected = "ERROR: Couldn't extract value from ROM")]
    fn test_rom_read_out_of_range_panics() {
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let bus = Bus::new(&rom_path).unwrap();

        // Create an address mapped to an offset beyond the 128 KiB dummy ROM.
        let addr = snes_addr!(0x7D:0xFFFF);
        bus.rom.read(addr);
    }

    #[test]
    fn test_apu_port_read_returns_spc700_output() {
        // The main CPU reads $2140 — it should see what the SPC700 wrote
        // to its own $F4 (port_out), not anything the main CPU wrote itself.
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let bus = Bus::new(&rom_path).unwrap();
        let mut apu = dummy_apu();

        apu.memory.port_out[0] = 0xAB; // simulate SPC700 having written this
        let addr = snes_addr!(0:0x2140);
        assert_eq!(bus.read(addr, &mut apu), 0xAB);
    }

    #[test]
    fn test_apu_port_write_lands_in_port_in() {
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();
        let mut apu = dummy_apu();

        let addr = snes_addr!(0:0x2140);
        bus.write(addr, 0xCD, &mut apu);
        assert_eq!(apu.memory.port_in[0], 0xCD, "SPC700 reads this via its own $F4");
    }

    #[test]
    fn test_apu_ports_mirrored_across_banks() {
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();
        let mut apu = dummy_apu();

        let addr = snes_addr!(0x80:0x2143); // port 3, mirrored bank
        bus.write(addr, 0xEF, &mut apu);
        assert_eq!(apu.memory.port_in[3], 0xEF);
    }

    #[test]
    fn test_apu_ports_do_not_leak_into_io() {
        // Sanity check: writing a port and then reading a nearby, non-port
        // address should not be affected — the intercept only fires for
        // exactly $2140-$2143.
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();
        let mut apu = dummy_apu();

        let port_addr = snes_addr!(0:0x2140);
        bus.write(port_addr, 0x99, &mut apu);

        let io_addr = snes_addr!(0:0x2144); // just past the port range
        bus.write(io_addr, 0x55, &mut apu);
        assert_eq!(bus.read(io_addr, &mut apu), 0x55);
    }
}