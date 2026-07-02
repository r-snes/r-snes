//! Runs gilyon/snes-tests' SPC-700 test ROM against our APU, bypassing the
//! IPL boot handshake by injecting the assembled `.spc` binaries directly
//! into APU RAM rather than going through a real 65816 + IPL ROM transfer.
//!
//! This still exercises the *post-load* handshake protocol exactly as
//! documented in spctest.asm / spc_common.inc from that repo -- we just
//! play the main-CPU driver's role ourselves in Rust instead of running
//! real 65816 code, since our main CPU crate isn't complete enough yet
//! to run the real driver.
//!
//! Fixture files: place spc_tests0.spc, spc_tests1.spc, spc_tests2.spc
//! (built via `spcasm -f plain` from gilyon/snes-tests' spctest/ sources)
//! in apu/tests/fixtures/.

use apu::Apu;

/// Result of running one chunk against the APU.
enum ChunkResult {
    /// All tests in this chunk passed. Carries the last test number
    /// reached, which becomes the seed for the next chunk (chunks
    /// continue numbering where the previous one left off).
    Success { last_test_num: u16 },
    /// A test failed partway through. Carries everything the real
    /// driver would report on real hardware.
    Failure {
        test_num: u16,
        psw: u8,
        a: u8,
        x: u8,
        y: u8,
    },
}

/// Cycle budget before we give up and assume the CPU is stuck (wrong PC,
/// infinite loop bug, etc.) rather than legitimately still working
/// through tests. Each individual test is only a handful of instructions,
/// so this is generous headroom for a chunk of a few thousand tests.
const MAX_CYCLES: u64 = 50_000_000;

fn run_chunk(spc_binary: &[u8], last_test_num_seed: u16) -> ChunkResult {
    let mut apu = Apu::new();

    // Inject the raw binary directly into APU RAM. The file already
    // encodes the $0-$2FF zero-padding before the real code at $300
    // (confirmed by hand-disassembling the first bytes at that offset),
    // so a straight copy starting at RAM offset 0 is correct.
    apu.memory.ram[..spc_binary.len()].copy_from_slice(spc_binary);

    // Skip the IPL handshake entirely -- jump straight to the code's
    // entry point instead of reading the (currently nonexistent) real
    // boot vector.
    apu.cpu.regs.pc = 0x0300;

    // The real IPL ROM's first act after reset is `mov x,#$ef; mov sp,x`,
    // leaving SP at $EF (not the raw post-reset $FF) before it ever hands
    // off to user code. Tests in this suite are written assuming that
    // real post-boot state -- e.g. test #0081 deliberately places its
    // ADDW operand at $01FF, relying on the stack (which starts pushing
    // at $01EF and grows downward) never reaching that high. Skipping
    // this and leaving SP at $FF causes an unrelated `push` to clobber
    // $01FF right before it's read, which looks like a CPU bug but isn't.
    apu.cpu.regs.sp = 0xEF;

    // Play the main-CPU side of the post-load handshake (see
    // spc_common.inc's `main:`): tell the SPC "go" (port1=1) and seed
    // the last-completed test number via ports 2/3, exactly as
    // spctest.asm's load_spc does after a real transfer completes.
    apu.memory.port_in[1] = 1;
    apu.memory.port_in[2] = (last_test_num_seed & 0xFF) as u8;
    apu.memory.port_in[3] = (last_test_num_seed >> 8) as u8;

    let mut cycles = 0u64;
    loop {
        apu.step(1);
        cycles += 1;

        match apu.memory.port_out[0] {
            // Success: port2/3 hold the last test number reached.
            1 => {
                let last_test_num = (apu.memory.port_out[2] as u16)
                    | ((apu.memory.port_out[3] as u16) << 8);
                return ChunkResult::Success { last_test_num };
            }

            // Failure signalled: port1 holds PSW, port2/3 hold the
            // failing test number. Snapshot both now -- the SPC
            // overwrites ports 1-3 with A/X/Y right after it sees
            // our acknowledgement below.
            2 => {
                let psw = apu.memory.port_out[1];
                let test_num = (apu.memory.port_out[2] as u16)
                    | ((apu.memory.port_out[3] as u16) << 8);

                apu.memory.port_in[1] = 2; // acknowledge, per protocol

                loop {
                    apu.step(1);
                    cycles += 1;
                    if apu.memory.port_out[0] == 3 {
                        return ChunkResult::Failure {
                            test_num,
                            psw,
                            a: apu.memory.port_out[1],
                            x: apu.memory.port_out[2],
                            y: apu.memory.port_out[3],
                        };
                    }
                    assert!(
                        cycles <= MAX_CYCLES,
                        "APU hung waiting for A/X/Y failure report after {cycles} cycles"
                    );
                }
            }

            _ => {}
        }

        assert!(
            cycles <= MAX_CYCLES,
            "APU hung after {cycles} cycles without reporting pass or fail \
             (pc={:#06x})",
            apu.cpu.regs.pc
        );
    }
}

#[test]
fn spc700_test_rom() {
    let chunk0 = std::fs::read("tests/fixtures/spc_tests0.spc")
        .expect("spc_tests0.spc not found in apu/tests/fixtures/ -- see module docs");
    let chunk1 = std::fs::read("tests/fixtures/spc_tests1.spc")
        .expect("spc_tests1.spc not found in apu/tests/fixtures/");
    let chunk2 = std::fs::read("tests/fixtures/spc_tests2.spc")
        .expect("spc_tests2.spc not found in apu/tests/fixtures/");

    // "-1" sentinel, matching spctest.asm's `ldx #$ffff; stx last_test_num`
    // at the very start -- the first chunk's first test (#0000) expects
    // this to wrap to 0 after init_test's incw.
    let mut last_test_num = 0xFFFFu16;

    for (name, chunk) in [
        ("spc_tests0", &chunk0),
        ("spc_tests1", &chunk1),
        ("spc_tests2", &chunk2),
    ] {
        match run_chunk(chunk, last_test_num) {
            ChunkResult::Success {
                last_test_num: reached,
            } => {
                println!("{name}: all tests passed (last test #{reached:04X})");
                last_test_num = reached;
            }
            ChunkResult::Failure {
                test_num,
                psw,
                a,
                x,
                y,
            } => {
                panic!(
                    "{name}: test #{test_num:04X} FAILED -- PSW={psw:02X} A={a:02X} X={x:02X} Y={y:02X}"
                );
            }
        }
    }
}
