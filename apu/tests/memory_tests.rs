/// Memory tests — APU address space
///
/// Covers every address region that matters for APU-internal operation:
///
///   - Normal RAM ($0000–$00EF, $0100–$EFFF): read/write/independence
///   - $F0 TEST:          write ignored, read returns 0
///   - $F1 CONTROL:       write stored, port-clear bits work
///   - $F2 DSPADDR:       latch stores 7-bit index
///   - $F3 DSPDATA:       routes through latch to DSP read_reg/write_reg
///   - $F4–$F7 CPUIO:     SPC700 write → port_out; SNES write → port_in
///   - $F8–$F9 AUXRAM:    normal RAM behaviour
///   - $FA–$FC TIMERDIV:  write stored in timer_div, read returns 0xFF
///   - $FD–$FF TIMEROUT:  read returns counter, read8_mut clears it
///   - $F200–$F27F:       direct DSP window (test-code path)
///   - read16/write16:    little-endian, correct wrapping at $FFFF
///   - cpu_port_write/read: SNES↔APU communication helpers

use apu::Memory;

// ============================================================
// Helpers
// ============================================================

/// Increment timer_out[n] directly to simulate the timer hardware firing.
/// (Timer hardware itself is tested separately in timer_tests.)
fn set_timer_out(mem: &mut Memory, timer: usize, val: u8) {
    mem.timer_out[timer] = val;
}

// ============================================================
// Normal RAM
// ============================================================

#[test]
fn test_ram_read_write_roundtrip() {
    let mut mem = Memory::new();
    mem.write8(0x0010, 0xAB);
    assert_eq!(mem.read8(0x0010), 0xAB);
}

#[test]
fn test_ram_zeroed_on_new() {
    let mem = Memory::new();
    // Spot-check a few addresses outside the I/O region
    for &addr in &[0x0000u16, 0x0050, 0x0200, 0x1000, 0xEFFE] {
        assert_eq!(mem.read8(addr), 0, "addr {addr:#06X} should be 0 on init");
    }
}

#[test]
fn test_ram_addresses_are_independent() {
    let mut mem = Memory::new();
    mem.write8(0x0100, 0x11);
    mem.write8(0x0101, 0x22);
    mem.write8(0x0200, 0x33);
    assert_eq!(mem.read8(0x0100), 0x11);
    assert_eq!(mem.read8(0x0101), 0x22);
    assert_eq!(mem.read8(0x0200), 0x33);
}

#[test]
fn test_ram_full_byte_range_stored() {
    let mut mem = Memory::new();
    for val in 0u8..=255 {
        mem.write8(0x0300, val);
        assert_eq!(mem.read8(0x0300), val, "value {val} not stored correctly");
    }
}

#[test]
fn test_ram_high_addresses_accessible() {
    let mut mem = Memory::new();
    mem.write8(0xEFFE, 0x55);
    assert_eq!(mem.read8(0xEFFE), 0x55);
}

// ============================================================
// $F0 — TEST register
// ============================================================

#[test]
fn test_f0_write_is_ignored() {
    // Writing to $F0 must not crash or corrupt neighbouring RAM.
    let mut mem = Memory::new();
    mem.write8(0x00F0, 0xFF); // should silently do nothing
    // Neighbouring addresses must be unaffected.
    assert_eq!(mem.read8(0x00EF), 0);
    assert_eq!(mem.read8(0x00F1), 0);
}

#[test]
fn test_f0_read_returns_zero() {
    let mem = Memory::new();
    assert_eq!(mem.read8(0x00F0), 0);
}

// ============================================================
// $F1 — CONTROL register
// ============================================================

#[test]
fn test_f1_write_stores_control() {
    let mut mem = Memory::new();
    mem.write8(0x00F1, 0b0000_0111); // enable all 3 timers
    assert_eq!(mem.control, 0b0000_0111);
}

#[test]
fn test_f1_read_returns_zero() {
    // $F1 is write-only on hardware; reads should return 0.
    let mut mem = Memory::new();
    mem.write8(0x00F1, 0xFF);
    assert_eq!(mem.read8(0x00F1), 0x00);
}

