use crate::memory::Memory;

pub struct Timers;

impl Timers {
    pub fn new() -> Self {
        Self
    }

    pub fn step(&mut self, _mem: &mut Memory) {
        // TODO: implement timer logic
    }
}
