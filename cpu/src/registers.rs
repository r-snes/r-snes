use std::fmt;

/// A struct which represents the WDC 65C816's registers
#[allow(non_snake_case, reason = "We are naming register in all caps")]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Registers {
    /// The accumulator register: stores the result of most operations
    pub A: u16,

    /// Data Bank register: Default bank address for memory transfers
    pub DB: u8,

    /// Direct register: offset applied to all memory addresses for instructions
    /// which use direct addressing
    pub D: u16,

    /// The index registers: used for 2D computations or memory access
    pub X: u16,
    pub Y: u16,

    /// Processor status register: contains various CPU flags
    pub P: RegisterP,

    /// Emulation flag: indicates whether or not the CPU is in emulation mode
    pub E: bool,

    /// Program bank register: stores the bank address of current execution
    pub PB: u8,

    /// Program Counter: stores the current execution address within [`Registers::PB`]
    pub PC: u16,

    /// Stack pointer: the address of the top of the stack
    pub S: u16,
}

#[allow(non_snake_case, reason = "We are naming register in all caps")]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct RegisterP {
    /// Carry flag: typically set when an arithmetic operation "carries out"
    pub C: bool,

    /// Negative flag: set when the result of a previous instruction is negative
    pub N: bool,

    /// Overflow flag: whether an operation overflew
    pub V: bool,

    /// Zero flag: set when the result is zero
    pub Z: bool,

    // Registers below are still unused
    /// Decimal flag
    pub D: bool,

    /// IRQ Disable flag
    pub I: bool,

    /// Memory/Accumulator flag: Controls the width of the A register
    ///
    /// When set to 0, the accumulator register (A) is 16 bits wide \
    /// When set to 1, the accumulator register (A) is 8 bits wide
    ///
    /// Always set to 1 in emulation mode, reset to 1 when switching to native mode
    pub M: bool,

    /// Index flag: Controls the width of the index registers X and Y
    ///
    /// When set to 0, both X and Y are 16 bits wide \
    /// When set to 1, both X and Y are 8 bits wide
    ///
    /// When switching from 0 to 1, the high byte of both registers resets to 0
    ///
    /// The X flag only exists in native mode (E=1), replaced by the B flag in emulation mode.\
    /// Reset to 1 when switching to native mode
    /// This boolean should thus be treated as the B flag when in emulation mode
    pub X: bool,
}

/// Implementation of the default state of the CPU registers on power-on or reset
/// TODO: Make this implementation truthful to the real 65816.
impl Default for Registers {
    fn default() -> Self {
        Self {
            A: 0,
            DB: 0,
            D: 0,
            X: 0,
            Y: 0,
            P: RegisterP::default(),
            E: false,
            PB: 0,
            PC: 0,
            S: 0,
        }
    }
}

/// TODO: Make this implementation truthful to the real 65816.
impl Default for RegisterP {
    fn default() -> Self {
        Self {
            C: false,
            N: false,
            V: false,
            Z: false,
            D: false,
            I: false,
            M: false,
            X: false,
        }
    }
}

impl From<u8> for RegisterP {
    fn from(p: u8) -> Self {
        Self {
            C: p & 1 << 0 != 0,
            Z: p & 1 << 1 != 0,
            I: p & 1 << 2 != 0,
            D: p & 1 << 3 != 0,
            X: p & 1 << 4 != 0,
            M: p & 1 << 5 != 0,
            V: p & 1 << 6 != 0,
            N: p & 1 << 7 != 0,
        }
    }
}

impl Into<u8> for RegisterP {
    fn into(self) -> u8 {
        u8::from(self.C) << 0
            | u8::from(self.Z) << 1
            | u8::from(self.I) << 2
            | u8::from(self.D) << 3
            | u8::from(self.X) << 4
            | u8::from(self.M) << 5
            | u8::from(self.V) << 6
            | u8::from(self.N) << 7
    }
}

impl fmt::Debug for Registers {
    #[cfg(not(tarpaulin_include))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::write!(f, "{} ", if self.E { "Emu" } else { "Nat" })?;
        std::write!(
            f,
            "{{ A: {:#06x}, X: {:#06x}, Y: {:#06x}, DB: {:#04x}, D: {:#06x}, S: {:#06x}, PB: {:#04x}, PC: {:#06x}, P: ({:?}) }}",
            self.A,
            self.X,
            self.Y,
            self.DB,
            self.D,
            self.S,
            self.PB,
            self.PC,
            self.P,
        )
    }
}

impl fmt::Debug for RegisterP {
    #[cfg(not(tarpaulin_include))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (flag, c) in [
            (self.N, 'N'),
            (self.V, 'V'),
            (self.M, 'M'),
            (self.X, 'X'),
            (self.D, 'D'),
            (self.I, 'I'),
            (self.Z, 'Z'),
            (self.C, 'C'),
        ] {
            std::write!(f, "{}", if flag { c } else { '-' })?;
        };
        Ok(())
    }
}
