//! This crate implements a functioning WDC65C816 processor,
//! with cycle-accurate emulation.<br>
//! The main API you should need is the [`cpu::CPU::cycle`] function,
//! which runs the CPU for one cycle, returning information about
//! the result of the cycle.<br>
//! See the [`cpu::CPU`] for more details.
//!
//! A complete documentation of all decisions made to properly implement
//! the CPU can be found in the [`docs`] module.

pub mod registers;
pub mod cpu;
mod instrs;
mod reg;

#[cfg(doc)]
#[cfg(not(doctest))]
pub mod docs {
    #![doc = include_str!("../README.md")]
}
