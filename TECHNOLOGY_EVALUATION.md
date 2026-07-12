# R-SNES - Technology Evaluation

This document records the comparative analyses behind the main technology choices of R-SNES: the implementation language, the graphics/windowing library, and the audio backend. It is intended both as a justification of past decisions and as a reference for revisiting them if the project's constraints change.

## 1. Methodology

Each candidate technology was evaluated against the needs of a cycle-accurate SNES emulator:

- **Correctness & safety** - an emulator manipulates raw memory layouts, bit-level registers and tightly coupled subsystems; classes of bugs eliminated by the language or library directly reduce debugging time.
- **Performance** - the emulator must sustain 60 FPS with cycle-level emulation of the CPU, PPU and APU on ordinary student hardware.
- **Ecosystem & bindings maturity** - libraries must have stable, maintained Rust bindings and good documentation.
- **Cross-platform support** - Windows and Linux are both used within the team.
- **Team fit** - learning value and motivation are explicit goals of this end-of-studies project.

Where quantitative measurements were made, the protocol and results are given in the project's benchmarking notes. Everything else is a qualitative assessment based on documentation, community feedback (NesDev forums) and prototyping.

## 2. Implementation language: C++ vs Rust

| Criterion | C++ | Rust |
|---|---|---|
| Memory safety | Manual; UB-prone | Guaranteed by the borrow checker (safe subset) |
| Performance | Native, zero-cost abstractions | Native, zero-cost abstractions |
| Tooling | Fragmented (CMake, vcpkg, Conan…) | Unified (cargo: build, test, doc, deps) |
| Modularity | Headers / modules, convention-based | First-class crates and workspaces |
| Emulation ecosystem | Very large (most existing emulators) | Growing; several serious emulators exist |
| Team experience | Prior coursework experience | Limited - deliberate learning goal |

### Analysis

Both languages meet the performance requirement: SNES emulation is not compute-bound on modern hardware, and neither language imposes runtime overhead (no GC, no VM). The decision therefore rested on everything *around* raw speed.

An emulator is an unusually good match for Rust's guarantees. The codebase is dominated by shared mutable state (the bus, memory-mapped registers, DMA transfers touching several subsystems at once), which is exactly where C++ codebases accumulate use-after-free and aliasing bugs. The borrow checker forces those interactions to be designed explicitly up front; a real constraint during development (notably around `&mut` access to shared register state), but one that converts entire categories of runtime bugs into compile-time errors.

Cargo was a second decisive factor for a four-person team: a single reproducible build command on all operating systems used by the team, integrated unit testing used throughout the project, and workspace support that maps directly onto our one-crate-per-subsystem architecture (CPU, PPU, APU, memory/cartridge).

Finally, the team explicitly wanted the project to be a vehicle for learning a modern systems language. C++ would have leaned on existing knowledge; Rust extended it.

### Decision

**Rust.** Retained for its safety guarantees on heavily shared state, its unified tooling, and its learning value. The main accepted cost is the borrow checker's learning curve and the smaller pool of emulator-specific reference code compared to C++.

## 3. Graphics / windowing library: minifb vs pixels vs egui/eframe vs SDL2 vs SDL3

| Criterion | minifb | pixels | egui / eframe | SDL2 | SDL3 |
|---|---|---|---|---|---|
| Scope | Framebuffer + basic input only | GPU-backed pixel-buffer blitting (via wgpu); no input/audio | Immediate-mode GUI framework; `eframe` adds windowing, but it's GUI-first, not emulator-frame-first | Window, rendering, input, audio, controllers | Same as SDL2, modernized API |
| Rust bindings | Native crate, minimal | Native crate, actively maintained | Native crate, very active | `sdl2` crate - mature, widely used | Immature at decision time |
| Stability | Stable but limited | Stable | Stable | Very stable (10+ years) | Recently released, API still settling |
| Controller support | No | No (needs pairing with `winit` for input, no native controller support) | No native game-controller support | Yes (essential for an emulator) | Yes |
| Documentation & community | Small | Good, smaller community than SDL2 | Extensive, but focused on GUI/tooling use cases rather than emulator front-ends | Extensive | Growing |

### Analysis

