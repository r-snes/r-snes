pub(crate) mod instr_tab;

pub(crate) mod prelude;
#[cfg(test)]
pub(crate) mod test_prelude;

mod arithmetic;
mod algorithms;

mod branches;
mod flags;
mod jumps;
mod loads;
mod stores;
mod uncategorised;
