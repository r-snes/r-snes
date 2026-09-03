use crate::{instrs::instr_tab::*, registers::Registers};
use common::snes_address::SnesAddress;
use instr_metalang_procmacro::cpu_instr_no_inc_pc;

/// Resumable main CPU of the SNES, a 65C816
///
/// The primary way to use this CPU is through the [`Self::cycle`] function,
/// which allows to resume execution between cycles, and inspecting
/// what kind of cycle (memory access or internal) the CPU just finished.
pub struct CPU {
    /// Internal registers accessible read/write to executed programs
    pub(crate) registers: Registers,

    /// Address bus: points to one byte in the global address space
    /// where memory I/O may occur if a read or write is executed.
    pub(crate) addr_bus: SnesAddress,

    /// Secondary address bus which may be used as a buffer in some
    /// instructions to maintain addresses between cycles
    pub(crate) addr_bus2: SnesAddress,

    /// Data bus: holds one byte that may be sent to RAM (at the address
    /// hold by the address bus) by executing a write) or coming from
    /// RAM (from the address in the address bus) right after a read has
    /// been executed.
    ///
    /// It is a public member to allow code managing the CPU to feed
    /// in bytes read from RAM into the CPU.
    pub data_bus: u8,

    /// CPU state (executing/STP/WAI)
    pub(crate) state: CPUState,

    /// Internal data bus used to store 16-bits operands before doing
    /// operations on them.
    pub(crate) internal_data_bus: u16,

    /// Member variable that holds a function pointer that will be called the next
    /// time time [`Self::cycle`] is called.
    pub(crate) next_cycle: InstrCycle,

    /// Function pointer to the cycle function to call once the current
    /// instruction finishes
    ///
    /// This is always [`opcode_fetch`] unless an interrupt has been
    /// successfully requested, in which case the interrupt will be served
    pub(crate) next_fetch: InstrCycle,
}

/// State of the CPU, hinting what may or may not happen
/// when calling the [`cycle`] function
#[derive(Default, Clone, Copy)]
pub enum CPUState {
    /// Default state of the CPU: running, executing instructions
    #[default]
    Running,

    /// Stopped by a STP instruction
    ///
    /// In this state, the CPU can only go back to the [`Running`] state
    /// by calling [`reset`].
    Stopped,

    /// Waiting for an interrupt (in a WAI instruction)
    WaitForInterrupt,
}

/// The result of a CPU cycle.
///
/// This enum is the return type of the [`CPU::cycle`] function: it is used
/// to inform the caller of what the CPU has done or I/O requests.
#[derive(Debug, PartialEq, Eq)]
pub enum CycleResult {
    /// The CPU wants to read from RAM. The caller should write in the data
    /// bus the byte contained at the address pointed to by the address bus.
    Read,

    /// The CPU wants to write to RAM. The caller should write to RAM the
    /// content of the data bus at the address pointed to by the address bus.
    Write,

    /// The CPU executes an internal cycle: it only tweaks internal registers,
    /// does not access RAM. No specific action is required from the caller.
    Internal,
}

impl CPU {
    pub fn new(registers: Registers) -> Self {
        Self {
            registers,
            addr_bus: SnesAddress::default(),
            addr_bus2: SnesAddress::default(),
            state: CPUState::default(),
            data_bus: 0,
            internal_data_bus: 0,
            next_cycle: InstrCycle(opcode_fetch),
            next_fetch: InstrCycle(opcode_fetch),
        }
    }

    /// Public getter to internal registers, can be used for tests or diagnostics
    pub fn regs(&self) -> &Registers {
        &self.registers
    }

    /// Public getter to the address bus, needs to be read by the
    /// code managing the CPU for RAM I/O
    pub fn addr_bus(&self) -> &SnesAddress {
        &self.addr_bus
    }