**minifb** was attractive for early prototyping (an early version of our emulator used it) - it opens a window and blits a framebuffer with almost no ceremony, which matched the PPU's output model (the emulator produces a complete frame; no GPU pipeline is needed). It was rejected for the final product because it stops there: no game-controller input, no audio, no wider multimedia support. Adopting it would have meant assembling and synchronizing several independent libraries ourselves.

**pixels** was also benchmarked: it gives GPU-accelerated framebuffer presentation through wgpu with a very small API surface, which is appealing for a "produce one frame, blit it" workflow like ours. It was set aside for the same structural reason as minifb - it only solves presentation, so we would still need to bring our own windowing/input crate (typically `winit`) and our own audio and controller-handling stack, reassembling by hand what SDL2 already provides as one coherent package.

**egui / eframe** was evaluated as well, since we already use egui for debug tooling. `eframe` can host a window and a render loop, but it is designed around immediate-mode GUI rendering rather than raw pixel-buffer presentation, and it has no built-in game-controller support. It remains a strong fit for our debug/introspection UI, but it isn't a substitute for the emulator's core presentation layer.

**SDL3** was evaluated as the forward-looking option. It was rejected for now on two grounds: the library itself was released too recently to have accumulated the field-testing SDL2 has. And, more importantly for us, its Rust bindings were not yet stable at the time of the decision (confirmed again when checking for a newer release). Building the project's presentation layer on unstable bindings was judged an unnecessary risk with no functional benefit for our needs.

**SDL2** offers everything the emulator's front-end requires in one coherent, battle-tested package: windowing, 2D texture presentation for the PPU framebuffer, keyboard and game-controller input, and audio output. The `sdl2` Rust crate is mature and widely deployed. SDL2's model also fits an emulator naturally: we upload one completed frame per emulated VBlank, which is precisely the workflow it was designed around.

### Decision

**SDL2**, for its stability, its complete feature coverage and its mature Rust bindings. minifb, pixels, and egui/eframe were rejected as too narrow in scope (presentation- or GUI-only, missing input/audio/controller support); SDL3 as too recent, with bindings not yet production-ready. This decision should be **revisited** once the SDL3 Rust ecosystem stabilizes; the migration path from SDL2 is well documented, and the presentation layer is isolated in its own module to keep that migration cheap.

## 4. Audio backend: cpal vs rodio vs adapted fork (evaluation in progress)

### Context

The SNES APU (S-SMP + S-DSP) produces a 16-bit stereo stream at 32 kHz. The audio backend's job is narrow but demanding: deliver that stream to the host's audio device with low latency, precise buffer control, and resampling to the device's native rate, while staying synchronized with the emulation clock. Audio is the subsystem where synchronization problems are the most audible: buffer underruns produce immediately noticeable crackling.

| Criterion | cpal | rodio | Adapted fork |
|---|---|---|---|
| Level of abstraction | Low - raw device streams | High - playback, mixing, decoding (built on cpal) | Tailored to our needs |
| Buffer/latency control | Full | Limited (managed internally) | Full |
| Resampling | Manual (we implement it) | Built-in | Depends on base |
| Maintenance burden | Low | Low | High - we own it |
| Fit for continuous synthesized streams | Good | Designed more for sample/file playback | Good by construction |

### Analysis (ongoing)

rodio's convenience features (decoding, mixing, sinks) solve problems an emulator does not have; we generate exactly one continuous stream, while its internal buffer management takes away the control we need most. cpal exposes the device callback directly, which lets us tie buffer fill level to emulation pacing, at the cost of implementing resampling and the ring buffer ourselves. A fork would only be justified if a specific blocking limitation is found in cpal.

### Status

**Evaluation in progress.** Current working hypothesis: **cpal**, with a small internal ring-buffer and resampling layer, because latency and synchronization control outweigh rodio's convenience for this use case. Final decision pending prototype measurements.

## 5. Decision summary

| Domain | Decision | Status | Revisit condition |
|---|---|---|---|
| Language | Rust | Final |  |
| Graphics/windowing | SDL2 | Final | SDL3 Rust bindings reach stability |
| Audio | cpal (working hypothesis) | In progress | Prototype latency measurements |