#[test]
fn test_f1_bit7_clears_port3_in() {
    let mut mem = Memory::new();
    mem.port_in[3] = 0xAB;
    mem.write8(0x00F1, 0x80); // bit 7 set → clear port 3 input latch
    assert_eq!(mem.port_in[3], 0, "port_in[3] must be cleared by CONTROL bit 7");
}

#[test]
fn test_f1_bit6_clears_port2_in() {
    let mut mem = Memory::new();
    mem.port_in[2] = 0xCD;
    mem.write8(0x00F1, 0x40); // bit 6 set → clear port 2 input latch
    assert_eq!(mem.port_in[2], 0, "port_in[2] must be cleared by CONTROL bit 6");
}

#[test]
fn test_f1_port_clear_bits_do_not_affect_other_ports() {
    let mut mem = Memory::new();
    mem.port_in[0] = 0x11;
    mem.port_in[1] = 0x22;
    mem.port_in[2] = 0x33;
    mem.port_in[3] = 0x44;
    mem.write8(0x00F1, 0xC0); // clear ports 2 and 3 only
    assert_eq!(mem.port_in[0], 0x11, "port 0 must be unaffected");
    assert_eq!(mem.port_in[1], 0x22, "port 1 must be unaffected");
    assert_eq!(mem.port_in[2], 0,    "port 2 must be cleared");
    assert_eq!(mem.port_in[3], 0,    "port 3 must be cleared");
}

// ============================================================
// $F2 — DSPADDR latch
// ============================================================

#[test]
fn test_f2_stores_dsp_address_latch() {
    // Verify the latch is stored by checking it is used correctly on the
    // next $F3 access — write a value and read it back through the same latch.
    let mut mem = Memory::new();
    mem.write8(0x00F2, 0x5D); // latch DIR register index
    mem.write8(0x00F3, 0x08); // write 0x08 to DIR
    // Read back via $F3 with the same latch still set
    assert_eq!(mem.read8(0x00F3), 0x08, "latch must route $F3 to the correct register");
}

#[test]
fn test_f2_masks_to_7_bits() {
    // 0xFF masked to 7 bits = 0x7F (EDL register).
    // Write a value, then latch 0x7F explicitly and confirm we read the same thing —
    // proving 0xFF and 0x7F select identical registers.
    let mut mem = Memory::new();
    mem.write8(0x00F2, 0xFF); // should behave identically to 0x7F
    mem.write8(0x00F3, 0xAB);
    // Now latch 0x7F directly and read back — must return the same value
    mem.write8(0x00F2, 0x7F);
    assert_eq!(mem.read8(0x00F3), 0xAB, "0xFF and 0x7F must select the same DSP register");
}

#[test]
fn test_f2_read_returns_current_latch() {
    let mut mem = Memory::new();
    mem.write8(0x00F2, 0x3A);
    assert_eq!(mem.read8(0x00F2), 0x3A);
}

#[test]
fn test_f2_latch_persists_across_unrelated_writes() {
    // After latching a register index, unrelated RAM and I/O writes must not
    // disturb the latch.  Verify by reading $F3 after the unrelated writes.
    let mut mem = Memory::new();
    mem.write8(0x00F2, 0x5D); // latch DIR register index
    mem.write8(0x00F3, 0x04); // write DIR = 4
    mem.write8(0x0100, 0xFF); // unrelated RAM write
    mem.write8(0x00FA, 0x20); // timer divisor write
    // $F2 latch must still point at 0x5D so $F3 reads DIR
    assert_eq!(mem.read8(0x00F3), 0x04,
        "DSP latch must not be disturbed by unrelated writes");
}

// ============================================================
// $F3 — DSPDATA (read/write via $F2 latch)
// ============================================================

#[test]
fn test_f3_write_reaches_dsp_register() {
    let mut mem = Memory::new();
    mem.write8(0x00F2, 0x5D); // select DIR register
    mem.write8(0x00F3, 0x08); // write value 8 to DIR
    assert_eq!(mem.dsp.read_reg(0x5D), 0x08, "$F3 write must update DSP register");
}

#[test]
fn test_f3_read_returns_dsp_register() {
    let mut mem = Memory::new();
    mem.dsp.write_reg(0x5D, 0x12); // set DIR directly
    mem.write8(0x00F2, 0x5D);      // latch the index
    assert_eq!(mem.read8(0x00F3), 0x12, "$F3 read must return DSP register value");
}

