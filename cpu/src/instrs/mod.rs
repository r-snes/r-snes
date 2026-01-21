pub(crate) mod instr_tab;

pub(crate) mod prelude;
#[cfg(test)]
mod test_prelude;

mod arithmetic;

mod branches;
mod flags;
mod jumps;
mod loads;
mod stores;
mod uncategorised;
