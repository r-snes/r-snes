use apu::Apu;
use bus::Bus;
use cpu::cpu::CPU;
use cpu::cpu::CycleResult;
use cpu::registers::Registers;
use ppu::ppu::PPU;

use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

pub struct Rsnes {
    pub rom_path: PathBuf,
    pub bus: Box<Bus>,
    pub cpu: Box<CPU>,
    pub ppu: Box<PPU>,
    pub apu: Box<Apu>,
    pub master_cycles: u64,
    pub cpu_master_cycles_to_wait: u16,
}

impl Rsnes {
    pub const MASTER_CLOCK_HZ: u64 = 21_477_300;
    pub const MASTER_CYCLE_DURATION: f64 = 1.0 / Self::MASTER_CLOCK_HZ as f64;

    pub fn load_rom<P: AsRef<Path>>(rom_path: &P) -> Result<Self, Box<dyn Error>> {
        let bus = Box::new(Bus::new(rom_path)?);
        let cpu = Box::new(CPU::new(Registers::default()));
        let ppu = Box::new(PPU::new());
        let apu = Box::new(Apu::new());

        Ok(Self {
            rom_path: rom_path.as_ref().to_path_buf().clone(),
            bus,
            cpu,
            ppu,
            apu,
            master_cycles: 0,
            cpu_master_cycles_to_wait: 0,
        })
    }

    /// This function will be called every master cycle, it will either decrease the
    /// number of master cycles to wait or execute a cpu cycle
    fn update_cpu_cycles(&mut self) {
        if self.cpu_master_cycles_to_wait > 0 {
            self.cpu_master_cycles_to_wait -= 1;
            return;
        }

        match self.cpu.cycle() {
            CycleResult::Internal => {
                self.cpu_master_cycles_to_wait = 6; // TODO : Confirm internal cpu cycle is 6 master cycles
            }
            CycleResult::Read => {
                let addr = *self.cpu.addr_bus();
                let byte = self.bus.read(addr);

                self.cpu.data_bus = byte;

                // Default to 6 cycles for now
                self.cpu_master_cycles_to_wait = 6; // TODO : have the bus return the number of cycle to wait
            }
            CycleResult::Write => {
                let addr = *self.cpu.addr_bus();
                let byte = self.cpu.data_bus;

                self.bus.write(addr, byte);

                // Default to 6 cycles for now
                self.cpu_master_cycles_to_wait = 6; // TODO : have the bus return the number of cycle to wait
            }
        }
    }

    // This function will be called every master cycle, it will update the CPU, PPU and APU state accordingly
    pub fn update(&mut self) {
        self.update_cpu_cycles();
    }
}
