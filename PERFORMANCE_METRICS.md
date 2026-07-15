# R-SNES - Performance Metrics

This document describes the performance indicators tracked for R-SNES - CPU sleep time and memory usage - the methodology used to measure them, and the results obtained.

## 1. Test protocol

- **Test ROM:** [snes-tests](https://github.com/gilyon/snes-tests/releases) - the basic SNES test ROM.
- **Platform:** native Windows machine.
- **Procedure:** launch the test ROM, let it run for 10 seconds, then read the accumulated sleep time.
- **Repetitions:** each stage's result is the average sleep time over 10 repetitions of the procedure.

## 2. Metric 1 - CPU sleep time

### Definition

The emulator's core loop advances emulation in fixed increments (`MASTER_CYCLE_DURATION`), derived from the SNES master clock frequency (`MASTER_CLOCK_HZ = 21,477,300 Hz`, i.e. one master cycle every ~46.6 nanoseconds). Whenever the loop is running ahead of real time, it sleeps for the remaining duration before the next master cycle is due; whenever it is not ahead, it processes master cycles immediately with no sleep.

The **sleep time** is the cumulative time spent in these sleep calls over the run, expressed as a percentage of total elapsed run time:

```
sleep_time_pct = (sum of all sleep durations) / (total run duration) × 100
```

### Why this metric

Because R-SNES already sustains a stable 60 FPS output, frame rate alone does not reveal how much CPU headroom the emulator has - a loop barely keeping up with the frame deadline and a loop finishing far ahead of it can both report 60 FPS. Sleep time exposes that difference directly: a higher sleep-time percentage means the emulator spends less real time doing emulation work to sustain the same output, i.e. it is more CPU-efficient at a fixed frame rate.

### Instrumentation

The sleep time is accumulated directly in the emulation loop (`main.rs`), by timing the existing `std::thread::sleep` call already used to pace master-cycle execution, and summing its duration across the run.

## 3. Results

| Stage | Description | Sleep time |
|---|---|---|
| 1 | Baseline - no optimization; master cycles processed without any timing/pacing logic | 0% |
| 2 | Added timers and accumulators to pace master-cycle execution against real time | ~70% |
| 3 | Adjusted master-cycle processing to run in batches rather than one at a time | ~80% |

### Analysis

**Stage 1 (baseline).** With no timing or pacing mechanism, the loop processed master cycles continuously with no sleep calls at all, so the CPU was active 100% of the time regardless of whether the emulation was actually ahead of real time - hence 0% sleep time. Output was already stable at 60 FPS at this stage, which is precisely why sleep time - rather than frame rate - was needed to reveal the difference: FPS alone looked identical across all three stages despite very different CPU costs to sustain it.

**Stage 2 (timers and accumulators).** Introducing `frame_accum` / `master_cycle_accum` and comparing elapsed real time against `MASTER_CYCLE_DURATION` allowed the loop to detect when it was running ahead of schedule and sleep for the difference instead of busy-looping. This alone raised sleep time from 0% to approximately 70%, meaning the emulator now spends roughly a third of run time on actual work and two-thirds idle, for the same 60 FPS output as the baseline.

**Stage 3 (batch processing of master cycles).** Rather than consuming one master cycle and re-checking the clock each time, master cycles due before the next real-time check are processed together in a batch within the `while master_cycle_accum >= MASTER_CYCLE_DURATION` loop. This reduces the per-cycle overhead of the pacing logic itself (fewer `Instant::now()` calls and loop-condition checks relative to work done), which further increased sleep time to approximately 80%.

## 4. Metric 2 - Memory usage

### Definition

Peak memory usage (peak working set) of the R-SNES process during a run, measured on native Windows, compared against other SNES emulators under the same conditions.

### Why this metric

Sleep time and memory usage cover two different resource axes - CPU time and memory footprint - giving a broader picture of R-SNES's efficiency than either alone. Comparing against established emulators also situates R-SNES's memory usage relative to known references rather than in isolation.

### Results

| Emulator | Peak memory usage |
|---|---|
| snes9x | ~30 MB |
| R-SNES | ~75 MB |
| ZSNES | ~75 MB (comparable) |
| Mesen | ~218 MB |
| bsnes | ~350 MB |

### Analysis

R-SNES's memory usage is comparable to ZSNES and substantially lower than Mesen and bsnes. Note that bsnes is widely regarded as the reference cycle-accurate SNES emulator, prioritizing accuracy over lightweight footprint, which likely explains its higher memory usage relative to less strictly accurate emulators. However, snes9x sits well below R-SNES at ~30 MB, showing there is still room for improvement - this is an area we intend to keep optimizing going forward.
