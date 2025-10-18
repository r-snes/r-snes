//! Module which contains basic imports needed by all
//! files which implement instructions.
//!
//! Should be imported as `use some::path::to::prelude::*;` to automatically
//! import everything required.

pub(crate) use instr_metalang_procmacro::cpu_instr;
pub(crate) use crate::instrs::instr_tab::{InstrCycle, opcode_fetch};
pub(crate) use crate::cpu::{CPU, CycleResult};
