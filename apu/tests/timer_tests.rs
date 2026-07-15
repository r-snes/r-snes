//! Tests for the SPC700 hardware timers.

use apu::Memory;
use apu::timers::Timers;

/// One 8 kHz base tick worth of CPU cycles.
const T01_PERIOD: u32 = 128;
/// One 64 kHz base tick worth of CPU cycles.
const T2_PERIOD: u32 = 16;

fn run(timers: &mut Timers, mem: &mut Memory, cycles: u32) {
    for _ in 0..cycles {
        timers.step(mem);
    }
}

#[test]
fn test_timer0_counts_at_8khz_with_divisor() {
    let mut mem = Memory::new();
    let mut timers = Timers::new();
    mem.write8(0x00FA, 2); // divisor 2
    mem.write8(0x00F1, 0x81); // keep IPL ROM bit, enable timer 0

    // Two 8 kHz ticks needed per output increment.
    run(&mut timers, &mut mem, T01_PERIOD * 2);
    assert_eq!(mem.timer_out[0], 1, "one increment after div*period cycles");
    run(&mut timers, &mut mem, T01_PERIOD * 2 * 3);
    assert_eq!(mem.timer_out[0], 4);
}

#[test]
fn test_timer2_runs_at_64khz() {
    let mut mem = Memory::new();
    let mut timers = Timers::new();
    mem.write8(0x00FC, 1); // divisor 1: output increments every base tick
    mem.write8(0x00F1, 0x84); // enable timer 2

    run(&mut timers, &mut mem, T2_PERIOD * 5);
    assert_eq!(mem.timer_out[2], 5, "timer 2 base clock is 16 CPU cycles");
}

#[test]
fn test_divisor_zero_means_256() {
    let mut mem = Memory::new();
    let mut timers = Timers::new();
    mem.write8(0x00FA, 0); // divisor 0 -> 256
    mem.write8(0x00F1, 0x81);

    run(&mut timers, &mut mem, T01_PERIOD * 255);
    assert_eq!(mem.timer_out[0], 0, "no increment before 256 base ticks");
    run(&mut timers, &mut mem, T01_PERIOD);
    assert_eq!(mem.timer_out[0], 1);
}

#[test]
fn test_output_counter_is_4_bit_and_wraps() {
    let mut mem = Memory::new();
    let mut timers = Timers::new();
    mem.write8(0x00FA, 1);
    mem.write8(0x00F1, 0x81);

    run(&mut timers, &mut mem, T01_PERIOD * 15);
    assert_eq!(mem.timer_out[0], 15);
    run(&mut timers, &mut mem, T01_PERIOD);
    assert_eq!(mem.timer_out[0], 0, "4-bit counter must wrap 15 -> 0");
}

#[test]
fn test_disabled_timer_does_not_count() {
    let mut mem = Memory::new();
    let mut timers = Timers::new();
    mem.write8(0x00FA, 1);
    // timer 0 NOT enabled
    run(&mut timers, &mut mem, T01_PERIOD * 32);
    assert_eq!(mem.timer_out[0], 0);
}

#[test]
fn test_enable_edge_resets_stage_and_output() {
    let mut mem = Memory::new();
    let mut timers = Timers::new();
    mem.write8(0x00FA, 4);
    mem.write8(0x00F1, 0x81);

    // Accumulate some count, then disable mid-period.
    run(&mut timers, &mut mem, T01_PERIOD * 4 + T01_PERIOD * 2);
    assert_eq!(mem.timer_out[0], 1);
    mem.write8(0x00F1, 0x80); // disable
    run(&mut timers, &mut mem, T01_PERIOD * 8);
    assert_eq!(mem.timer_out[0], 1, "must not count while disabled");

    // Re-enable: output resets, and a full div*period elapses before
    // the next increment (stage was reset too).
    mem.write8(0x00F1, 0x81);
    run(&mut timers, &mut mem, 1); // let the edge register
    assert_eq!(mem.timer_out[0], 0, "enable edge must clear the output");
    run(&mut timers, &mut mem, T01_PERIOD * 4 - 1);
    assert_eq!(mem.timer_out[0], 1);
}

#[test]
fn test_read_clears_output_via_read8_mut() {
    let mut mem = Memory::new();
    let mut timers = Timers::new();
    mem.write8(0x00FA, 1);
    mem.write8(0x00F1, 0x81);

    run(&mut timers, &mut mem, T01_PERIOD * 3);
    assert_eq!(mem.read8_mut(0x00FD), 3, "CPU read returns the count");
    assert_eq!(mem.read8_mut(0x00FD), 0, "and clears it");
}
