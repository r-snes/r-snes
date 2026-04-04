use apu::Apu;
use bus::Bus;
use common::snes_address::SnesAddress;
use cpu::cpu::CPU;
use cpu::cpu::CycleResult;
use ppu::ppu::PPU;
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

pub struct RSnes {
    pub _rom_path: PathBuf,
    pub bus: Bus,
    pub cpu: CPU,
    pub ppu: PPU,
    pub apu: Apu,
    pub master_cycles: u64,
    pub cpu_master_cycles_to_wait: u16,
}

impl RSnes {
    pub const MASTER_CLOCK_HZ: u64 = 21_477_300;
    pub const MASTER_CYCLE_DURATION: f64 = 1.0 / Self::MASTER_CLOCK_HZ as f64;

    pub fn load_rom<P: AsRef<Path>>(rom_path: &P) -> Result<Self, Box<dyn Error>> {
        let bus = Bus::new(rom_path)?;
        let cpu = CPU::poweron();
        let ppu = PPU::new();
        let apu = Apu::new();

        Ok(Self {
            _rom_path: rom_path.as_ref().to_path_buf().clone(),
            bus,
            cpu,
            ppu,
            apu,
            master_cycles: 0,
            cpu_master_cycles_to_wait: 0,
        })
    }

    fn dma_transfer(&mut self) {
        let mdmaen = self.bus.io.mdmaen;

        for channel_nb in 0..8 {
            if mdmaen & (1 << channel_nb) == 0 {
                continue;
            }
            self.execute_dma_channel(channel_nb);
        }

        self.bus.io.mdmaen = 0;
    }

    fn execute_dma_channel(&mut self, channel_nb: u8) {
        let ch = self.bus.io.dma_channels[channel_nb as usize];

        // Get transfer parameters from channel DMAP register
        let direction = (ch.dmap >> 7) & 1;
        let fixed = (ch.dmap >> 4) & 1;
        let decrement = (ch.dmap >> 5) & 1;
        let mode = ch.dmap & 0x07;

        let mut a_addr = SnesAddress {
            bank: ch.a1b,
            addr: ((ch.a1th as u16) << 8) | ch.a1tl as u16,
        };

        // 0x0000 means 65536 bytes, u32 needed to not overflow
        let mut remaining: u32 = {
            let raw = ((ch.dash as u16) << 8) | ch.dasl as u16;
            if raw == 0 { 0x10000 } else { raw as u32 }
        };

        let b_offsets: &[u8] = match mode {
            0 => &[0],
            1 => &[0, 1],
            2 | 6 => &[0, 0],
            3 | 7 => &[0, 0, 1, 1],
            4 => &[0, 1, 2, 3],
            5 => &[0, 1, 0, 1],
            _ => unreachable!(),
        };
        let mut pattern_idx = 0;

        while remaining > 0 {
            let b_offset = b_offsets[pattern_idx % b_offsets.len()];
            let b_addr = SnesAddress {
                bank: 0x00,
                addr: 0x2100 | ch.bbad as u16 + b_offset as u16,
            };

            if direction == 0 {
                // A-Bus to B-Bus
                let byte = self
                    .bus
                    .read(a_addr, &mut self.cpu, &mut self.ppu, &mut self.apu);
                self.bus
                    .write(b_addr, byte, &mut self.cpu, &mut self.ppu, &mut self.apu);
            } else {
                // B-Bus to A-Bus
                let byte = self
                    .bus
                    .read(b_addr, &mut self.cpu, &mut self.ppu, &mut self.apu);
                self.bus
                    .write(a_addr, byte, &mut self.cpu, &mut self.ppu, &mut self.apu);
            }

            if fixed == 0 {
                if decrement == 0 {
                    a_addr.increment();
                } else {
                    a_addr.decrement();
                }
            }

            pattern_idx += 1;
            remaining -= 1;

            // Each byte transferred takes 8 master cycles - ROUGH WAY TO HANDLE IT, TO CHANGE LATER
            self.cpu_master_cycles_to_wait += 8;
        }

        // Reset DMA channel registers
        let ch = &mut self.bus.io.dma_channels[channel_nb as usize];
        ch.dasl = 0;
        ch.dash = 0;
        ch.a1tl = a_addr.addr as u8;
        ch.a1th = (a_addr.addr >> 8) as u8;
    }

    /// This function will be called every master cycle, it will either decrease the
    /// number of master cycles to wait or execute a cpu cycle
    fn update_cpu_cycles(&mut self) {
        if self.cpu_master_cycles_to_wait > 0 {
            self.cpu_master_cycles_to_wait -= 1;
            return;
        }

        // Check for DMA start
        if self.bus.io.mdmaen != 0 {
            self.dma_transfer();
        }

        match self.cpu.cycle() {
            CycleResult::Internal => {
                self.cpu_master_cycles_to_wait = 6; // TODO : Confirm internal cpu cycle is 6 master cycles
            }
            CycleResult::Read => {
                let addr = *self.cpu.addr_bus();
                let byte = self
                    .bus
                    .read(addr, &mut self.cpu, &mut self.ppu, &mut self.apu);

                self.cpu.data_bus = byte;

                // Default to 6 cycles for now
                self.cpu_master_cycles_to_wait = 6; // TODO : have the bus return the number of cycle to wait
            }
            CycleResult::Write => {
                let addr = *self.cpu.addr_bus();
                let byte = self.cpu.data_bus;

                self.bus
                    .write(addr, byte, &mut self.cpu, &mut self.ppu, &mut self.apu);

                // Default to 6 cycles for now
                self.cpu_master_cycles_to_wait = 6; // TODO : have the bus return the number of cycle to wait
            }
        }
    }

    /// This function will be called every master cycle, it will update the CPU, PPU and APU state accordingly
    pub fn update(&mut self) {
        self.update_cpu_cycles();

        self.master_cycles += 1;
    }
}
