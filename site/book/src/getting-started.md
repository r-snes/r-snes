# Getting started

This page explains how to install R-SNES and play your first game.

## Installing

### From a release

Download the file for your system from the [latest release](https://github.com/r-snes/r-snes/releases/latest):

| System | File |
| :------- | :----- |
| Windows | `r-snes_<version>_x86_64.exe` |
| Debian, Ubuntu, Linux Mint, Pop!_OS… | `r-snes_<version>_amd64.deb` |
| Fedora, Bazzite… | `r-snes_<version>_x86_64.rpm` |

On Windows, run the `.exe`. On Linux, install the package with your package manager:

```sh
# Debian, Ubuntu…
sudo apt install ./r-snes_<version>_amd64.deb

# Fedora…
sudo dnf install ./r-snes_<version>_x86_64.rpm
```

Each release also lists a `SHA256SUMS` file, to check that your download isn't corrupted.

macOS hasn't been tested yet. If you try it, please tell us how it went in the [discussions](https://github.com/r-snes/r-snes/discussions).

### With Nix

The repository's flake provides an R-SNES package. Run any released version directly, here `v0.1.0`:

```sh
nix run 'github:r-snes/r-snes?ref=v0.1.0'
```

### From source

You can also build R-SNES yourself. You need:

- the Rust toolchain (`cargo` and `rustc`), installed with [rustup](https://rustup.rs/);
- SDL2 on Linux (`libsdl2-dev` on Debian/Ubuntu, `SDL2-devel` on Fedora). On Windows, SDL2 is built and linked automatically;
- the usual desktop libraries on Linux (for example `libX11` and `libxkbcommon`).

Then get the source code and build it:

```sh
git clone https://github.com/r-snes/r-snes.git
cd r-snes
cargo build --release
```

The emulator ends up in `target/release/r-snes` (`r-snes.exe` on Windows). You can also build and launch it in one go with `cargo run --release`.

> [!TIP]
> If you use Nix, `nix develop` gives you a shell with every dependency needed to build R-SNES.

## Getting a game

R-SNES doesn't come with any game. You need a ROM file, usually with a `.sfc` or `.smc` extension. Only use ROMs you are legally allowed to use, for example dumps of cartridges you own. See our [legal notice](https://github.com/r-snes/r-snes/blob/main/LEGAL.md) for details.

To check that everything works, you can use the free test ROMs from [snes-tests](https://github.com/gilyon/snes-tests) (`cputest-full.sfc`, `spctest.sfc`), which R-SNES passes.

## Launching a game

There are two ways to open a game.

**From the window:**  
Open the **File** menu and choose **Open ROM…** (or press `Ctrl+L`), then pick your ROM file.

**From a terminal:**  
Give the ROM path as an argument:

```sh
r-snes path/to/game.sfc

# or, from the source folder
cargo run --release -- path/to/game.sfc
```

The game starts right away. Press `Esc` to close it and go back to the logo screen, or `Ctrl+Q` to quit.

Run `r-snes --help` to see every command-line option.

## While playing

The menu bar at the top of the window gives access to everything:

- **File**: open or close a ROM, [load a plugin](using-plugins.md), quit.
- **Emulation**: pause/resume, reset the console, run the plugin's action.
- **View**: show information about the loaded ROM (title, region, mapping mode, checksum…).

The game is paused while a menu is open, so nothing happens behind your back.

Next, learn the [controls](controls.md).
