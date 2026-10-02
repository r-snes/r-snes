//! Crate implementing the permission representation
//! and GUI of R-SNES plugins.
//!
//! The implementation of injected callbacks and objects
//! in the Lua VM are kept outside of this crate, so that
//! it stays independent from other parts of the project

mod derive_alias;

pub mod perm_tree;
pub mod permission;
pub mod plugin;
