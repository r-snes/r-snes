use crate::{
    instrs::instr_tab::*,
    registers::Registers,
};
use common::snes_address::SnesAddress;

/// Resumable main CPU of the SNES, a 65C816
///
/// The primary way to use this CPU is through the [`cycle`] function,
/// which allows to resume execution between cycles, and inspecting
/// what kind of cycle (memory access or internal) the CPU just finished.
pub struct CPU {
    /// Internal registers accessible read/write to executed programs
    pub(crate) registers: Registers,

    /// Address bus: points to one byte in the global address space
    /// where memory I/O may occur if a read or write is executed.
    pub(crate) addr_bus: SnesAddress,

    /// Data bus: holds one byte that may be sent to RAM (at the address
    /// hold by the address bus) by executing a write) or coming from
    /// RAM (from the address in the address bus) right after a read has
    /// been executed.
    ///
    /// It is a public member to allow code managing the CPU to feed
    /// in bytes read from RAM into the CPU.
    pub data_bus: u8,

    /// Internal data bus used to store 16-bits operands before doing
    /// operations on them.
    #[expect(dead_code, reason = "not used by any of the currently impl'd instrs")]
    pub(crate) internal_data_bus: u16,

    /// Member variable that holds a function pointer that will be called the next
    /// time time [`Self::cycle`] is called.
    pub(crate) next_cycle: InstrCycle,
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
            data_bus: 0,
            internal_data_bus: 0,
            next_cycle: InstrCycle(opcode_fetch),
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

    /// Execute a single CPU cycle.
    ///
    /// This function is the core part of the public API to this struct.
    /// See the following example usage:
    ///
    /// ```rs
    /// let mut cpu = CPU::new(Registers::default);
    ///
    /// // Example RAM, would be much more complicated in practice
    /// let mut ram = [0u8; 65536 * 256];
    ///
    /// loop {
    ///     match cpu.cycle() {
    ///         // The CPU completed an internal cycle, no action required
    ///         CycleResult::Internal => {
    ///             // sleep for the amount of time for internal cycles
    ///         },
    ///
    ///         // The CPU wants to read from memory
    ///         CycleResult::Read => {
    ///             // Get the read address
    ///             let addr = *cpu.address_bus();
    ///
    ///             // Read the byte from RAM
    ///             let byte = ram[(addr.bank << 16) | addr.addr];
    ///
    ///             // Inject the byte from RAM into the CPU data bus
    ///             cpu.data_bus = byte;
    ///
    ///             // sleep for the amount of time depending on the read address
    ///         },
    ///
    ///         // The CPU wants to write to memory
    ///         CycleResult::Write => {
    ///             // Get the write address
    ///             let addr = *cpu.address_bus();
    ///
    ///             // Get the byte to write
    ///             let byte = cpu.data_bus;
    ///
    ///             // Inject the byte from the CPU data bus into RAM
    ///             ram[(addr.bank << 16) | addr.addr] = byte;
    ///
    ///             // sleep for the amount of time depending on the write address
    ///         }
    ///     },
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
}
