# R-SNES

R-SNES aims to be a decently fast and and very accurate SNES emulator, while staying easy to understand as per modern coding standards.

> [!NOTE]
> For now, this is still heavily a work in progress. We are barely able to run [a test ROM](https://github.com/gilyon/snes-tests) on our latest development branches, and are still quite far from being able to run actual games.
We're starting to work towards [plugins](#plugins), but this too is completely WIP.
>
> For the sake of brevity (and optimism!), many parts of this README will describe features of the emulator as if they were already functional, which is quite unlikely to be the case for now.

## Project purpose

Being a SNES emulator, R-SNES allows users to play SNES games. We also try to stay performant (low resource usage on the host machine) and accurate to our best effort (trying to make games behave the exact same as they would on the original console).

### Plugins

We also wish to make the player experience extensible with Lua plugins which can affect many parts of the emulator, be run automatically for each rendered frame, on each CPU cycle, or when a specific value in memory changes (among other trigger conditions).

Plugins will also be able to access things *outside* the emulator (which we refer to as "external"), which could allow them to, for example, read data from files, or save data extracted from a running game to a file, or even send them over the network in HTTP requests.

Of course, plugins being able to do so much can cause risks since they might spread widely to non-developer communities (to people who can't easily audit what the plugin they are running is doing).
For that, we require plugins to describe what permissions they require, and prompt the user to confirm that they do intend to grant said permissions to the plugin before running it, so they can at least raise an eyebrow if some plugin requires complete unrestricted access to the internet if it doesn't seem like it needs it.

## Installing

For now the only supported "install" method is building the emulator from source.

For this, you will need a local install of the rust toolchain, including `cargo` and `rustc`, and building should be as simple as getting a local copy of the code source and then running `cargo build --release` from the project root.

You may need some dependencies:
- SDL2
- Basic system-specific desktop libraries (for example, `libX11` and `libxkbcommon` for linux systems)

For now, the project is known to work on Linux (wayland) and Windows, but could also already work fine on other systems, give it a try!

## Project structure

Each component (hardware piece of the original console) is implemented in its own crate (thus in its own subfolder, see the up to date list of crates in the root Cargo.toml), and the main emulator program is implemented directly in `src/`.

## Language choice

The emulator is implemented in Rust. This choice of language is mostly by personal preference, but our preferences are also influenced by having worked with C and C++ for a few years, and we all come to agree it is easier to collaborate with Rust (even though we had far less experience with it at the start of the project) than with other programming languages which can compete in performance and low-level control such as C and C++.
