use crate::registers::{Registers};

pub struct CPU {
    registers: Registers
}

impl CPU {
    pub fn new(registers: Registers) -> Self {
        Self { registers }
    }

    pub fn regs(&self) -> &Registers {
        &self.registers
    }

    /// `INX` instruction: increment register X
    ///
    /// Flags set:
    /// - `Z`: whether the result is zero
    /// - `N`: whether the result is negative (if it were interpreted as signed)
    ///
    /// Returns the number of CPU cycles of execution
    pub fn inx(&mut self) -> i32 {
        self.registers.X = self.registers.X.wrapping_add(1);
        self.registers.P.Z = self.registers.X == 0;
        self.registers.P.N = self.registers.X > 0x7fff;

        2
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
}