#[test]
fn test_f3_latch_selects_correct_register() {
    // Write two different registers via $F2/$F3 and verify both land correctly.
    let mut mem = Memory::new();

    mem.write8(0x00F2, 0x00); // voice 0 VOL L
    mem.write8(0x00F3, 0x7F);

    mem.write8(0x00F2, 0x01); // voice 0 VOL R
    mem.write8(0x00F3, 0x3F);

    assert_eq!(mem.dsp.read_reg(0x00), 0x7F, "VOL L must be 0x7F");
    assert_eq!(mem.dsp.read_reg(0x01), 0x3F, "VOL R must be 0x3F");
}

#[test]
fn test_f3_sequential_writes_with_relatch() {
    // Simulate what real SPC700 boot code does: set DIR, set KON.
    let mut mem = Memory::new();

    mem.write8(0x00F2, 0x5D); // DIR
    mem.write8(0x00F3, 0x04);

    mem.write8(0x00F2, 0x4C); // KON — must re-latch before each write
    mem.write8(0x00F3, 0x01);

    assert_eq!(mem.dsp.read_reg(0x5D), 0x04, "DIR must be 0x04");
    assert_eq!(mem.dsp.read_reg(0x4C), 0x01, "KON must be 0x01");
}

#[test]
fn test_f3_write_does_not_change_latch() {
    // Writing $F3 must not alter the latch — a second $F3 write must still
    // go to the same register without re-latching via $F2.
    let mut mem = Memory::new();
    mem.write8(0x00F2, 0x2C); // latch EVOLL ($2C)
    mem.write8(0x00F3, 0xAA); // write 0xAA to EVOLL
    mem.write8(0x00F3, 0xBB); // second write — must still hit EVOLL
    assert_eq!(mem.dsp.read_reg(0x2C), 0xBB,
        "second $F3 write must still target the latched register");
    // Also confirm a different register was NOT accidentally written
    assert_eq!(mem.dsp.read_reg(0x2D), 0x00,
        "adjacent register must be unaffected");
}

// ============================================================
// $F4–$F7 — CPU↔APU communication ports
// ============================================================

#[test]
fn test_spc700_write_to_port_stored_in_port_out() {
    let mut mem = Memory::new();
    mem.write8(0x00F4, 0x11);
    mem.write8(0x00F5, 0x22);
    mem.write8(0x00F6, 0x33);
    mem.write8(0x00F7, 0x44);
    assert_eq!(mem.port_out[0], 0x11);
    assert_eq!(mem.port_out[1], 0x22);
    assert_eq!(mem.port_out[2], 0x33);
    assert_eq!(mem.port_out[3], 0x44);
}

#[test]
fn test_spc700_reads_port_in_from_cpu() {
    // Simulate the SNES CPU writing to the APU comm ports.
    let mut mem = Memory::new();
    mem.port_in[0] = 0xAA;
    mem.port_in[1] = 0xBB;
    mem.port_in[2] = 0xCC;
    mem.port_in[3] = 0xDD;
    assert_eq!(mem.read8(0x00F4), 0xAA, "SPC700 must read SNES-written value at $F4");
    assert_eq!(mem.read8(0x00F5), 0xBB);
    assert_eq!(mem.read8(0x00F6), 0xCC);
    assert_eq!(mem.read8(0x00F7), 0xDD);
}

#[test]
fn test_port_in_and_port_out_are_independent() {
    // Writing from SPC700 side must not affect what the SPC700 reads,
    // and vice versa — they use separate storage.
    let mut mem = Memory::new();
    mem.port_in[0]  = 0x12; // SNES → APU
    mem.write8(0x00F4, 0x99); // APU → SNES (writes port_out)
    assert_eq!(mem.read8(0x00F4), 0x12, "SPC700 read must still return port_in value");
    assert_eq!(mem.port_out[0],   0x99, "port_out must hold SPC700-written value");
}

#[test]
fn test_cpu_port_write_helper_sets_port_in() {
    let mut mem = Memory::new();
    mem.cpu_port_write(0, 0xAB);
    mem.cpu_port_write(3, 0xCD);
    assert_eq!(mem.port_in[0], 0xAB);
    assert_eq!(mem.port_in[3], 0xCD);
}

