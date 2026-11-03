# Architecture

R-SNES is a Cargo workspace. Each hardware component of the console is implemented in its own crate, and the root crate (`src/`) ties them together into the emulator program.

## The crates

| Crate | Folder | Role | API docs |
|:------|:-------|:-----|:---------|
| `r-snes` | `src/` | Front-end (window, GUI, audio output) and the main loop driving every component | [r_snes](../api/r_snes/index.html) |
| `cpu` | `cpu/` | The 65816 CPU core. Instructions are written in a small DSL compiled by the `instr_metalang_procmacro` sub-crate | [cpu](../api/cpu/index.html) |
| `ppu` | `ppu/` | Picture Processing Unit: VRAM, CGRAM, PPU registers and the rendering pipeline (one module per background mode) | [ppu](../api/ppu/index.html) |
| `apu` | `apu/` | Audio Processing Unit: the S-SMP CPU and the S-DSP sound chip (envelopes, BRR samples, voices, timers) | [apu](../api/apu/index.html) |
| `bus` | `bus/` | The system bus: address space, WRAM, memory-mapped I/O, ROM loading and header parsing | [bus](../api/bus/index.html) |
| `common` | `common/` | Small shared types and helpers, such as SNES addresses | [common](../api/common/index.html) |
| `plugins` | `plugins/` | The Lua plugin runtime and the permission tree, with two derive-macro sub-crates | [plugins](../api/plugins/index.html) |
| `product_order` | `product_order/` | Ordering logic used to compare permission nodes | [product_order](../api/product_order/index.html) |

The CPU crate also has an in-depth [design document](../api/cpu/docs/index.html) explaining its public API, its `cycle` function and the instruction meta-language.

## How they fit together

The root crate acts as the orchestrator: it owns every component and steps the CPU, PPU and APU forward cycle by cycle, each at its own clock speed. It then collects their output (the rendered frame and the audio samples) and hands it to the front-end.

The CPU and PPU don't access memory directly: they go through the `bus` crate, which owns the SNES memory map and knows how to route an address to RAM, ROM or a memory-mapped I/O register. This keeps each component focused on its own logic, while `bus` is the single place that knows where things live in memory.

The APU is a bit different: it has its own CPU (the S-SMP) and its own memory, so it doesn't use the shared bus at all. The main loop only steps it at its own clock speed and collects its audio output.

```text
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

Plugins hook into this loop: the root crate gives them access to the components they have permission for, and calls their autoactions at the right moments (before each instruction, at intervals…).

## Where to go next

- The [contribution policy](https://github.com/r-snes/r-snes/blob/main/CONTRIBUTING.md) lists the rules every crate follows: unit tests keeping the workspace above 80% coverage, `cargo fmt`, no warnings, and doc comments on every public item.
- The [coverage dashboard](../coverage/index.html) shows how well each crate is tested.
