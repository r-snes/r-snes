Nice project! The SNES APU is a fascinating but tricky beast. It’s essentially its own little computer (Sony SPC700 CPU + DSP + 64KB RAM), running in parallel with the main 65816 CPU. Since you’re tasked with making a **standalone APU emulator** first, here’s a structured roadmap that balances correctness, modularity, and testability:

---

## 📍 Roadmap for Developing the SNES APU in Rust

### **Phase 1: Foundations**

1. **Research & Specs**

   * Read the **SPC700 CPU instruction set** and cycle timings.
   * Understand the **DSP registers** (envelope, pitch modulation, echo, FIR filter).
   * Gather test ROMs for APU (e.g., Blargg’s APU test suite).
   * Document memory map (64KB APU RAM, I/O registers, DSP registers).

2. **Project Setup**

   * Create a dedicated Rust crate for the APU.
   * Define a clean API boundary (e.g., `step(cycles)`, `read/write registers`, `render_audio(samples)`).
   * Add logging/tracing for debugging since timing bugs are common.

---

### **Phase 2: SPC700 CPU Emulation**

1. **CPU Core**

   * Implement registers and addressing modes.
   * Implement the full instruction set (opcode decoding).
   * Handle cycle-accurate timing (important for sync).
   * Add interrupts (Timer 0/1/2).
2. **Memory & I/O**

   * Implement APU RAM (64KB).
   * Map I/O ports (communication with SNES CPU).
   * Timer registers (increment at fixed rates).

✅ Deliverable: Run small SPC700 programs in isolation.

---

### **Phase 3: DSP Core**

1. **DSP Architecture**

   * Implement DSP registers (pitch, volume, ADSR, echo, FIR).
   * Support BRR sample decoding (SNES compressed sample format).
   * Implement voice mixing (8 voices).

2. **Timing**

   * Ensure DSP runs at the correct clock relative to SPC700.
   * Handle sample rate generation (32 kHz output).

3. **Audio Buffer**

   * Mix samples into a buffer for the host system.
   * Provide an interface to output raw PCM.

✅ Deliverable: Play a single BRR sample correctly.

---

### **Phase 4: Integration**

1. **APU Subsystems**

   * Wire SPC700 ↔ DSP (shared memory, registers).
   * Implement timers properly.
   * Synchronize execution (SPC700, DSP, timers).
2. **SNES CPU Communication**

   * Support I/O ports for sending/receiving data between 65816 CPU and APU.
   * Implement boot sequence (SNES CPU uploads sound driver code to APU RAM).

✅ Deliverable: Run real SNES sound drivers (like Nintendo’s boot ROM).

---

### **Phase 5: Testing & Debugging**

1. **Verification**

   * Run Blargg’s APU test ROMs.
   * Compare against known SPC dumps (SPC files).
   * Use regression testing (SPC → PCM waveform).

2. **Debug Tools**

   * Add disassembler for SPC700 instructions.
   * Add DSP register viewer / logging.
   * Optionally export `.wav` files for debugging.

---

### **Phase 6: Optimization & Refinement**

1. **Performance**

   * Optimize DSP mixing (SIMD if needed).
   * Consider cycle skipping when idle.

2. **Accuracy**

   * Improve timing accuracy (sub-instruction precision).
   * Match quirks (known hardware bugs, envelope edge cases).

3. **Integration Ready**

   * Ensure the APU can be dropped into the full SNES emulator.
   * Expose a stable interface for stepping and audio output.

✅ Final Deliverable: A standalone Rust APU library that can:

* Run SPC700 programs
* Emulate DSP accurately
* Output PCM audio
* Be integrated into the SNES emulator core

---

⚡ **Tips for development**

* Start **CPU first**, DSP second, then connect.
* Use SPC files (SNES music dumps) as a test harness.
* Don’t optimize too early — get correctness first.
* Keep the interface modular so the APU can run standalone or inside the SNES core.

---

Do you want me to also make you a **detailed timeline with milestones** (like Week 1 = CPU core, Week 2 = DSP basics, etc.) so you can follow it step by step, or do you prefer just this higher-level roadmap?