#[test]
fn test_cpu_port_read_helper_returns_port_out() {
    let mut mem = Memory::new();
    mem.port_out[1] = 0x55;
    mem.port_out[2] = 0x66;
    assert_eq!(mem.cpu_port_read(1), 0x55);
    assert_eq!(mem.cpu_port_read(2), 0x66);
}

#[test]
fn test_cpu_port_helpers_out_of_range_safe() {
    let mut mem = Memory::new();
    mem.cpu_port_write(99, 0xFF); // out of range — must not panic
    assert_eq!(mem.cpu_port_read(99), 0);
}

#[test]
fn test_all_4_ports_independent() {
    let mut mem = Memory::new();
    for i in 0..4usize {
        mem.cpu_port_write(i, i as u8 * 0x11);
    }
    for i in 0..4usize {
        assert_eq!(mem.cpu_port_read(i), 0); // port_out still 0
        assert_eq!(mem.port_in[i], i as u8 * 0x11);
    }
}

// ============================================================
// $F8–$F9 — Auxiliary RAM
// ============================================================

#[test]
fn test_aux_ram_f8_read_write() {
    let mut mem = Memory::new();
    mem.write8(0x00F8, 0x42);
    assert_eq!(mem.read8(0x00F8), 0x42);
}

#[test]
fn test_aux_ram_f9_read_write() {
    let mut mem = Memory::new();
    mem.write8(0x00F9, 0x99);
    assert_eq!(mem.read8(0x00F9), 0x99);
}

#[test]
fn test_aux_ram_independent_of_neighbours() {
    let mut mem = Memory::new();
    mem.write8(0x00F8, 0xAA);
    mem.write8(0x00F9, 0xBB);
    assert_eq!(mem.read8(0x00F8), 0xAA);
    assert_eq!(mem.read8(0x00F9), 0xBB);
}

// ============================================================
// $FA–$FC — Timer divisors
// ============================================================

#[test]
fn test_timer_div_write_stored() {
    let mut mem = Memory::new();
    mem.write8(0x00FA, 0x40); // T0 divisor
    mem.write8(0x00FB, 0x80); // T1 divisor
    mem.write8(0x00FC, 0xFF); // T2 divisor
    assert_eq!(mem.timer_div[0], 0x40);
    assert_eq!(mem.timer_div[1], 0x80);
    assert_eq!(mem.timer_div[2], 0xFF);
}

#[test]
fn test_timer_div_read_returns_0xff() {
    // Timer divisors are write-only from the SPC700's perspective.
    let mut mem = Memory::new();
    mem.write8(0x00FA, 0x20);
    assert_eq!(mem.read8(0x00FA), 0xFF, "$FA must read as 0xFF (write-only register)");
    assert_eq!(mem.read8(0x00FB), 0xFF);
    assert_eq!(mem.read8(0x00FC), 0xFF);
}

#[test]
fn test_timer_div_zero_allowed() {
    // Hardware treats 0 as 256; storage must accept 0 without clamping.
    let mut mem = Memory::new();
    mem.write8(0x00FA, 0x00);
    assert_eq!(mem.timer_div[0], 0x00);
}

// ============================================================
// $FD–$FF — Timer output counters
// ============================================================

#[test]
fn test_timer_out_read_returns_counter() {
    let mut mem = Memory::new();
    set_timer_out(&mut mem, 0, 7);
    set_timer_out(&mut mem, 1, 3);
    set_timer_out(&mut mem, 2, 15);
    assert_eq!(mem.read8(0x00FD), 7);
    assert_eq!(mem.read8(0x00FE), 3);
    assert_eq!(mem.read8(0x00FF), 15);
}

#[test]
fn test_timer_out_read8_mut_clears_counter() {
    let mut mem = Memory::new();
    set_timer_out(&mut mem, 0, 9);
    let first  = mem.read8_mut(0x00FD);
    let second = mem.read8_mut(0x00FD);
    assert_eq!(first,  9, "first read must return the counter value");
    assert_eq!(second, 0, "second read must return 0 after clear-on-read");
}

