use crate::{instr_tab::*, registers::Registers};
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

    /// `INX` instruction: increment register X
    ///
    /// Flags set:
    /// - `Z`: whether the result is zero
    /// - `N`: whether the result is negative (if it were interpreted as signed)
    pub(crate) fn inx(&mut self) {
        self.registers.X = self.registers.X.wrapping_add(1);
        self.registers.P.Z = self.registers.X == 0;
        self.registers.P.N = self.registers.X > 0x7fff;
    }
}

#[cfg(test)]
mod test {
    use duplicate::duplicate_item;

    use super::*;

    #[test]
    fn test_1_plus_1_is_2() {
        let mut regs = Registers::default();

        regs.X = 1;
        let mut cpu = CPU::new(regs);

        cpu.inx();
        assert_eq!(cpu.regs().X, 2);
    }

    #[test]
    fn test_1_plus_1_is_2_cycle_api() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;

        regs.X = 1;
        let mut cpu = CPU::new(regs);

        assert_eq!(
            cpu.cycle(),
            CycleResult::Read,
            "Expecting a read cycle for opcode fetch"
        );
        assert_eq!(
            *cpu.addr_bus(),
            SnesAddress {
                bank: 0x12,
                addr: 0x3456
            },
            "Read query should be from address at PB:PC"
        );
        cpu.data_bus = 0xe8; // Inject the INX opcode into the CPU

        assert_eq!(
            cpu.cycle(),
            CycleResult::Internal,
            "Expecting internal cycle for register increment"
        );
        assert_eq!(cpu.regs().X, 2, "Expecting value 2 in X register");
    }

    #[test]
    fn nop_does_nothing() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        let mut regs_copy = regs.clone();

        let mut cpu = CPU::new(regs);

        assert_eq!(
            cpu.cycle(),
            CycleResult::Read,
            "Expecting a read cycle for opcode fetch"
        );
        cpu.data_bus = 0xea; // Inject the NOP opcode into the CPU

        assert_eq!(
            cpu.cycle(),
            CycleResult::Internal,
            "Expecting internal cycle for register increment"
        );

        regs_copy.PC = regs_copy.PC + 1;
        assert_eq!(cpu.registers, regs_copy, "Only PC should have been touched");
    }

    #[duplicate_item(
        DUP_instr_name DUP_set_flag DUP_opcode;
        [clv] [V] [0xb8];
        [cld] [D] [0xd8];
        [cli] [I] [0x58];
        [clc] [C] [0x18];
    )]
    #[test]
    fn DUP_instr_name() {
        let mut regs = Registers::default();
        regs.PB = 0x12;
        regs.PC = 0x3456;
        let mut regs_copy = regs.clone();

        regs.P.DUP_set_flag = true;
        let mut cpu = CPU::new(regs);

        assert_eq!(
            cpu.cycle(),
            CycleResult::Read,
            "Expecting a read cycle for opcode fetch"
        );
        cpu.data_bus = DUP_opcode; // Inject the clear opcode into the CPU

        assert_eq!(
            cpu.cycle(),
            CycleResult::Internal,
            "Expecting internal cycle for clear flag"
        );

        regs_copy.PC = regs_copy.PC + 1;
        regs_copy.P.DUP_set_flag = false;

        assert_eq!(cpu.registers, regs_copy, "Flag should be cleared");

        // Execute the instruction once more to check the flag stays clear
        assert_eq!(
            cpu.cycle(),
            CycleResult::Read,
            "Expecting a read cycle for opcode fetch"
        );
        assert_eq!(
            cpu.addr_bus,
            SnesAddress {
                bank: 0x12,
                addr: 0x3457
            },
            "Opcode fetch should be from the next byte",
        );
        regs_copy.PC = regs_copy.PC + 1;
        assert_eq!(cpu.registers, regs_copy, "Flag should stay cleared");
    }
}
