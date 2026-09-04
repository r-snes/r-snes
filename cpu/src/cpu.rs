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
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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
    use duplicate::duplicate_item;

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

    #[duplicate_item(
        DUP_name        DUP_interrupt   DUP_vector;
        [nmi_mid_nop]   [nmi]           [0xFFFA];
        [irq_mid_nop]   [irq]           [0xFFFE];
    )]
    #[test]
    fn DUP_name() {
        let mut cpu = CPU::new(Registers {
            E: true,
            S: 0x0188,
            PC: 0xeeaa,
            P: 0.into(),
            ..Default::default()
        });

        expect_opcode_fetch(&mut cpu, 0xea);
        cpu.DUP_interrupt();
        expect_internal_cycle(&mut cpu, "NOP cycle");

        expect_write_cycle(&mut cpu, snes_addr!(0:0x0188), 0xee, "save PCH");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0187), 0xab, "save PCL");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0186), 0, "save P");
        expect_vector_to(&mut cpu, DUP_vector);
    }

    /// This test runs an interrupt during a jump instruction and immediately
    /// returns from the interrupt, the expected behaviour is:
    /// - Jump instruction runs to completion, it determines a jump address
    ///   and stores it in PC
    /// - The interrupt is served:
    ///   - PC is saved on the stack (this should store
    ///     the new PC computed by the jump)
    ///   - We jump to the interrupt routine
    /// - We artificially input a RTI to return from the interrupt routine,
    ///   which should pull the saved PC back in the register
    /// - The CPU uses the pulled PC to jump to the address determined
    ///   by the jump which happened before the interrupt
    ///
    /// Technically, everything from the RTI onwards is already covered by
    /// other tests: once the PB:PC from the jump is saved in the stack and
    /// we reached the interrupt routine, the "NMI mid jump" is already
    /// successful.<br>
    /// We still test the entire scenario to show what it would look like
    #[duplicate_item(
        DUP_name        DUP_interrupt   DUP_vector;
        [nmi_mid_jump]  [nmi]           [0xFFEA];
        [irq_mid_jump]  [irq]           [0xFFEE];
    )]
    #[test]
    fn DUP_name() {
        let regs = Registers {
            PB: 4,
            PC: 7,
            E: false,
            S: 0x0244,
            P: 123.into(),
            ..Default::default()
        };
        assert!(!regs.P.I, "we need IRQ to not be disabled");
        let mut cpu = CPU::new(regs);

        expect_opcode_fetch(&mut cpu, 0x5c); // jml
        expect_read_cycle(&mut cpu, snes_addr!(4:8), 0x56, "PCL");
        cpu.DUP_interrupt(); // request an interrupt which has to be served only once the JML completes
        expect_read_cycle(&mut cpu, snes_addr!(4:9), 0x34, "PCH");
        expect_read_cycle(&mut cpu, snes_addr!(4:10), 0x12, "PB");

        expect_write_cycle(&mut cpu, snes_addr!(0:0x0244), 0x12, "save PB");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0243), 0x34, "save PCH");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0242), 0x56, "save PCL");
        expect_write_cycle(&mut cpu, snes_addr!(0:0x0241), 123, "save P");
        expect_read_cycle(&mut cpu, snes_addr!(0:DUP_vector), 0x68, "interrupt vec lo");
        expect_read_cycle(
            &mut cpu,
            snes_addr!(0:DUP_vector + 1),
            0x24,
            "interrupt vec hi",
        );

        expect_read_cycle(&mut cpu, snes_addr!(0:0x2468), 0x40, "fetch RTI opcode");
        expect_internal_cycle(&mut cpu, "RTI first idle");
        expect_internal_cycle(&mut cpu, "RTI second idle");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x0241), 123, "restore P");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x0242), 0x56, "restore PCL");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x0243), 0x34, "restore PCH");
        expect_read_cycle(&mut cpu, snes_addr!(0:0x0244), 0x12, "restore PB");

        expect_opcode_fetch_cycle(&mut cpu);
        assert_eq!(cpu.registers.PB, 0x12);
        assert_eq!(cpu.registers.PC, 0x3456);
    }
}
