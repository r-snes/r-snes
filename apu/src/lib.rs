//! SNES audio processing unit (APU) emulation.
//!
//! The APU is an independent subsystem of the SNES built around the Sony
//! SPC700 CPU (1.024 MHz), 64 KB of dedicated audio RAM, three hardware
//! timers and the S-DSP.

pub mod apu;
pub mod cpu;
pub mod dsp;
pub mod jingle;
pub mod memory;
pub mod timers;

pub use apu::Apu;
pub use cpu::Spc700;
pub use memory::Memory;
