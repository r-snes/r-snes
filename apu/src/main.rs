//! SPC player — runs a `.spc` snapshot through the APU on its own.
//!
//! A `.spc` file is a frozen APU: all 64 KB of APU RAM (which already holds
//! the game's sound driver, song data and samples), the 128 DSP registers,
//! and the SPC700 registers. Loading it and letting the SPC700 run plays the
//! song with the game's own driver and no main SNES CPU involved, so:
//!
//!   - .spc sounds right, game doesn't  → upload / CPU↔APU sync problem
//!   - .spc sounds wrong too            → SPC700, timers, or DSP problem
//!
//! Usage:
//!   cargo run --release -- <file.spc> [seconds] [out.wav]
//!
//! Defaults: 30 seconds, output next to the input with a .wav extension.

use apu::Apu;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

const SAMPLE_RATE: u32 = 32_000;

// ============================================================
// .SPC FILE LAYOUT (v0.30)
// ============================================================

const SPC_MAGIC: &[u8] = b"SNES-SPC700 Sound File Data";
const SPC_MIN_LEN: usize = 0x1_0200;

const OFF_HAS_ID666: usize = 0x23; // 26 = ID666 tag present, 27 = absent
const OFF_PC: usize = 0x25; // 2 bytes, little-endian
const OFF_A: usize = 0x27;
const OFF_X: usize = 0x28;
const OFF_Y: usize = 0x29;
const OFF_PSW: usize = 0x2A;
const OFF_SP: usize = 0x2B;
const OFF_SONG_TITLE: usize = 0x2E; // 32 bytes
const OFF_GAME_TITLE: usize = 0x4E; // 32 bytes
const OFF_RAM: usize = 0x100; // 64 KB
const OFF_DSP: usize = 0x1_0100; // 128 bytes
const OFF_EXTRA_RAM: usize = 0x1_01C0; // 64 bytes: the RAM hidden under the IPL ROM ($FFC0–$FFFF)

/// DSP registers that must not be restored blindly in the main loop.
const DSP_KON: u8 = 0x4C; // written last, after everything else is in place
const DSP_ENDX: u8 = 0x7C; // any write clears it, so restoring it is meaningless

struct SpcFile {
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    psw: u8,
    sp: u8,
    ram: Vec<u8>,
    dsp: [u8; 128],
    extra_ram: [u8; 64],
    song_title: String,
    game_title: String,
}

impl SpcFile {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < SPC_MIN_LEN {
            return Err(format!(
                "file is {} bytes, a .spc needs at least {SPC_MIN_LEN}",
                bytes.len()
            ));
        }
        if !bytes.starts_with(SPC_MAGIC) {
            return Err("missing \"SNES-SPC700 Sound File Data\" header".into());
        }

        let (song_title, game_title) = if bytes[OFF_HAS_ID666] == 26 {
            (
                tag_string(&bytes[OFF_SONG_TITLE..OFF_SONG_TITLE + 32]),
                tag_string(&bytes[OFF_GAME_TITLE..OFF_GAME_TITLE + 32]),
            )
        } else {
            (String::new(), String::new())
        };

        let mut dsp = [0u8; 128];
        dsp.copy_from_slice(&bytes[OFF_DSP..OFF_DSP + 128]);
        let mut extra_ram = [0u8; 64];
        extra_ram.copy_from_slice(&bytes[OFF_EXTRA_RAM..OFF_EXTRA_RAM + 64]);

        Ok(Self {
            pc: u16::from_le_bytes([bytes[OFF_PC], bytes[OFF_PC + 1]]),
            a: bytes[OFF_A],
            x: bytes[OFF_X],
            y: bytes[OFF_Y],
            psw: bytes[OFF_PSW],
            sp: bytes[OFF_SP],
            ram: bytes[OFF_RAM..OFF_RAM + 0x1_0000].to_vec(),
            dsp,
            extra_ram,
            song_title,
            game_title,
        })
    }
}

/// ID666 text fields are fixed-width and NUL- or space-padded.
fn tag_string(raw: &[u8]) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).trim().to_string()
}

// ============================================================
// LOADING INTO THE APU
// ============================================================

