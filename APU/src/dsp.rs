use crate::memory::Memory;

pub struct Dsp;

impl Dsp {
    pub fn new() -> Self {
        Self
    }

    pub fn step(&mut self, _mem: &mut Memory) {
        // TODO: implement DSP tick (process voices, echo, etc.)
    }
}
