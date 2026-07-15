# RSNES: Roadmap PGE5

## Overview

With a team of 4 and a mostly complete core, all five workstreams kick off in
parallel on day one and complete on a staggered schedule. Game compatibility and
the plug-in system form the protected core of the project; the other workstreams
act as adjustment variables if delays occur. Integration and polish run
continuously from kickoff to final delivery.

---

## Features

### Feature 1: Plug-ins API Enrichment

The plugin system is already functional: plugins are loaded dynamically through a
sandboxed VM acting as the communication layer with the emulator, without modifying
the core codebase. Capabilities exposed to plugins are, for now, intentionally
limited.

The next step is to enrich the API along two axes:

#### New Triggers (Launch Conditions):
Allow plugins to react to emulator events rather than running continuously, giving
precise control over when and how they interact with a ROM. Known candidate
conditions include:

- **On a timer**: trigger the plugin at a fixed interval or after a set delay
- **On key press**: activate when the player presses a specific input
- **On a specific in-game event**: fire when a particular state or condition is
  detected in the ROM (e.g. a memory value changing, a scene loading)

> The full list of supported triggers is to be determined during implementation.

#### Network Access:
Introduce network capabilities for plugins, gated behind a dedicated permission,
consistent with the rest of the permission system. The exact scope and features of
network support are still to be determined.

### Feature 2: Save States

Save and restore the complete console state instantly, independently of the game's
own save system. A save state captures the full machine state: CPU registers, RAM
contents, PPU state, APU state, and cartridge SRAM. Restoring a state replays that
exact moment.

- Serialize and deserialize each subsystem (CPU, PPU, APU, memory) to/from a file
  reloadable identically
- Support at least one save slot per game
- Stretch goal: multiple named slots with metadata (timestamp, screenshot thumbnail)

### Feature 3: Game Compatibility

Ensure correct operation of as many official SNES titles as possible. This
milestone serves as a practical, end-to-end integration test across the CPU, PPU,
APU, and cartridge emulation layers.

The approach is iterative: run games on the emulator, identify incorrect
behaviours, fix the corresponding inaccuracies, and retest. The list of sensitive
games provided by the NesDev community serves as the starting point.

- Boot and reach gameplay in at least one commercial title
- Identify and document any remaining compatibility gaps

### Feature 4: Embedded Cartridge Components

Some cartridges embed additional compute chips; without emulating them, those games
are unplayable. We plan to integrate 2 to 3 of these components, starting with the
most widespread.

#### Memory Maps (prerequisite)
- **LoROM (Mode 20)**: the most common memory map; ROM data mapped in 32 KB banks
  across the lower address space. Used by most early and mid-generation titles.
- **HiROM (Mode 21)**: full 64 KB ROM banks mapped into the upper address space,
  allowing larger cartridge capacities without bank-switching overhead. Used by
  many later and larger titles.

#### Co-processors (deliverable)
- **SA-1**: a secondary 65C816 CPU embedded in certain cartridges (e.g. Super
  Mario RPG, Kirby Super Star). It runs in parallel with the main CPU and handles
  fast math, DMA transfers, and bitmap operations. Emulating it requires a separate
  execution context and memory arbitration logic.
- **DSP series** and other chips: candidates to be selected by prevalence in the
  game library.

> **Goal:** implement at least 2–3 co-processors and pass the majority of tests in
> the SNES test ROM suite.

### Feature 5: Final Polish

Finishing phase, running continuously throughout the project and becoming the main
focus once the other features land:

- Performance optimization (targeted profiling)
- Fixing the last known bugs
- Interface adjustments based on feedback

The objective is to deliver an emulator that is stable and pleasant to use, not
merely functional.

---

## Timeline (31 August 2026 – mid-February 2027)

Starts are staggered by priority and by dependency: save states and embedded
components only make sense once commercial games actually run, so they begin
when game compatibility reaches its first milestone rather than at kickoff.
 
| # | Feature                   | Start              | Target End           | Effort profile |
|---|---------------------------|--------------------|----------------------|----------------|
| 3 | Game Compatibility        | 31 August 2026     | end of Septembre 2026 | main team focus from day one |
| 1 | Plug-ins (API enrichment) | 31 August 2026     | early February 2027  | continuous background effort |
| 2 | Save States               | 1 Septembre 2026  | End of Decembre 2027     | starts once games are running |
| 4 | Embedded Components       | 1 Septembre 2026  | early February 2027  | starts once games are running |
| 5 | Final Polish              | continuous         | 15 February 2027     | primary focus from mid-January |

### Key Dates

- **31 August 2026**: PGE5 kickoff — all workstreams begin
- **Mid-October 2026**: Plugin API enrichment complete
- **Early November 2026**: Save states complete
- **End of December 2026**: Game compatibility target reached
- **Early February 2027**: Embedded components complete
- **15 February 2027**: Final delivery

---

## In Case of Delays or Unforeseen Issues

### Detection

- **Continuous tracking of GitHub milestones**: every feature is broken down into
  issues attached to a milestone with a due date, making progress visible at a
  glance.
- **A slipping milestone = an immediate alert**: a milestone counts as slipping as
  soon as its remaining issues can no longer realistically be closed by its due
  date at the current pace, not once the date has passed.

### Graduated Response

Applied in order of increasing cost; escalate only if the previous level is
insufficient:

1. **Absorb**: minor slippage is handled within the feature's own schedule by
   re-ordering its remaining issues.
2. **Reassignment**: a second team member reinforces the blocking point, drawn
   first from a workstream that is ahead or already completed.
3. **Scope reduction**: the feature is simplified, never removed. E.g. 2 embedded
   components instead of 3, a single save slot instead of named slots, triggers
   without network access for plug-ins.

### Prioritization

- **Protected coreis  game compatibility and plug-ins**: our main focus; never
  descoped, and blockers on these take precedence over everything else.
- **Adjustment variables, save states, embedded components, and final polish**:
  degradable without compromising the delivery.
- When arbitration is needed, effort flows from adjustment variables toward the
  protected core, never the reverse.