#[test]
fn test_timer_out_read8_does_not_clear() {
    // Immutable read8 (used by debugger / test assertions) must NOT clear.
    let mut mem = Memory::new();
    set_timer_out(&mut mem, 1, 5);
    let _ = mem.read8(0x00FE);
    assert_eq!(mem.timer_out[1], 5, "immutable read must not clear the counter");
}

#[test]
fn test_timer_out_write_is_ignored() {
    // Timer output registers are read-only; writes must be silently dropped.
    let mut mem = Memory::new();
    set_timer_out(&mut mem, 2, 4);
    mem.write8(0x00FF, 0xFF); // attempt to overwrite
    assert_eq!(mem.timer_out[2], 4, "write to $FF must not alter timer_out[2]");
}

#[test]
fn test_timer_out_all_three_independent() {
    let mut mem = Memory::new();
    set_timer_out(&mut mem, 0, 1);
    set_timer_out(&mut mem, 1, 2);
    set_timer_out(&mut mem, 2, 3);
    // read8_mut clears each individually
    assert_eq!(mem.read8_mut(0x00FD), 1);
    assert_eq!(mem.read8_mut(0x00FE), 2);
    assert_eq!(mem.read8_mut(0x00FF), 3);
    // all should now be zero
    assert_eq!(mem.timer_out[0], 0);
    assert_eq!(mem.timer_out[1], 0);
    assert_eq!(mem.timer_out[2], 0);
}

// ============================================================
// $F200–$F27F — Direct DSP register window (test-code path)
// ============================================================

#[test]
fn test_direct_dsp_window_write_read() {
    let mut mem = Memory::new();
    mem.write8(0xF200, 0x7F); // voice 0 VOL L via direct window
    assert_eq!(mem.read8(0xF200), 0x7F);
}

#[test]
fn test_direct_dsp_window_reaches_dsp() {
    let mut mem = Memory::new();
    mem.write8(0xF200 + 0x5D, 0x08); // DIR register via direct window
    assert_eq!(mem.dsp.read_reg(0x5D), 0x08);
}

#[test]
fn test_direct_dsp_window_and_f2f3_protocol_share_same_dsp() {
    // A write via the direct window must be visible through $F3,
    // and vice versa — they're the same underlying DSP register file.
    let mut mem = Memory::new();

    // Write via direct window, read via $F2/$F3 protocol
    mem.write8(0xF200 + 0x0C, 0x55); // MVOLL via direct window
    mem.write8(0x00F2, 0x0C);
    assert_eq!(mem.read8(0x00F3), 0x55, "direct-window write must be visible via $F3");

    // Write via $F2/$F3 protocol, read via direct window
    mem.write8(0x00F2, 0x1C);
    mem.write8(0x00F3, 0x66); // MVOLR via protocol
    assert_eq!(mem.read8(0xF200 + 0x1C), 0x66, "$F3 write must be visible via direct window");
}

// ============================================================
// read16 / write16
// ============================================================

#[test]
fn test_read16_little_endian() {
    let mut mem = Memory::new();
    mem.write8(0x0200, 0x34); // low byte
    mem.write8(0x0201, 0x12); // high byte
    assert_eq!(mem.read16(0x0200), 0x1234);
}

#[test]
fn test_write16_little_endian() {
    let mut mem = Memory::new();
    mem.write16(0x0300, 0xABCD);
    assert_eq!(mem.read8(0x0300), 0xCD, "low byte must be at base address");
    assert_eq!(mem.read8(0x0301), 0xAB, "high byte must be at base+1");
}

#[test]
fn test_read16_wraps_at_0xffff() {
    // A 16-bit read at $FFFF must read $FFFF (lo) and $0000 (hi) without panic.
    let mut mem = Memory::new();
    mem.write8(0xFFFF, 0x11);
    mem.write8(0x0000, 0x22);
    let val = mem.read16(0xFFFF);
    assert_eq!(val, 0x2211);
}

#[test]
fn test_write16_wraps_at_0xffff() {
    let mut mem = Memory::new();
    mem.write16(0xFFFF, 0x5566);
    assert_eq!(mem.read8(0xFFFF), 0x66, "low byte at $FFFF");
    assert_eq!(mem.read8(0x0000), 0x55, "high byte wraps to $0000");
}
