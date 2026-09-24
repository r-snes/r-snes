# R-SNES Plugin API Documentation

R-SNES can load and execute lua plugins to extend the functionnality of games.

Plugins can access almost all parts of the emulated hardware as well as some elements of the host system (the user's machine). To get access to anything, a plugin has to request a permission, which will inform the user of what the plugin is trying to access.<br>
This is also meant to allow *you*, the plugin developer, to clearly express what your plugin requires, and what it doesn't, to make it easier for users to trust that your plugin won't do anything malicious (in cases where it doesn't even request permissions for things that can be harmful).

## Plugin table overview

Here's an example of what a plugin file looks like:

```lua
return {
    permissions = {},

    init = function() end,
    exit = function() end,

    actions = {},
    autoactions = {},
}
```

Basically, plugin files must end with a `return` statement which returns a "plugin table". You're free to declare global variables, functions and such before the return statement.

> [!NOTE]
> Any code you write before this `return` will run before the
> permission request pop-up, which means it will run without any
> permissions.<br>
> It's still fine to define functions which will use permissions
> if they're only called once the permissions are granted (called from
> functions passed into the plugin table)

The `permissions` field declares what permissions the plugin request, as described in [**Permission requests**](#permission-requests)<br>
`init` and `exit` are the very first and very last bits of code that your plugin will run with its granted permissions. `init` is run once, right after the plugin received permissions. `exit` is run once, just before your plugin is unloaded.<br>
`actions` is a table which defines some functions that the user will be able to manually call, see [**Manual actions**](#manual-actions-user-run).<br>
`autoactions` defines functions which will called automatically by the emulator, see [**Autoactions**](#autoactions).

All of these fields can be omitted, in which case they will default to doing nothing

## Permission requests

The permission table is how plugins describe what permissions they request to the user.

### Permission table syntax

Permission requests follow a permission tree, here's a simple
example of one:
```
+-foo
| +-a
| +-b
|
+-bar
|
+-baz
  +-c
  | +-ca
  | +-cb
  |
  +-d
    +-da
    +-db
```

For this simple example, let's say all the leaf nodes are booleans:
| Permission tree leaf node | Type |
|---------------------------|------|
| `foo.a`                   | bool |
| `foo.b`                   | bool |
| `bar`                     | bool |
| `baz.c.ca`                | bool |
| `baz.c.cb`                | bool |
| `baz.d.da`                | bool |
| `baz.d.db`                | bool |

One way of requesting permissions with this permission tree is to
specify each field:
```lua
return {
    permissions = {
        foo = { a = true, b = true },
        bar = true,
        baz = {
            c = { ca = false, cb = false },
            d = { da = true, db = true },
        },
    },
}
```

R-SNES supports several ways to request these exact permissions in a more concise and forward-compatible way:
- To start with, since we request *everything* under `foo` and `baz.d`, and *nothing* under `baz.c`, we can use the `"all"` and `"none"` values which will recursively request either all or none of the leaf nodes:
  ```lua
  return {
      permissions = {
          foo = "all",
          bar = true,
          baz = {
              c = "none",
              d = "all",
          },
      },
  }
  ```
- If you really want to be explicit about requesting no permissions from `baz.c`, you can leave `c = "none"`, but you can get the exact same result by simply not mentioning `c` at all, nodes that are omitted default to their `"none"` value:
  ```lua
  return {
      permissions = {
          foo = "all",
          bar = true,
          baz = {
              d = "all",
          },
      },
  }
  ```
- If you want your plugin to be forward compatible with later versions which might divide `bar` into several nodes, you can replace its `true` value with `"all"`, it will produce the exact same result (likewise, `"none"` can be used in place of `false`).
  ```lua
  return {
      permissions = {
          foo = "all",
          bar = "all",
          baz = {
              d = "all",
          },
      },
  }
  ```
- Lastly, you can replace instances of `node = "all"` with just `"node"`, to rewrite `baz.d` for example:
  ```lua
  return {
      permissions = {
          foo = "all",
          bar = "all",
          baz = { "d" },
      },
  }
  ```
  By the same logic, you can even get it down to
  ```lua
  return {
      permissions = { "foo", "bar", baz = { "d" } },
  }
  ```


### Actual permission tree

```
+-internal
| +-cpu
| | +-registers
| |
| +-ppu
| | +-cgram
| |
| +-bus
| | +-read
| | +-write
| |
| +-input
| |
| +-emulator
|   +-dialog
|   +-pause
|
+-external
  +-filesystem
  | +-files
  |
  +-http
```

| Permission tree leaf node   | Type   | Functions/Objects injected |
|-----------------------------|--------|----------------------------|
| `internal.cpu.registers`    | bool\* | `rsnes.cpu`                |
| `internal.ppu.cgram`        | bool\* | `rsnes.ppu.write_cgram`    |
| `internal.bus.read`         | bool\* | `rsnes.bus.read`           |
| `internal.bus.write`        | bool\* | `rsnes.bus.write`          |
| `internal.input`            | bool\* | `rsnes.input`              |
| `internal.emulator.dialog`  | bool\* | *nothing* (unimplemented)  |
| `internal.emulator.pause`   | bool\* | *nothing* (unimplemented)  |
| `external.filesystem.files` | table of `<filename> = <open_mode>` | `rsnes.files` |
| `external.http`             | bool\* | *nothing* (unimplemented)  |

\* Fields noted "bool\*" are currently boolean (`true`/`false`) but might be split into more granular permissions in the future, so it is recommended to only pass `"all"` or `"none"` instead of `true`/`false` for your plugin to be forward-compatible

## Registering actions

quick intro to actions in general?

### Manual actions (user-run)

desc of everything under `actions` in the plugin table (only `default` for now?)

### Autoactions

desc of everything under `autoactions` in the plugin table: `on_instr` and `on_interval`

## R-SNES functions/objects

### `rsnes.cpu`

`rsnes.cpu` gives read-only access to all CPU registers and to the address bus:
| Field                   | Type    | Description |
|-------------------------|---------|-------------|
| `cpu.pc`/`cpu.PC`       | Integer | **P**rogram **B**ank: bank number where the CPU is reading code |
| `cpu.pc`/`cpu.PC`       | Integer | **P**rogram **C**ounter: address within PB where the CPU is reading code |
| `cpu.a`/`cpu.A`         | Integer | General purpose **A**cumulator |
| `cpu.x`/`cpu.X`         | Integer | Primary index register |
| `cpu.y`/`cpu.Y`         | Integer | Secondary index register |
| `cpu.d`/`cpu.D`         | Integer | **D**irect page register |
| `cpu.db`/`cpu.DB`       | Integer | **D**ata **B**ank register |
| `cpu.s`/`cpu.S`         | Integer | **S**tack pointer (within bank 0) |
| `cpu.p`/`cpu.P`         | Integer | **P**rocessor status register |
| `cpu.e`/`cpu.E`         | Boolean | **E**mulation bit (true when the CPU is in 8-bit compatibility ("emulation") mode) |
| `cpu.bus_bank`          | Integer | Bank number where the CPU is reading/writing in memory |
| `cpu.bus_addr`          | Integer |  Address within `cpu.bus_bank` where the CPU is reading/writing in memory |

The values read are always up to date: reads in `rsnes.cpu` forward to reading directly from the CPU, and writes are ignored.

### `rsnes.ppu`

`rsnes.ppu` gives various accesses to the **P**icture **P**rocessing **U**nit, which is responsible for all the image rendering.

The PPU has 3 memory units which plugins can (**will** in the near future, a lot is still to do) interact with:
- [CGRAM](https://snes.nesdev.org/wiki/Palettes): color palette which stores 256 15-bit colours which sprites and backgrounds refer to by index to describes colours to render.
- VRAM: 64KiB memory region storing [tile data](https://snes.nesdev.org/wiki/Tiles) (which tiles exist) and [tile map](https://snes.nesdev.org/wiki/Tilemaps) (which tile data is used for each on-screen tile).
- [OAM](https://snes.nesdev.org/wiki/Sprites): sprite memory stores which tiles (from VRAM) sprites are made of and the position of sprites on screen

| Function                              | Arguments      | Return value |
|---------------------------------------|----------------|--------------|
| `rsnes.ppu.write_cgram(index, color)` | `index`: cgram index 0-255, `color`: 15-bit BGR colour | `nil` |

(`read_cgram` and other functions for VRAM and OAM will come in the future)

### `rsnes.bus`

`rsnes.bus` gives full access to the entire addressable memory space, following the [SNES memory map](https://snes.nesdev.org/wiki/Memory_map):
- Cartridge:
  - ROM
  - S-RAM (save data)
- IO Registers
- RAM

| Function                       | Arguments      | Return value |
|--------------------------------|----------------|--------------|
| `rsnes.bus.read(addr)`         | `addr`: global SNES address 0-0xFFFFFF | Read byte (Integer 0-255) |
| `rsnes.bus.write(addr, value)` | `addr`: global SNES address 0-0xFFFFFF, `value`: byte to write | `nil` |

> [!WARN]
> Accessing the IO zone with `rsnes.bus` can have intricate side effects compared to other memory regions (i.e. even only reading certain registers can affect internal state, which could alter further reads from the CPU), so where possible you should use other functions such as `rsnes.ppu.write_cgram` to directly access the raw memory instead of going through the IO interface.

### `rsnes.input`

`rsnes.input` allows plugins to "press" controller buttons in place of the player

| Function                     | Arguments | Return value |
|------------------------------|-----------|--------------|
| `rsnes.input.press_a`        | *none*    | `nil`        |
| `rsnes.input.press_b`        | *none*    | `nil`        |
| `rsnes.input.press_x`        | *none*    | `nil`        |
| `rsnes.input.press_y`        | *none*    | `nil`        |
| `rsnes.input.press_up`       | *none*    | `nil`        |
| `rsnes.input.press_down`     | *none*    | `nil`        |
| `rsnes.input.press_left`     | *none*    | `nil`        |
| `rsnes.input.press_right`    | *none*    | `nil`        |
| `rsnes.input.press_l`        | *none*    | `nil`        |
| `rsnes.input.press_r`        | *none*    | `nil`        |
| `rsnes.input.press_select`   | *none*    | `nil`        |
| `rsnes.input.press_start`    | *none*    | `nil`        |
|                              |           |              |
| `rsnes.input.release_a`      | *none*    | `nil`        |
| `rsnes.input.release_b`      | *none*    | `nil`        |
| `rsnes.input.release_x`      | *none*    | `nil`        |
| `rsnes.input.release_y`      | *none*    | `nil`        |
| `rsnes.input.release_up`     | *none*    | `nil`        |
| `rsnes.input.release_down`   | *none*    | `nil`        |
| `rsnes.input.release_left`   | *none*    | `nil`        |
| `rsnes.input.release_right`  | *none*    | `nil`        |
| `rsnes.input.release_l`      | *none*    | `nil`        |
| `rsnes.input.release_r`      | *none*    | `nil`        |
| `rsnes.input.release_select` | *none*    | `nil`        |
| `rsnes.input.release_start`  | *none*    | `nil`        |

### `rsnes.files`
