//! Registry of every CPU cycle function, so the CPU can be saved at any cycle.
//!
//! The CPU stores what it does next as function pointers
//! ([`CPU::next_cycle`](crate::cpu::CPU) and `next_fetch`). A function
//! pointer can't be written to a file: its address changes between builds
//! and between runs. This module gives every cycle function a stable name,
//! its `module_path!()` followed by the function name (for example
//! `cpu::instrs::instr_tab::opcode_fetch`), and [`InstrCycle`] is serialized
//! as that name and deserialized by looking the pointer back up.
//!
//! The table is built at link time with [`linkme`]: each
//! [`register_cycles!`] call adds its functions to [`CYCLE_FUNCTIONS`], from
//! any module of the crate, without a central list to maintain.
//!
//! Registration rules:
//! - every function that can end up in an [`InstrCycle`] must be registered,
//!   or saving the CPU while it points to that function fails. The
//!   `cpu_instr!` proc-macro registers everything it generates; hand-written
//!   cycle functions call [`register_cycles!`] in their module;
//! - a cycle function must be a named function, not a closure: closures have
//!   no name to register;
//! - every registered function must be `#[inline(never)]`. In release
//!   builds, a small function that may be inlined can get one copy per
//!   codegen unit, each with its own address: the pointer stored by the CPU
//!   would then not match the one in the table, and saving would fail only
//!   in release.
//!
//! The opposite can happen too: in release builds the linker may fold
//! functions with identical machine code into one address (MSVC's
//! `/OPT:ICF`). [`name_of`] then returns one of their names; that is fine,
//! since every name of that address leads back to the same code.

use crate::cpu::{CPU, CycleResult};
use crate::instrs::instr_tab::InstrCycle;
use linkme::distributed_slice;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Signature of every CPU cycle function.
pub(crate) type CycleFn = fn(&mut CPU, bool) -> (CycleResult, InstrCycle);

/// One registered cycle function and its stable name.
pub(crate) struct CycleEntry {
    /// `module_path!()` of the function, `::`, then its name.
    pub(crate) name: &'static str,
    /// The function itself.
    pub(crate) func: CycleFn,
}

/// Every registered cycle function, gathered at link time.
#[distributed_slice]
pub(crate) static CYCLE_FUNCTIONS: [CycleEntry];

/// Registers cycle functions of the current module in [`CYCLE_FUNCTIONS`].
///
/// Takes a comma-separated list of function names, in scope at the call site.
macro_rules! register_cycles {
    ($($func:ident),+ $(,)?) => {
        $(
            const _: () = {
                #[::linkme::distributed_slice(crate::instrs::cycle_registry::CYCLE_FUNCTIONS)]
                static ENTRY: crate::instrs::cycle_registry::CycleEntry =
                    crate::instrs::cycle_registry::CycleEntry {
                        name: concat!(module_path!(), "::", stringify!($func)),
                        func: $func,
                    };
            };
        )+
    };
}
pub(crate) use register_cycles;

/// Name of a registered cycle function, `None` if it isn't registered.
pub(crate) fn name_of(func: CycleFn) -> Option<&'static str> {
    CYCLE_FUNCTIONS
        .iter()
        .find(|entry| std::ptr::fn_addr_eq(entry.func, func))
        .map(|entry| entry.name)
}

/// Cycle function registered under `name`, `None` if there is none.
pub(crate) fn from_name(name: &str) -> Option<CycleFn> {
    CYCLE_FUNCTIONS
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.func)
}

impl Serialize for InstrCycle {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let name = name_of(self.0).ok_or_else(|| {
            serde::ser::Error::custom(format!(
                "the CPU cycle function at {:p} is not registered (see cycle_registry)",
                self.0
            ))
        })?;
        serializer.serialize_str(name)
    }
}