    /// Checks if the next cycle that will be executed is the
    /// first cycle of an instruction
    pub fn is_instr_start(&self) -> bool {
        std::ptr::fn_addr_eq(
            self.next_cycle.0,
            opcode_dispatch as for<'a> fn(&'a mut CPU) -> (CycleResult, InstrCycle),
        )
    }

    /// Execute a single CPU cycle.
    ///
    /// This function is the core part of the public API to this struct.
    /// See the following example usage:
    ///
    /// ```no_run
    /// # use cpu::cpu::{CPU, CycleResult};
    /// # use cpu::registers::Registers;
    ///
    /// let mut cpu = CPU::poweron();
    ///
    /// // Example RAM, would be much more complicated in practice
    /// let mut ram = [0u8; 65536 * 256];
    ///
    /// loop {
    ///     match cpu.cycle() {
    ///         // The CPU completed an internal cycle, no action required
    ///         CycleResult::Internal => {
    ///             // sleep for the amount of time for internal cycles
    ///         }
    ///
    ///         // The CPU wants to read from memory
    ///         CycleResult::Read => {
    ///             // Get the read address
    ///             let addr = *cpu.addr_bus();
    ///
    ///             // Read the byte from RAM
    ///             let byte = ram[((addr.bank as usize) << 16) | addr.addr as usize];
    ///
    ///             // Inject the byte from RAM into the CPU data bus
    ///             cpu.data_bus = byte;
    ///
    ///             // sleep for the amount of time depending on the read address
    ///         }
    ///
    ///         // The CPU wants to write to memory
    ///         CycleResult::Write => {
    ///             // Get the write address
    ///             let addr = *cpu.addr_bus();
    ///
    ///             // Get the byte to write
    ///             let byte = cpu.data_bus;
    ///
    ///             // Inject the byte from the CPU data bus into RAM
    ///             ram[((addr.bank as usize) << 16) | addr.addr as usize] = byte;
    ///
    ///             // sleep for the amount of time depending on the write address
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// See [`CycleResult`] for more information about the return value of
    /// this function.
    pub fn cycle(&mut self) -> CycleResult {
        let (ret, next_cycle) = (self.next_cycle.0)(self);

        self.next_cycle = next_cycle;
        ret
    }

    /// Request an NMI interrupt
    pub fn nmi(&mut self) {
        use crate::instrs::nmi;
        match self.state {
            CPUState::Running => {
                // complete the current instruction first, then serve the interrupt
                self.next_fetch = InstrCycle(nmi::nmi_cyc1);
            }
            CPUState::WaitForInterrupt => {
                // serve the interrupt immediately
                self.next_cycle = InstrCycle(nmi::nmi_cyc1);
            }
            // stopped, don't even serve interrupts
            CPUState::Stopped => {}
        }
    }

    /// Request an IRQ interrupt
    pub fn irq(&mut self) {
        use crate::instrs::irq;
        match self.state {
            CPUState::Running => {
                if self.registers.P.I {
                    // flag I disables IRQ when running normally
                    return;
                }

                // complete the current instruction first, then serve the interrupt
                self.next_fetch = InstrCycle(irq::irq_cyc1);
            }
            CPUState::WaitForInterrupt => {
                if self.registers.P.I {
                    // IRQ with I flag while in WAI resumes execution
                    // but doesn't go through the interrupt routine
                    self.next_fetch = InstrCycle(opcode_fetch);
                    self.state = CPUState::Running;
                } else {
                    // serve the interrupt immediately
                    self.next_cycle = InstrCycle(irq::irq_cyc1);
                }
            }
            // stopped, don't even serve interrupts
            CPUState::Stopped => {}
        }
    }

    /// Resets the CPU as with the RESB input signal
    ///
    /// This resets some CPU registers and jumps program execution to
    /// the address contained at 0:FFFC in bank 0
    pub fn reset(&mut self) {
        // mark the CPU as running again in case reset is hit while in STP or WAI
        self.state = CPUState::Running;

        // set the next cycle to be the reset sequence defined below
        self.next_cycle = InstrCycle(reset_cyc1);
    }

    /// Construct a freshly reset CPU, as it would be on power-on
    pub fn poweron() -> Self {
        let mut ret = Self::new(Registers::default());

        ret.reset();
        ret
    }
}

cpu_instr_no_inc_pc!(reset {
    cpu.registers.DB = 0;
    cpu.registers.D = 0;
    cpu.registers.PB = 0;

    *cpu.registers.X.hi_mut() = 0;
    *cpu.registers.Y.hi_mut() = 0;

    cpu.registers.P.M = true;
    cpu.registers.P.X = true;
    cpu.registers.P.D = false;
    cpu.registers.P.I = true;
    cpu.registers.E = true;

    cpu.addr_bus = snes_addr!(0:0xfffc);
    meta FETCH16_INTO cpu.registers.PC;

    cpu.state = CPUState::Running;
    cpu.next_fetch = InstrCycle(opcode_fetch);
});

#[cfg(test)]
mod tests {
    use crate::instrs::test_prelude::*;

    #[test]
    fn poweron() {
        let mut cpu = super::CPU::poweron();

        expect_vector_to(&mut cpu, 0xfffc);
    }

    #[test]
    fn only_reset_affects_stp() {
        let mut cpu = CPU::new(Registers::default());

        expect_opcode_fetch(&mut cpu, 0xdb);
        for _ in 0..100 {
            expect_internal_cycle(&mut cpu, "stp spin loop");
        }
        cpu.irq();
        for _ in 0..100 {
            expect_internal_cycle(&mut cpu, "stp spin loop");
        }
        cpu.nmi();
        for _ in 0..100 {
            expect_internal_cycle(&mut cpu, "stp spin loop");
        }

        cpu.reset();
        expect_vector_to(&mut cpu, 0xfffc);
    }

    }
}
