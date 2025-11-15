//! Module which contains required imports and utility
//! function for writing unit tests for CPU instructions
//!
//! Should be used as `use some::path::to::test_prelude::*;`

pub(crate) use crate::registers::Registers;
pub(crate) use common::snes_address::{SnesAddress,snes_addr};

use crate::cpu::CPU;
use crate::cpu::CycleResult;

/// Same as [`expect_opcode_fetch`], but doesn't require providing an
/// opcode to inject for the next cycle. This only checks that the CPU
/// is fetching from the appropriate address
pub(crate) fn expect_opcode_fetch_cycle(cpu: &mut CPU) {
    assert_eq!(
        cpu.cycle(),
        CycleResult::Read,
        "Expecting a read cycle for an opcode fetch",
    );

    let expected_address = SnesAddress {
        bank: cpu.registers.PB,
        addr: cpu.registers.PC,
    };
    assert_eq!(
        *cpu.addr_bus(),
        expected_address,
        "Opcode fetch should be from {:#?} (current PB:PC)",
        expected_address
    );
}

/// Expects that the CPU does an opcode fetch cycle (a read cycle reading
/// from PB:PC).
pub(crate) fn expect_opcode_fetch(cpu: &mut CPU, opcode: u8) {
    expect_opcode_fetch_cycle(cpu);
    cpu.data_bus = opcode;
}

/// Expects (creates an assertion) the CPU to return an internal cycle
pub(crate) fn expect_internal_cycle(cpu: &mut CPU, reason: &str) {
    assert_eq!(
        cpu.cycle(),
        CycleResult::Internal,
        "Expecting an internal cycle for {reason}",
    );
}

/// Expects (creates an assertion) the CPU to return a Read cycle
/// from the specified address, and injects the value
/// passed as parameter to be received by the CPU for the next cycle.
pub(crate) fn expect_read_cycle(
    cpu: &mut CPU,
    expected_address: SnesAddress,
    value: u8,
    reason: &str,
) {
    assert_eq!(
        cpu.cycle(),
        CycleResult::Read,
        "Expecting a read cycle for {reason}",
    );
    assert_eq!(
        *cpu.addr_bus(),
        expected_address,
        "Read cycle for {reason} should be from {:#?}",
        expected_address
    );
    cpu.data_bus = value;
}

/// Expects (creates an assertion) the CPU to return a Write cycle
/// at the specified address, of the specified value.
pub(crate) fn expect_write_cycle(
    cpu: &mut CPU,
    expected_address: SnesAddress,
    expected_value: u8,
    reason: &str,
) {
    assert_eq!(
        cpu.cycle(),
        CycleResult::Write,
        "Expecting a write cycle for {reason}",
    );
    assert_eq!(
        *cpu.addr_bus(),
        expected_address,
        "Write cycle for {reason} should be from {:#?}",
        expected_address
    );
    assert_eq!(
        cpu.data_bus, expected_value,
        "Write cycle for {reason} at {:#?} should be of value {:#x}",
        expected_address, expected_value,
    )
}
