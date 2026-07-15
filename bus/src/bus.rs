use crate::io::Io;
use crate::rom::Rom;
use crate::wram::Wram;
use apu::Apu;
use common::snes_address::SnesAddress;
use duplicate::duplicate;
use ppu::ppu::PPU;
use std::error::Error;
use std::path::Path;

pub struct Bus {
    pub wram: Wram,
    pub rom: Rom,
    pub io: Io,
}

impl Bus {
    pub fn new<P: AsRef<Path>>(rom_path: P) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            rom: Rom::load_from_file(rom_path)?,
            wram: Wram::default(),
            io: Io::default(),
        })
    }

    duplicate! {
        [
            DUP_method  DUP_parameters                                  DUP_return_t    DUP_method_param;
            [ read ]    [ &mut self, addr: SnesAddress ]                [ u8 ]          [ addr ];
            [ write ]   [ &mut self, addr: SnesAddress, value: u8 ]     [ () ]          [ addr, value ];
        ]
        pub fn DUP_method(DUP_parameters, ppu: &mut PPU, apu: &mut Apu) -> DUP_return_t {
            match addr.bank {
                0x00..=0x3F | 0x80..=0xBF => match addr.addr {
                    0x0000..0x2000 => self.wram.DUP_method(DUP_method_param),
                    0x2000..0x6000 => self.io.DUP_method(DUP_method_param, ppu, apu),
                    0x6000..0x8000 => self.rom.DUP_method(DUP_method_param), // TODO : Expansion port
                    0x8000..=0xFFFF => self.rom.DUP_method(DUP_method_param),
                },
                0x7E..=0x7F => self.wram.DUP_method(DUP_method_param),
                0x40..=0x7D | 0xC0..=0xFF => self.rom.DUP_method(DUP_method_param),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom::test_rom::*;
    use common::snes_address::snes_addr;

    fn init_extern_components() -> (PPU, Apu) {
        let ppu = PPU::new();
        let apu = Apu::new();

        (ppu, apu)
    }

    #[test]
    fn test_wram_read_write_through_bus() {
        let (mut ppu, mut apu) = init_extern_components();
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();

        let addr = snes_addr!(0:0x0010);
        bus.write(addr, 0x42, &mut ppu, &mut apu);
        assert_eq!(bus.read(addr, &mut ppu, &mut apu), 0x42);

        let addr_mirror = snes_addr!(0x80:0x0010);
        assert_eq!(bus.read(addr, &mut ppu, &mut apu), 0x42);
        assert_eq!(bus.read(addr_mirror, &mut ppu, &mut apu), 0x42);

        let real_addr = snes_addr!(0x7E:0x0010);
        assert_eq!(bus.read(real_addr, &mut ppu, &mut apu), 0x42);

        bus.write(real_addr, 0x21, &mut ppu, &mut apu);
        assert_eq!(bus.read(real_addr, &mut ppu, &mut apu), 0x21);
        assert_eq!(bus.read(addr, &mut ppu, &mut apu), 0x21);
        assert_eq!(bus.read(addr_mirror, &mut ppu, &mut apu), 0x21);
    }

    #[test]
    fn test_io_read_write_through_bus() {
        let (mut ppu, mut apu) = init_extern_components();
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();

        bus.io.open_bus = 0x20;
        let addr = snes_addr!(0:0x5000);
        let read_value = bus.read(addr, &mut ppu, &mut apu);
        assert_eq!(read_value, 0x20);

        bus.write(addr, 0x40, &mut ppu, &mut apu);
        let read_value = bus.read(addr, &mut ppu, &mut apu);
        assert_eq!(read_value, 0x40);
    }

    #[test]
    fn test_rom_read_write_through_bus() {
        let (mut ppu, mut apu) = init_extern_components();
        let mut rom_data = create_valid_lorom(0x100000 * 0x40);
        rom_data[0x0001] = 0x42;
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();

        let addr = snes_addr!(0:0x8001);
        assert_eq!(bus.read(addr, &mut ppu, &mut apu), 0x42);
        bus.write(addr, 0x21, &mut ppu, &mut apu);
        assert_eq!(bus.read(addr, &mut ppu, &mut apu), 0x42);

        let other_addr = snes_addr!(0x40:0x8001);
        assert_eq!(bus.read(other_addr, &mut ppu, &mut apu), 0);
        bus.write(other_addr, 0x21, &mut ppu, &mut apu);
        assert_eq!(bus.read(other_addr, &mut ppu, &mut apu), 0);
    }

    #[test]
    #[should_panic(expected = "ERROR: Couldn't extract value from ROM")]
    fn test_rom_read_out_of_range_panics() {
        let (mut ppu, mut apu) = init_extern_components();
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();

        // Create an address mapped to an offset beyond the 128 KiB dummy ROM.
        let addr = snes_addr!(0x7D:0xFFFF);
        bus.read(addr, &mut ppu, &mut apu);
    }

    // ---- APU communication port tests (from the APU link branch) ----
    // The port handling itself now lives in Io ($2140-$2143 within the
    // 0x2000..0x6000 range routed to io.read/io.write with `apu`), but
    // these tests still exercise the full path through the Bus and
    // verify the CPU<->SPC700 port protocol end to end.

    #[test]
    fn test_apu_port_read_returns_spc700_output() {
        // The main CPU reads $2140 — it should see what the SPC700 wrote
        // to its own $F4 (port_out), not anything the main CPU wrote itself.
        let (mut ppu, mut apu) = init_extern_components();
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();

        apu.memory.port_out[0] = 0xAB; // simulate SPC700 having written this
        let addr = snes_addr!(0:0x2140);
        assert_eq!(bus.read(addr, &mut ppu, &mut apu), 0xAB);
    }

    #[test]
    fn test_apu_port_write_lands_in_port_in() {
        let (mut ppu, mut apu) = init_extern_components();
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();

        let addr = snes_addr!(0:0x2140);
        bus.write(addr, 0xCD, &mut ppu, &mut apu);
        assert_eq!(
            apu.memory.port_in[0], 0xCD,
            "SPC700 reads this via its own $F4"
        );
    }

    #[test]
    fn test_apu_ports_mirrored_across_banks() {
        let (mut ppu, mut apu) = init_extern_components();
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();

        let addr = snes_addr!(0x80:0x2143); // port 3, mirrored bank
        bus.write(addr, 0xEF, &mut ppu, &mut apu);
        assert_eq!(apu.memory.port_in[3], 0xEF);
    }

    #[test]
    fn test_apu_ports_do_not_leak_into_io() {
        // $2140-$217F are ALL APU ports (the four CPUIO registers are
        // mirrored every 4 bytes, matching hardware), so a "nearby" address
        // like $2144 is still a port. Probe a genuinely unrelated, readable
        // I/O register instead: DMAP0 at $4300.
        let (mut ppu, mut apu) = init_extern_components();
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();

        let port_addr = snes_addr!(0:0x2140);
        bus.write(port_addr, 0x99, &mut ppu, &mut apu);

        let io_addr = snes_addr!(0:0x4300); // DMAP0 — real register storage
        bus.write(io_addr, 0x55, &mut ppu, &mut apu);

        assert_eq!(bus.read(io_addr, &mut ppu, &mut apu), 0x55);
        assert_eq!(
            bus.io.dma_channels[0].dmap, 0x55,
            "write went to the DMA register"
        );
        assert_eq!(apu.memory.port_in[0], 0x99, "port write stayed in the APU");
    }
}