fn load_spc(apu: &mut Apu, spc: &SpcFile) {
    // 0. Leave the HLE IPL. A fresh Apu sits in its boot state machine
    //    waiting for the main CPU's $CC upload handshake, and `step` runs
    //    that instead of the SPC700 until an execute command arrives. A
    //    .spc is a snapshot taken *after* the upload, so hand the core to
    //    the SPC700 directly. Done first so everything restored below
    //    overrides the boot-time side effects (SP, zero-page clear, the
    //    $AA/$BB announce on the output ports).
    apu.skip_ipl_boot();

    // 1. RAM. Copied raw: the I/O bytes at $F0–$FF land in the array too,
    //    but they're re-applied through the bus below so they take effect.
    apu.memory.ram.copy_from_slice(&spc.ram);
    // The main dump's $FFC0–$FFFF may hold the IPL ROM image if it was mapped
    // at snapshot time; the real RAM underneath is stored separately.
    apu.memory.ram[0xFFC0..].copy_from_slice(&spc.extra_ram);

    // 2. DSP registers, through the real $F2/$F3 path so write_reg's side
    //    effects (voice fields, FIR coefficients, echo setup…) all happen.
    for idx in 0..128u8 {
        if idx == DSP_KON || idx == DSP_ENDX {
            continue;
        }
        write_dsp(apu, idx, spc.dsp[idx as usize]);
    }
    // KON last, once pitch/SRCN/ADSR are in place. Voices that were sounding
    // at snapshot time restart from their sample start; the driver will
    // re-key the next notes itself anyway.
    write_dsp(apu, DSP_KON, spc.dsp[DSP_KON as usize]);

    // 3. I/O registers.
    let io = |addr: u16| spc.ram[addr as usize];

    // Timer targets before CONTROL, so timers start with the right divisor.
    apu.memory.write8(0x00FA, io(0x00FA));
    apu.memory.write8(0x00FB, io(0x00FB));
    apu.memory.write8(0x00FC, io(0x00FC));

    // CONTROL: keep timer enables (bits 0–2) and IPL ROM mapping (bit 7).
    // Bits 4–5 are one-shot "clear port" strobes, not state, so drop them.
    apu.memory.write8(0x00F1, io(0x00F1) & 0x87);

    // $F4–$F7 in the dump are what the SPC700 reads, i.e. the values the
    // main CPU last wrote. Restore them on the main-CPU side of the ports.
    for port in 0..4 {
        apu.memory.cpu_port_write(port, io(0x00F4 + port as u16));
    }

    // DSP address latch last, since restoring the DSP registers moved it.
    apu.memory.write8(0x00F2, io(0x00F2));

    // 4. SPC700 registers.
    set_cpu_registers(apu, spc);
}

fn write_dsp(apu: &mut Apu, idx: u8, val: u8) {
    apu.memory.write8(0x00F2, idx);
    apu.memory.write8(0x00F3, val);
}

/// Restores the SPC700 registers into `Spc700::regs`.
/// If PSW is stored as separate flags, use whatever "from byte" helper the
/// PLP / RETI implementation uses to unpack it.
fn set_cpu_registers(apu: &mut Apu, spc: &SpcFile) {
    apu.cpu.regs.pc = spc.pc;
    apu.cpu.regs.a = spc.a;
    apu.cpu.regs.x = spc.x;
    apu.cpu.regs.y = spc.y;
    apu.cpu.regs.sp = spc.sp;
    apu.cpu.regs.psw = spc.psw;
}

// ============================================================
// WAV OUTPUT
// ============================================================

/// Write 16-bit stereo interleaved samples as a standard PCM .wav file.
fn write_wav(path: &Path, samples: &[i16]) -> std::io::Result<()> {
    let channels: u16 = 2;
    let bits: u16 = 16;
    let block_align = channels * bits / 8;
    let byte_rate = SAMPLE_RATE * block_align as u32;
    let data_len = (samples.len() * 2) as u32;

    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    fs::write(path, out)
}

// ============================================================
// ENTRY POINT
// ============================================================

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <file.spc> [seconds] [out.wav]", args[0]);
        process::exit(1);
    }

    let in_path = Path::new(&args[1]);
    let seconds: u32 = match args.get(2) {
        Some(s) => s.parse().unwrap_or_else(|_| {
            eprintln!("seconds must be a whole number, got {s:?}");
            process::exit(1);
        }),
        None => 30,
    };
    let out_path = match args.get(3) {
        Some(p) => Path::new(p).to_path_buf(),
        None => in_path.with_extension("wav"),
    };

    let bytes = fs::read(in_path).unwrap_or_else(|e| {
        eprintln!("could not read {}: {e}", in_path.display());
        process::exit(1);
    });
    let spc = SpcFile::parse(&bytes).unwrap_or_else(|e| {
        eprintln!("{} is not a valid .spc: {e}", in_path.display());
        process::exit(1);
    });

    if !spc.game_title.is_empty() || !spc.song_title.is_empty() {
        println!("{} — {}", spc.game_title, spc.song_title);
    }
    println!(
        "PC={:#06X} A={:#04X} X={:#04X} Y={:#04X} SP={:#04X} PSW={:#04X}",
        spc.pc, spc.a, spc.x, spc.y, spc.sp, spc.psw
    );
    // The registers most relevant to the echo-clobbering suspect.
    println!(
        "DSP: FLG={:#04X} ESA={:#04X} EDL={:#04X} DIR={:#04X} | CONTROL={:#04X}",
        spc.dsp[0x6C], spc.dsp[0x6D], spc.dsp[0x7D], spc.dsp[0x5D], spc.ram[0xF1]
    );

    let mut apu = Apu::new();
    load_spc(&mut apu, &spc);

    // Render one second at a time so progress is visible on long runs.
    let mut samples: Vec<i16> = Vec::with_capacity((SAMPLE_RATE * seconds * 2) as usize);
    for sec in 0..seconds {
        // render_audio returns stereo frames ([L, R]); flatten them into the
        // interleaved L, R, L, R… layout the WAV writer expects.
        samples.extend(apu.render_audio(SAMPLE_RATE as usize).into_iter().flatten());
        eprint!("\rrendered {}/{seconds} s", sec + 1);
    }
    eprintln!();

    let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0);
    println!("peak amplitude: {peak} / 32768");

    if let Err(e) = write_wav(&out_path, &samples) {
        eprintln!("could not write {}: {e}", out_path.display());
        process::exit(1);
    }
    println!("wrote {}", out_path.display());
}
