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

    /// `CLC` instruction: clear carry flag
    pub fn clc(&mut self) -> i32 {
        self.registers.P.C = false;
        2
    }

    /// `CLD` instruction: clear decimal flag
    pub fn cld(&mut self) -> i32 {
        self.registers.P.D = false;
        2
    }

    /// `CLI` instruction: clear interrupt flag
    pub fn cli(&mut self) -> i32 {
        self.registers.P.I = false;
        2
    }

    /// `CLV` instruction: clear overflow flag
    pub fn clv(&mut self) -> i32 {
        self.registers.P.V = false;
        2
    }

    /// `SEC` instruction: set carry flag
    pub fn sec(&mut self) -> i32 {
        self.registers.P.C = true;
        2
    }

    /// `SEI` instruction: set interrupt flag
    pub fn sei(&mut self) -> i32 {
        self.registers.P.I = true;
        2
    }

    /// `SED` instruction: set decimal flag
    pub fn sed(&mut self) -> i32 {
        self.registers.P.D = true;
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
