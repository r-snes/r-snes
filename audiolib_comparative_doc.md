# RSNES: Audio Output Library Comparison

## Overview

RSNES implements its own APU emulation (SPC700 + S-DSP) from scratch as part of the
core codebase. This document focuses exclusively on the **audio output layer** —
the library responsible for receiving the raw PCM stream produced by the APU and
sending it to the host OS in real time.

The primary criterion for evaluation is **accuracy to the original SNES hardware**,
meaning the output layer must be capable of faithfully reproducing the PCM stream
without introducing timing drift, latency artifacts, or resampling distortion.

---

## Candidates

---

### cpal
**Repository:** `github.com/RustAudio/cpal`
**Language:** Pure Rust
**License:** Apache 2.0

A low-level, cross-platform audio I/O library. It exposes direct control over audio
streams, buffer sizes, and device selection. Supports multiple backends depending on
the platform: ASIO and WASAPI on Windows, CoreAudio on macOS, and ALSA, JACK, or
PipeWire on Linux. Also supports WebAssembly via the Web Audio API.

**Strengths:**
- Low-level API gives full control over buffer size and stream timing, critical for
  keeping audio output in sync with the emulated APU
- Cross-platform with optional low-latency backends (ASIO on Windows, JACK on Linux)
- Pure Rust, straightforward `cargo add` integration
- Actively maintained
- WebAssembly-compatible for potential browser builds

**Weaknesses:**
- Requires additional platform libraries on Linux (ALSA dev packages)
- Low-level nature means more integration work upfront
- Cross-compiling can be tricky due to native library dependencies

---

### rodio
**Repository:** `github.com/RustAudio/rodio`
**Language:** Pure Rust (built on cpal)

A higher-level audio playback library built on top of cpal. Adds format decoding,
source mixing, and stream management abstractions on top of cpal's raw stream access.

**Strengths:**
- Simple, ergonomic API — faster to get something running
- Useful if audio file playback is ever needed alongside APU output

**Weaknesses:**
- The abstraction layer removes the fine-grained timing control that accurate APU
  output requires
- Designed primarily for file-based playback, not raw PCM streaming from an emulated
  DSP
- Adds unnecessary overhead and dependencies for RSNES's use case

---

### Custom Implementation
**Language:** Rust + platform audio APIs directly

Rather than depending on a third-party output library, RSNES could interface directly
with platform audio APIs (WASAPI on Windows, CoreAudio on macOS, ALSA on Linux).
This is the approach taken by several high-accuracy emulators when precise control
over audio timing is critical.

**Strengths:**
- Maximum control over buffer management, timing, and synchronisation with the APU
- No dependency on a third-party crate that could change or be abandoned
- Can be tailored exactly to RSNES's internal audio pipeline

**Weaknesses:**
- Significantly more development effort — each platform requires its own
  implementation
- Maintenance burden falls entirely on the RSNES team
- Platform audio APIs can be complex and poorly documented (WASAPI in particular)
- Likely overkill if cpal's level of control proves sufficient

---

## Summary

| Option      | Control Level | Cross-platform | Integration Effort | Maintenance     | Recommended |
|-------------|---------------|----------------|--------------------|-----------------|-------------|
| cpal        | High          | ✅ Yes         | Low–Medium         | External (active) | ✅ Yes    |
| rodio       | Low           | ✅ Yes         | Low                | External (active) | ❌ No     |
| Custom      | Maximum       | ⚠️ Manual     | Very High          | Internal        | ⚠️ If needed |

## Recommendation

**cpal** is the recommended starting point for RSNES. Its low-level API provides
sufficient control over buffer sizes and stream timing to keep audio output accurately
synchronised with the APU, while avoiding the significant development overhead of a
custom implementation. A custom solution should only be considered if cpal proves
unable to meet RSNES's timing or accuracy requirements during integration testing.
