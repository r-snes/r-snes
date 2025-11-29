//! Module which contains basic imports needed by all
//! files which implement instructions.
//!
//! Should be imported as `use some::path::to::prelude::*;` to automatically
//! import everything required.

pub(crate) use common::snes_address::{SnesAddress, snes_addr};
pub(crate) use common::u16_split::*;
pub(crate) use crate::instrs::instr_tab::{InstrCycle, opcode_fetch};
pub(crate) use crate::cpu::{CPU, CycleResult, CycleResult::*};
