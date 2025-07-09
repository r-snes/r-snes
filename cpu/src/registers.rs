/// A struct which represents the WDC 65C816's registers
#[allow(non_snake_case, reason = "We are naming register in all caps")]
#[derive(Debug)]
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

    /// Program bank register: stores the bank address of current execution
    pub PB: u8,

    /// Program Counter: stores the current execution address within [`Registers::PB`]
    pub PC: u16,

    /// Stack pointer: the address of the top of the stack
    pub S: u16,
}

#[allow(non_snake_case, reason = "We are naming register in all caps")]
#[derive(Debug)]
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

    /// Memory/Accumulator flag
    pub M: bool,

    /// Index flag
    pub X: bool,

    /// Emulation flag: whether the CPU is in 8-bit compatibility mode
    pub E: bool,

    /// Break flag
    pub B: bool,
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
            E: false,
            B: false,
        }
    }
}
