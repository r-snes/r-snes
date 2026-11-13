# R-SNES

[![CI](https://github.com/r-snes/r-snes/actions/workflows/actions.yaml/badge.svg?branch=main)](https://github.com/r-snes/r-snes/actions/workflows/actions.yaml)
[![Latest release](https://img.shields.io/github/v/release/r-snes/r-snes)](https://github.com/r-snes/r-snes/releases/latest)
[![Website](https://img.shields.io/website?url=https%3A%2F%2Fr-snes.github.io%2Fr-snes%2F)](https://r-snes.github.io/r-snes/)
[![GitHub License](https://img.shields.io/github/license/r-snes/r-snes?color=blue)](LICENSE)

R-SNES aims to be a decently fast and very accurate SNES emulator, while staying easy to understand as per modern coding standards.

> [!NOTE]
> For now, this is still a work in progress. We're able to run commercial games with no crashes, partially working graphics (some rendering features and effects are still missing), and working game sound.
> The [plugin system](#plugins) is functional, with several modules already implemented: RAM manipulation, CPU, PPU, display, and file read/write.

## Project purpose

Being a SNES emulator, R-SNES allows users to play SNES games. We also try to stay performant (low resource usage on the host machine) and accurate to our best effort (trying to make games behave the exact same as they would on the original console).

### Plugins

The player experience is extensible with Lua plugins, which can read and interact with many parts of the emulator (memory, PPU, controller inputs...). They can run automatically before each CPU instruction, at regular intervals (in emulated or real time), or when the player triggers them with a key press.

Plugins can also access things *outside* the emulator (which we refer to as "external"). For now, they can read and write files, for example to save data extracted from a running game. In the future, we would like to let them do more, such as sending data over the network in HTTP requests.

Of course, plugins being able to do so much can cause risks since they might spread widely to non-developer communities (to people who can't easily audit what the plugin they are running is doing).
For that, we require plugins to describe what permissions they require, and prompt the user to confirm that they do intend to grant said permissions to the plugin before running it, so they can at least raise an eyebrow if some plugin requires access to their files if it doesn't seem like it needs it.

If you want to try writing plugins yourself, follow our [first plugin tutorial](https://r-snes.github.io/r-snes/book/first-plugin.html), then check out the [plugin API documentation](https://r-snes.github.io/r-snes/book/plugin-api.html). If you have any question or suggestion about this documentation, plugins in general (or anything else), feel free to [open a discussion about it](https://github.com/r-snes/r-snes/discussions).

## Installing

To start using R-SNES, go to our [releases pages](https://github.com/r-snes/r-snes/releases), and download the file suited for your platform. We currently support:

- Windows: .exe build
- Debian-based linux distros (including Ubuntu, Linux Mint, Pop!_OS...): .deb package
- Fedora-based linux distros (including Bazzite): .rpm package
- Nix on linux: nix package available in our flake (`nix run 'github:r-snes/r-snes?ref=v0.1.0'`, or any other version you'd like)

For now, R-SNES isn't available in any public package repository, but feel free to request it to be added. MacOS support is unknown, we've not tested it, feel free to try a manual install and report to us.
If you have any trouble installing R-SNES, or want to discuss platform compatibility or integration in package repositories, please open a [discussion](https://github.com/r-snes/r-snes/discussions) about it.

Then, the [getting started guide](https://r-snes.github.io/r-snes/book/getting-started.html) explains how to launch a game, and lists the [controls](https://r-snes.github.io/r-snes/book/controls.html).

### Manual install/build

You can also manually build from source:

For this, you will need a local install of the Rust toolchain, including `cargo` and `rustc`. Building should be as simple as getting a local copy of the source code and then running `cargo build --release` from the project root.

You may need some system dependencies:

- SDL2
- Basic system-specific desktop libraries (for example, `libX11` and `libxkbcommon` on Linux)

## Project structure

Each component (hardware piece of the original console) is implemented in its own crate, in its own subfolder at the repository root: `cpu/`, `ppu/`, `apu/` and `bus/`, along with the plugin system in `plugins/` and a few helper crates (see the up-to-date list of workspace members in the root `Cargo.toml`). The main emulator program (front-end, window/GUI, and top-level glue between crates) lives directly in `src/`, and the project's website in `site/`.

The [architecture page](https://r-snes.github.io/r-snes/book/architecture.html) of our documentation describes each crate and how they work together, and the [API documentation](https://r-snes.github.io/r-snes/api/r_snes/index.html) is generated from the code.

## Language and technology choices

The emulator is implemented in Rust. This choice of language is mostly by personal preference, but our preferences are also influenced by having worked with C and C++ for a few years: we all came to agree that it is easier to collaborate with Rust (even though we had far less experience with it at the start of the project) than with other programming languages that can compete in performance and low-level control, such as C and C++.

Beyond the language itself, the project's front-end relies on SDL2 for windowing, with egui layered on top for easier UI handling.

## Contributing

R-SNES is open to contributions! If you're looking for a place to start, check the issues labelled [`good first issue`](https://github.com/r-snes/r-snes/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22). Before opening a pull request, please read our [contributing guide](CONTRIBUTING.md): it explains what we accept, and the quality bar every change must meet.

You can also help by [reporting bugs](https://github.com/r-snes/r-snes/issues), or by sharing your questions, ideas and plugins in the [discussions](https://github.com/r-snes/r-snes/discussions).

## License

R-SNES is released under the [MIT license](LICENSE). It doesn't include any ROM, BIOS or firmware: see our [legal notice](LEGAL.md).
