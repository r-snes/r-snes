use crate::{instr_tab::*, registers::Registers};
use common::snes_address::SnesAddress;

/// Resumable main CPU of the SNES, a 65C816
///
/// The primary way to use this CPU is through the [`cycle`] function,
/// which allows to resume execution between cycles, and inspecting
/// what kind of cycle (memory access or internal) the CPU just finished.
#[expect(non_snake_case, reason = "Registers are named in full caps")]
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
    pub(crate) internal_data_bus: u16,

    // /// Instruction register: in the original hardware, this holds the opcode
    // /// of the current instruction being executed; in our case, we can take
    // /// the shortcut of holding a reference to our custom Instruction type.
    // pub(crate) IR: &'static Instruction,
    // /// Timing control unit: holds how many cycles of the current
    // /// instruction have been executed
    // pub(crate) TCU: usize,

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
            // IR: INSTRUCTIONS[0xea].unwrap(), // By default hold a NOP (no-operation) to do nothing
            // TCU: 1, // NOP is only 1 cycle, so we go to an opcode fetch cycle
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
        // // check for opcode fetch cycle
        // if self.TCU == self.IR.len() {
        //     self.TCU = 0;
        //     self.addr_bus = SnesAddress {
        //         bank: self.registers.PB,
        //         addr: self.registers.PC,
        //     };
        //     return CycleResult::Read;
        // }

        // // if first cycle of an instruction, set IR according to the fetched opcode
        // if self.TCU == 0 {
        //     if let Some(instr) = INSTRUCTIONS[self.data_bus as usize] {
        //         self.IR = instr;
        //     } else {
        //         todo!("Unimplemented instruction ({:#02x})", self.data_bus);
        //     }
        // }

        // // actually run the instr cycle
        // let ret = match self.IR[self.TCU] {
        //     InstructionCycle::Read => CycleResult::Read,
        //     InstructionCycle::Write => CycleResult::Write,
        //     InstructionCycle::Internal(internal) => {
        //         internal(self);
        //         CycleResult::Internal
        //     }
        // };

        // self.TCU += 1;
        // ret

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
}