impl<'de> Deserialize<'de> for InstrCycle {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        from_name(&name)
            .map(InstrCycle)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown CPU cycle function `{name}`")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn names_are_unique() {
        let mut names = HashSet::new();
        for entry in CYCLE_FUNCTIONS.iter() {
            assert!(names.insert(entry.name), "registered twice: {}", entry.name);
        }
    }

    #[test]
    fn every_entry_round_trips() {
        for entry in CYCLE_FUNCTIONS.iter() {
            let from_own_name = from_name(entry.name).unwrap();
            assert!(std::ptr::fn_addr_eq(from_own_name, entry.func));

            // With identical code folding, `name_of` may return the name of
            // another function sharing this address: it must lead back to it.
            let name = name_of(entry.func).unwrap();
            assert!(std::ptr::fn_addr_eq(from_name(name).unwrap(), entry.func));
        }
    }

    // The tests below run real instructions and check that every cycle
    // function the CPU stores can be saved.
    // The registers, flags and memory are pseudo-random, from a fixed seed:
    // every run of a test executes exactly the same instructions.

    /// `(E, M, X)` flags selecting 8 or 16-bit registers, to run every opcode in.
    /// Emulation mode forces M and X to 1.
    const REGISTER_SIZE_MODES: [(bool, bool, bool); 5] = [
        (true, true, true),
        (false, false, false),
        (false, false, true),
        (false, true, false),
        (false, true, true),
    ];

    /// Runs per opcode and register size mode, each from different random registers
    /// (so branches are both taken and not taken, pages are crossed...).
    const RUNS_PER_MODE: usize = 4;

    /// Interrupt raised while an instruction runs.
    #[derive(Clone, Copy, PartialEq)]
    enum Interrupt {
        None,
        Irq,
        Nmi,
    }

    /// Random memory and registers to run instructions on.
    struct Bench {
        rng: fastrand::Rng,
        memory: Vec<u8>,
    }

    impl Bench {
        /// Memory is 1 MB, mirrored over the 16 MB address space.
        const MEMORY_MASK: usize = 0xF_FFFF;
        /// Stops instructions that never end on their own (STP, WAI).
        const MAX_CYCLES: u32 = 100;

        fn new(seed: u64) -> Self {
            let mut rng = fastrand::Rng::with_seed(seed);
            let mut memory = vec![0; Self::MEMORY_MASK + 1];
            rng.fill(&mut memory);
            Self { rng, memory }
        }

        /// A CPU about to fetch `opcode`, with random registers in the given register size mode.
        fn cpu_before(&mut self, opcode: u8, (emulation, m, x): (bool, bool, bool)) -> CPU {
            use crate::registers::{RegisterP, Registers};

            let mut p = RegisterP::from(self.rng.u8(..));
            p.M = m;
            p.X = x;
            let cpu = CPU::new(Registers {
                A: self.rng.u16(..),
                DB: self.rng.u8(..),
                D: self.rng.u16(..),
                X: self.rng.u16(..),
                Y: self.rng.u16(..),
                P: p,
                E: emulation,
                PB: self.rng.u8(..),
                PC: self.rng.u16(..),
                S: self.rng.u16(..),
            });
            let pc = usize::from(common::snes_address::SnesAddress {
                bank: cpu.registers.PB,
                addr: cpu.registers.PC,
            });
            self.memory[pc & Self::MEMORY_MASK] = opcode;
            cpu
        }

        /// Runs `cpu` until the next instruction starts, checking after
        /// every cycle that the functions it stores are registered.
        fn run(&mut self, cpu: &mut CPU, interrupt: Interrupt, context: &str) {
            let interrupt_at = self.rng.u32(0..8);

            for cycle in 0..Self::MAX_CYCLES {
                if interrupt == Interrupt::Nmi && cycle == interrupt_at {
                    cpu.nmi();
                }
                let irqb = !(interrupt == Interrupt::Irq && cycle >= interrupt_at);

                let result = cpu.cycle_with_irqb(irqb);
                let address = usize::from(*cpu.addr_bus()) & Self::MEMORY_MASK;
                match result {
                    CycleResult::Read => cpu.data_bus = self.memory[address],
                    CycleResult::Write => self.memory[address] = cpu.data_bus,
                    CycleResult::Internal => {}
                }

                for func in [cpu.next_cycle.0, cpu.next_fetch.0] {
                    assert!(
                        name_of(func).is_some(),
                        "{context}, cycle {cycle}: unregistered cycle function"
                    );
                }

                if cycle > 0 && cpu.is_instr_start() {
                    break;
                }
            }
        }

        /// Runs every opcode in every register size mode, `RUNS_PER_MODE` times each.
        fn run_every_opcode(&mut self, interrupt: Interrupt) {
            for opcode in 0..=255u8 {
                for mode in REGISTER_SIZE_MODES {
                    for _ in 0..RUNS_PER_MODE {
                        let mut cpu = self.cpu_before(opcode, mode);
                        let context = format!("opcode {opcode:02X}, (E, M, X) = {mode:?}");
                        self.run(&mut cpu, interrupt, &context);
                    }
                }
            }
        }
    }

    #[test]
    fn every_opcode_is_registered() {
        Bench::new(1).run_every_opcode(Interrupt::None);
    }

    #[test]
    fn every_opcode_interrupted_by_irq_is_registered() {
        Bench::new(2).run_every_opcode(Interrupt::Irq);
    }

    #[test]
    fn every_opcode_interrupted_by_nmi_is_registered() {
        Bench::new(3).run_every_opcode(Interrupt::Nmi);
    }

    #[test]
    fn reset_sequence_is_registered() {
        let mut cpu = CPU::poweron();
        Bench::new(4).run(&mut cpu, Interrupt::None, "reset");
    }

    #[test]
    fn unknown_name_is_rejected() {
        assert!(from_name("cpu::instrs::nothing_cyc1").is_none());

        let bytes = postcard::to_stdvec("cpu::instrs::nothing_cyc1").unwrap();
        let error = postcard::from_bytes::<InstrCycle>(&bytes)
            .err()
            .expect("an unknown name must not deserialize");

        assert!(matches!(error, postcard::Error::SerdeDeCustom), "{error:?}");
    }
}
