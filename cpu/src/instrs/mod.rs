pub mod instr_tab;

pub mod prelude;
#[cfg(test)]
pub(crate) mod test_prelude;

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
pub mod uncategorised;
