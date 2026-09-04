//! Module which contains required imports and utility
//! function for writing unit tests for CPU instructions
//!
//! Should be used as `use some::path::to::test_prelude::*;`

pub(crate) use crate::cpu::{CPU, CycleResult};
pub(crate) use crate::registers::Registers;
pub(crate) use common::snes_address::{SnesAddress, snes_addr};
pub(crate) use common::u16_split::*;

fn describe_cycle(cpu: &CPU, cyc_type: CycleResult) -> String {
    match cyc_type {
        CycleResult::Internal => "Internal".to_owned(),
        CycleResult::Read => format!("Read at {:?}", cpu.addr_bus),
        CycleResult::Write => format!("Write of {:#.2x} at {:?}", cpu.data_bus, cpu.addr_bus),
    }
}

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
    let cyc_type = cpu.cycle();
    assert_eq!(
        cyc_type,
        CycleResult::Internal,
        "Expecting an internal cycle for {reason}, but got {}",
        describe_cycle(cpu, cyc_type),
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
    let cyc_type = cpu.cycle();
    assert_eq!(
        cyc_type,
        CycleResult::Read,
        "Expecting a read cycle for {reason}, but got {}",
        describe_cycle(cpu, cyc_type),
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
    let cyc_type = cpu.cycle();
    assert_eq!(
        cyc_type,
        CycleResult::Write,
        "Expecting a write cycle for {reason}, but got {}",
        describe_cycle(cpu, cyc_type),
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

/// Expects (creates an assertion) the CPU to read an interrupt vector,
/// and jump to it, testing 3 cycles:
/// - read interrupt vector lo
/// - read interrupt vector hi
/// - opcode fetch at the address contained in the interrupt vector
pub(crate) fn expect_vector_to(cpu: &mut CPU, vector: u16) {
    expect_read_cycle(cpu, snes_addr!(0:vector), 0x68, "start address lo");
    expect_read_cycle(cpu, snes_addr!(0:vector + 1), 0x24, "start address hi");
    expect_opcode_fetch_cycle(cpu);

    // we only test the CPU started fetching an opcode from the provided address
    assert_eq!(cpu.regs().PC, 0x2468);
    assert_eq!(cpu.regs().PB, 0);
}
