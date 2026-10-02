//! SNES memory bus: route CPU accesses to the WRAM, the I/O registers
//! and the cartridge, according to the global SNES memory map

pub mod bus;
pub mod cartridge;
pub mod constants;
pub mod io;
pub mod joypad;
pub mod wram;

pub use bus::Bus;
