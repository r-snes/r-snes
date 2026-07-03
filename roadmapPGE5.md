# RSNES: Roadmap

## Overview

With a team of 4 and a mostly complete core, the project begins with a clear priority
focus. Feature 1 and 3 are completed first before moving on to Feature 2 and 4. Integration and
polish runs continuously throughout the entire project.

---

## Feature 1: Plugin System

Introduce a modular plugin architecture allowing external plugins to interact with game
ROMs at runtime. The emulator integrates a virtual machine (VM) which serves as the
communication layer between plugins and the emulator, providing a sandboxed and
controlled execution environment. Plugins are loaded dynamically, without modifying
the core codebase.

### Network Module
Add network capabilities to plugins, enabling them to interact with game ROMs over a
network. The exact scope and features of network support are still to be determined.

### Launch Conditions
Define the conditions under which a plugin is triggered during gameplay. Rather than
running continuously, plugins are activated by specific events, giving precise control
over when and how they interact with a ROM. Known candidate conditions include:

- **On a timer**: trigger the plugin at a fixed interval or after a set delay
- **On key press**: activate when the player presses a specific input
- **On a specific in-game event**: fire when a particular state or condition is
  detected in the ROM (e.g. a memory value changing, a scene loading)

> The full list of supported launch conditions is to be determined at a later date.

---

## Feature 2: Cartridge Components

The SNES used a variety of cartridge board configurations, each with different memory
mapping schemes and co-processors. Accurate emulation of these components is essential
for broad game compatibility.

### LoROM (Mode 20)
The most common memory map on the SNES. ROM data is mapped in 32 KB banks across the
lower address space. Most early and mid-generation titles use this layout.

### HiROM (Mode 21)
Maps the full 64 KB ROM banks into the upper address space, allowing for larger
cartridge capacities without bank switching overhead. Used by many later and larger titles.

### SA-1
A secondary 65C816 CPU embedded in certain cartridges (e.g. Super Mario RPG, Kirby
Super Star). It runs in parallel with the main CPU and handles fast math, DMA
transfers, and bitmap operations. Emulating it requires a separate execution context
and memory arbitration logic.

> **Goal:** implement at least 2–3 of these components and pass the majority of tests
> in the SNES test ROM suite.

---

## Feature 3: Game Compatibility

Validate end-to-end emulation by successfully launching and running real commercial SNES
titles. This milestone serves as a practical integration test across CPU, PPU, APU, and
cartridge emulation layers.

- Boot and reach gameplay in at least one commercial title
- Identify and document any remaining compatibility gaps

---

## Feature 4: Save States

Allow players to snapshot and restore emulator state at any moment during gameplay.
A save state captures the full machine state: CPU registers, RAM contents, PPU state,
APU state, and cartridge SRAM. Restoring a state replays that exact moment.

- Serialize and deserialize full machine state to/from disk
- Support at least one save slot per game
- Stretch goal: multiple named slots with metadata (timestamp, screenshot thumbnail)

## Timeline (31 August 2026 – mid-February 2027)

With a team of 4 and a mostly complete core, the project begins with a clear priority
focus. F1 and F3 are completed first before moving on to F2 and F4. Integration and
polish runs continuously throughout the entire project.

| Milestone                 | Start            | End                  | Duration  |
|---------------------------|------------------|----------------------|-----------|
| M1 — Focused on F1 & F3        | 31 August 2026   | 15 November 2026     | ~2.5 months |
| M2 — Focused on F2 & F4 | 15 November 2026 | 15 January 2027      | ~2 months |
| Finishin Touches & Polish      | 15 January 2027   | 15 February 2027     | ~1 months |

### Key Dates

- **31 August 2026**: PGE5 start.
- **15 November 2026**: End of the first Milestone focused on F1 and F3.
- **15 January 2027**: End of the second Milestone focused on F2 and F4. 
- **15 January – 15 February 2027**: Final integration, polish, and wrap-up.
- **15 February 2027**: Final delivery

### Priorities

- **P1 — Plugin System (F1) and Game Compatibility (F3):** tackled first from day one;
  blockers on these take precedence over everything else
- **P2 — Cartridge Components (F2) and Save States (F4):** begin once P1 milestones
  are complete
- **Ongoing — Integration & Polish:** runs continuously from kickoff to final delivery, with it being the focus after M2 until the delivery.
