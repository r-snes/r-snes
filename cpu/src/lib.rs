#![doc = include_str!("../README.md")]

pub mod registers;
pub mod cpu;
mod instrs;
mod reg;

#[cfg(doc)]
#[cfg(not(doctest))]
pub mod docs {
    #![doc = include_str!("../docs/README.md")]
}
