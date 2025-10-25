//! Module which contains required imports and utility
//! function for writing unit tests for CPU instructions
//!
//! Should be used as `use some::path::to::test_prelude::*;`

pub(crate) use crate::registers::Registers;
pub(crate) use common::snes_address::SnesAddress;

use crate::cpu::CPU;
use crate::cpu::CycleResult;

pub(crate) fn expect_opcode_fetch(cpu: &mut CPU, opcode: u8) {
    expect_read_cycle(
        cpu,
        SnesAddress {
            bank: cpu.registers.PB,
            addr: cpu.registers.PC,
        },
        opcode,
        "opcode fetch",
    );
}

pub(crate) fn expect_internal_cycle(cpu: &mut CPU, reason: &str) {
    assert_eq!(
        cpu.cycle(),
        CycleResult::Internal,
        "Expecting an internal cycle for {reason}",
    );
}

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
        cpu.data_bus,
        expected_value,
        "Write cycle for {reason} at {:#?} should be of value {:#x}",
        expected_address,
        expected_value,
    )
}
