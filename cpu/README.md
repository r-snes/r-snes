This crate implements a functioning WDC65C816 processor, the
16-bit microprocessor used as the main CPU of the SNES.

This crate focuses on cycle-accurate emulation, and as such, the main
API you should need is the [`cpu::CPU::cycle`] function, which runs
the CPU for one cycle, returning information about the result of
the cycle.<br>
See the [`cpu::CPU`] for more details.

A complete documentation of all decisions made to properly implement
the CPU can be found in the [`docs`] module.
