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

## 4. Audio backend: cpal vs rodio vs SDL2 audio queue vs adapted fork

### Context

The SNES APU (S-SMP + S-DSP) produces a 16-bit stereo stream at a fixed 32 kHz. The backend's job is narrow but demanding: deliver that stream to the host audio device with low latency and precise buffer control, while staying synchronized with the emulation clock - underruns produce immediately audible crackling, over-buffering produces lag behind the video. One structural property matters throughout: the emulator *pushes* ready samples once per frame (the APU buffers its DSP output internally and exposes a single drain call), so a backend that pulls samples from an audio thread adds a synchronization problem we don't otherwise have.

| Criterion | cpal | rodio | SDL2 audio queue | Adapted fork |
|---|---|---|---|---|
| Delivery model | Pull (device callback, audio thread) | Pull, hidden behind `Sink`/`Source` | Push, from the emulation thread | Depends on base |
| Buffer/latency control | Full | Limited (managed internally) | Direct (queued size is readable) | Full |
| Resampling from 32 kHz | Manual (we implement it) | Built-in | Built-in (device conversion) | Depends on base |
| New dependency | Yes | Yes | No - SDL2 already adopted (§3) | Yes, and we own it |
| Maintenance burden | Low, but ring buffer + underrun policy are ours | Low | Low | High |

### Analysis

**rodio** solves problems an emulator does not have - decoding, mixing, sinks - while its internal buffer management takes away the control we need most: latency is not precisely tunable, and feeding one continuous synthesized stream means wrapping our output in a custom `Source` only to lose visibility into how much of it is buffered.

**cpal** was the initial working hypothesis and is the strongest standalone candidate: the device callback allows tying buffer fill level directly to emulation pacing. Its cost is the pull model itself - a producer/consumer split across threads, so the lock-free ring buffer, underrun policy and 32 kHz resampling are all code we write and maintain, to obtain a capability that turned out to already be in our dependency tree.

**SDL2's audio queue** became the natural fit once the front-end decision (§3) landed on SDL2, whose scope already includes audio. Its push model matches the emulator's structure exactly: each frame, the emulation loop queues the drained samples and moves on; SDL converts from 32 kHz to the device rate, and the queued-size query provides the same pacing signal cpal's callback would have - synchronization becomes a single-threaded comparison instead of a cross-thread protocol. The only part we own is a small bounded buffer inside the APU, drained once per frame and capped so unconsumed audio is discarded rather than accumulated.

**An adapted fork** was kept only as a contingency: owning platform audio backends (WASAPI, ALSA, CoreAudio) is exactly the burden these libraries exist to absorb, and no blocking limitation justifying it was found.

### Decision

**SDL2's audio queue, fed by a small buffering layer in the APU.** Retained because it matches the emulator's push-based output, gives direct latency control, and adds no dependency or audio-thread synchronization. cpal was rejected as redundant once SDL2 was in the stack; rodio for insufficient buffer control; a fork for its maintenance cost. The accepted trade-off is coupling audio to SDL2: an eventual SDL3 migration (§3) takes the audio path with it, mitigated by the same isolation - the core only exposes `drain_samples()`, and everything SDL-specific lives in the front-end.

## 5. Decision summary

| Domain | Decision | Status | Revisit condition |
|---|---|---|---|
| Language | Rust | Final |  |
| Graphics/windowing | SDL2 | Final | SDL3 Rust bindings reach stability |
| Audio | SDL2 audio queue + in-APU buffering | Final | Measured latency/underrun problems; SDL3 migration (moves with §3) |