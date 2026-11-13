# Using plugins

Plugins are Lua files (`.lua`) that add features to the emulator: cheats, game autoplayers, tools that log what the console is doing, and more. This page explains how to run one.

## Loading a plugin

Plugins are loaded **before** starting a game:

1. Start R-SNES. If a game is running, close it with `Esc`.
2. Open the **File** menu and choose **Load plugin…** (or press `Ctrl+P`), then pick the `.lua` file.
3. A **Plugin Permissions** window lists everything the plugin asks for (see [below](#reviewing-permissions)). Click **Grant requested permissions** to accept, or **Cancel plugin execution** to refuse.
4. Open a ROM (`Ctrl+L`). The plugin starts together with the game.

When you close the game, the plugin stops too. Load it again before opening the next game if you still want it.

> [!NOTE]
> To read the permissions a plugin asks for, R-SNES runs the plugin file when you load it, so any code written before its final `return` runs at that moment, before you see the permission window. This code can't reach the emulator, since the `rsnes` table only exists once permissions are granted, and Lua's file and system libraries aren't available to plugins.

## Reviewing permissions

Plugins run in a sandbox: by default they can't see or change anything, neither in the console nor on your computer. To do something useful, a plugin must ask for **permissions**, and the permission window shows them as a tree, for example:

- `internal.cpu.registers`: read the CPU registers;
- `internal.bus.write`: write anywhere in the console's memory;
- `internal.input`: press controller buttons in your place;
- `external.filesystem.files`: open specific files on your computer.

Permissions under `internal` only affect the emulated console. Permissions under `external` reach outside of it, onto your computer: be careful and only grant them to plugins you trust. Ask yourself whether the plugin really needs what it asks for; a cheat plugin has no reason to read your files.

Tick **Show 'none' fields** to also see the permissions the plugin did *not* ask for. Closing the window without choosing (or pressing `Esc`) refuses the plugin.

The full list of permissions is in the [Plugin API reference](plugin-api.md#actual-permission-tree).

## Running a plugin's action

Some plugins work on their own: they run code on every CPU instruction, or every few seconds. Others have an **action** you trigger yourself: press `R` (or use **Emulation > Run plugin action**) while the game runs.

Check the plugin's own instructions (usually comments at the top of the file) to know what it does.

## Seeing a plugin's messages

Plugins can print messages with Lua's `print` function. These messages go to the terminal R-SNES was started from, so start R-SNES from a terminal to see them:

```sh
r-snes
# or, from the source folder
cargo run --release
```

On Windows, release builds don't open a console. Build with the `console` feature to get one: `cargo run --release --features console`.

If a plugin crashes, R-SNES shows its error in a window.

## Loading a plugin from the command line

You can start a game with a plugin directly:

```sh
r-snes --load-plugin-noconfirm my_plugin.lua path/to/game.sfc
```

> [!WARNING]
> As its name says, this option grants **every** permission the plugin asks for, **without asking you**. Only use it with plugins you wrote or fully trust.

## Example plugins

The [`plugin_examples/`](https://github.com/r-snes/r-snes/tree/main/plugin_examples) folder of the repository has a few plugins to try:

| Plugin | What it does |
| :------- | :------------- |
| `rom_test_blue.lua` | Press `R` to cycle the colours of the [snes-tests](https://github.com/gilyon/snes-tests) screens (blue, green, red). |
| `breakout_autoplay.lua` | Plays the [PVSnesLib breakout example](https://github.com/alekmaul/pvsneslib) by itself, by reading the ball position in memory and moving the paddle. |
| `interval_plugin.lua` | Prints a message every 5 seconds, in emulated time and in real time. |
| `read_file.lua` | Reads `plugin_examples/example.txt` and prints it (run R-SNES from the repository root). |
| `error_default.lua` | Shows what happens when a plugin uses something it didn't ask permission for. |

Want to make your own? Head to [Writing your first plugin](first-plugin.md).
