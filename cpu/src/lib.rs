#![doc = include_str!("../README.md")]

pub mod cpu;
mod instrs;
mod reg;
pub mod registers;

#[cfg(doc)]
#[cfg(not(doctest))]
pub mod docs {
    #![doc = include_str!("../docs/README.md")]
}
