/// A struct which represents the WDC 65C816's registers
#[derive(Debug)]
pub struct Registers {}

/// Implementation of the default state of the CPU registers on power-on or reset
impl Default for Registers {
    fn default() -> Self {
        Self {}
    }
}
