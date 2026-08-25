pub(crate) mod instr_tab;

pub(crate) mod prelude;
#[cfg(test)]
pub(crate) mod test_prelude;

pub(crate) use interrupts::{nmi, irq};

mod algorithms;
mod arithmetic;

mod branches;
mod flags;
mod interrupts;
mod jumps;
mod loads;
mod stack;
mod stores;
mod transfers;
mod uncategorised;
