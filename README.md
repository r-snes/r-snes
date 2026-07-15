# R-SNES

R-SNES aims to be a decently fast and very accurate SNES emulator, while staying easy to understand as per modern coding standards.

> [!NOTE]
> For now, this is still heavily a work in progress. We are barely able to run [a test ROM](https://github.com/gilyon/snes-tests) on our latest development branches, and are still quite far from being able to run actual games.
We're starting to work towards [plugins](#plugins), but this too is completely WIP.
>
> For the sake of brevity (and optimism!), many parts of this README will describe features of the emulator as if they were already functional, which is quite unlikely to be the case for now.

## Project purpose

Being a SNES emulator, R-SNES allows users to play SNES games. We also try to stay performant (low resource usage on the host machine) and accurate to our best effort (trying to make games behave the exact same as they would on the original console).

### Plugins

We also wish to make the player experience extensible with Lua plugins which can affect many parts of the emulator, be run automatically for each rendered frame, on each CPU cycle, or when a specific value in memory changes (among other trigger conditions).

Plugins will also be able to access things *outside* the emulator (which we refer to as "external"), which could allow them to, for example, read data from files, save data extracted from a running game to a file, or even send it over the network in HTTP requests.

Of course, plugins being able to do so much can cause risks since they might spread widely to non-developer communities (to people who can't easily audit what the plugin they are running is doing).
For that, we require plugins to describe what permissions they require, and prompt the user to confirm that they do intend to grant said permissions to the plugin before running it, so they can at least raise an eyebrow if some plugin requires complete unrestricted access to the internet if it doesn't seem like it needs it.

## Project documents

- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Contribution policy. External pull requests are currently closed while the core systems (CPU, APU, PPU...) are under active development as part of a university project, but issues and discussions are welcome. Also covers the rules that apply once contributions open: one crate per component, mandatory unit tests (80% coverage minimum per crate), `cargo fmt`, zero warnings, and doc comments on all public items.
- **[LEGAL.md](LEGAL.md)** - Legal and ethical framework for developing a SNES emulator under French/EU law: clean-room implementation, the private-copy exception, the interoperability exception for reverse engineering, and the project's policy of never distributing ROMs, BIOS, or firmware.
- **[FINANCE.md](FINANCE.md)** - Funding policy: the project is non-commercial and won't be sold, voluntary donations are allowed, and a future model for commissioned plugins (public-release or exclusive-release) is outlined.

## Installing

For now the only supported "install" method is building the emulator from source.

For this, you will need a local install of the Rust toolchain, including `cargo` and `rustc`. Building should be as simple as getting a local copy of the source code and then running `cargo build --release` from the project root.

You may need some system dependencies:

- SDL2
- Basic system-specific desktop libraries (for example, `libX11` and `libxkbcommon` on Linux)

For now, the project is known to work on Linux (Wayland) and Windows, but could also already work fine on other systems, give it a try!

## Project structure

Each component (hardware piece of the original console) is implemented in its own crate, in its own subfolder at the repository root (see the up-to-date list of workspace members in the root `Cargo.toml`). The main emulator program (front-end, window/GUI, and top-level glue between crates) lives directly in `src/`.

- **`cpu/`** - Emulation of the 65816 CPU core, including the full instruction set. Instructions are defined through a small custom DSL, compiled via the `instr_metalang_procmacro` proc-macro sub-crate.
- **`ppu/`** - Emulation of the Picture Processing Unit: VRAM/CGRAM, PPU registers, and the frame rendering pipeline (one module per background/rendering mode).
- **`apu/`** - Emulation of the Audio Processing Unit: the S-SMP CPU core and the S-DSP sound chip (ADSR envelopes, BRR sample decoding, voices, timers).
- **`bus/`** - The system bus tying the components together: address space, WRAM, memory-mapped I/O, and ROM loading/header parsing.
- **`common/`** - Small shared types and utilities used across the other crates (e.g. SNES address types, bit-splitting helpers).
- **`plugins/`** - The Lua plugin system described [above](#plugins): the plugin runtime and the permission tree that gates what a plugin is allowed to do, with two dedicated derive-macro sub-crates.
- **`product_order/`** - Derive-macro crate that provides comparison/ordering logic used by the permission tree in `plugins/`, so permission nodes can be compared against each other.

At the repository root you'll also find the workspace's `Cargo.toml`/`Cargo.lock`, project documentation (`README.md`, `CONTRIBUTING.md`, `TECHNOLOGY_EVALUATION.md`, `LEGAL.md`, `FINANCE.md`), test ROMs used for validation (e.g. `cputest-*.sfc`, `spctest.sfc`), a Nix flake for a reproducible dev environment, and the project's static website under `site/`.

```
R-SNES/
├── src/                  # main binary: front-end, GUI, wiring between crates
│   └── rsnes/
├── cpu/                  # 65816 CPU core + instruction set
│   ├── instr_metalang_procmacro/
│   ├── docs/
│   │   └── README.md
│   └── README.md
├── ppu/                  # Picture Processing Unit (rendering, VRAM, CGRAM)
│   └── rendering/
├── apu/                  # Audio Processing Unit (S-SMP + S-DSP)
│   └── dsp/
├── bus/                  # system bus, WRAM, ROM loading/header
│   └── rom/
├── common/               # shared types & utilities
├── plugins/              # Lua plugin runtime + permission system
│   ├── perm_tree_node_derive/
│   └── permission_derive_macro/
├── product_order/        # permission-tree comparison/ordering derive macro
├── site/                 # project website
├── Cargo.toml            # workspace definition
└── README.md
```

## Project architecture

Architecturally, the root `r-snes` crate (`src/`) acts as the orchestrator: it owns and steps every hardware component crate (`cpu`, `ppu`, `apu`) forward cycle by cycle, and collects their output (the rendered frame, the audio samples) to hand off to the front-end.

The `cpu` and `ppu` crates don't talk to memory directly - they go through the `bus` crate, which owns the SNES memory map and knows how to route a given address to RAM, ROM, or a memory-mapped IO device. This keeps each component crate focused purely on its own emulation logic, while `bus` is the single place that knows *where things live in memory*.

The `apu` crate is a bit different: it has its own internal CPU (the S-SMP) and its own separate memory zone, so it doesn't go through the shared `bus` crate at all - it manages its reads and writes internally. The main program only steps it forward cycle by cycle according to its own clock speed (which differs from the main CPU's), the same way it does for the CPU and PPU, and then collects its audio output.

```
                              +--------------------+    
                              | r-snes crate: root |          | Key:    
                              |--------------------|   2      |  1. Steps component cycles
      +-----------------------|    main program:   |<------+  |  2. Fills output (image/audio)   
      |                       |  orchestrates all  |----+  |  |  3. Read/Write requests
      |  +------------------->|     components     |    |  |  |  4. Perform read/write
      |  |                    +--------------------+    |  |  |  5. Return CPU reads
      |  |2                     ^  |  |      |  ^      1|  |
     1|  |                     3|  |1 |5     |  |       v  |
      v  |                      |  v  v      |  |     +------------------------+
+-----------------+  +-------------------+   |  |     |    ppu crate: PPU      |
| apu crate: APU  |  |  cpu crate: CPU   |   |  |     |------------------------|
|-----------------|  |-------------------|   |  |     |  graphics processing.  |
| audio rendering |  | handles execution |   |  |     | writes the framebuffer |
+-----------------+  |    of the main    |   |  |     |    according to        |
         ^           |     program       |   |  |5    |    write requests      |
         |           +-------------------+  3|  |     +------------------------+
         |                                   |  |               ^
         |               +------------------------------+       |
         |               |         bus crate |  |       |       |
         |               |-------------------|--|-------|       |
         |               |                   v  |       |       |
         |       4       |          +----------------+  |      4|
         +---------------|--------->|      Bus       |<-|-------+
                         |          |----------------|  | 
                         |          | defines the    |  | 
                         |  +-------| memory mapping |  | 
                         |  |       +----------------+  | 
                         |  |4      4|        4|        |
                         |  v        v         v        |
                         | RAM      ROM     IO devices  |
                         +------------------------------+
```

## Language and technology choices

The emulator is implemented in Rust. This choice of language is mostly by personal preference, but our preferences are also influenced by having worked with C and C++ for a few years: we all came to agree that it is easier to collaborate with Rust (even though we had far less experience with it at the start of the project) than with other programming languages that can compete in performance and low-level control, such as C and C++.

Beyond the language itself, the project's front-end relies on SDL2 for windowing, with egui layered on top for easier UI handling.

The full reasoning behind these decisions, methodology, comparison tables, and what would make us reconsider each one, is documented in [`TECHNOLOGY_EVALUATION.md`](TECHNOLOGY_EVALUATION.md).
