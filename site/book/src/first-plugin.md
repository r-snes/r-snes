# Writing your first plugin

In this tutorial you'll write a small plugin, step by step: first a simple "Hello, world!", then a plugin that reacts to a key press, reads the CPU state, and runs code at regular intervals.

You don't need to know Lua well to follow along. If you have never seen it, [Learn Lua in Y minutes](https://learnxinyminutes.com/docs/lua/) is a quick introduction.

## What you need

- R-SNES ([Getting started](getting-started.md)), started **from a terminal** so you can see what your plugin prints (on Windows, see [Seeing a plugin's messages](using-plugins.md#seeing-a-plugins-messages)).
- A text editor.
- A ROM to run the plugin with. The free [snes-tests](https://github.com/gilyon/snes-tests) ROMs are fine.

## Step 1: Hello, world!

Create a file named `hello.lua` with this content:

```lua
return {
    permissions = {},

    init = function()
        print("Hello, world!")
    end,
}
```

Every plugin is a Lua file that ends by **returning a table**: the plugin table. Here it has two fields:

- `permissions` lists what the plugin is allowed to access. This plugin only prints a message, so it asks for nothing: `{}`.
- `init` is a function R-SNES calls once, when the game starts.

Now run it:

1. Start R-SNES from a terminal.
2. **File > Load plugin…** (`Ctrl+P`) and select `hello.lua`.
3. The permission window shows that the plugin asks for nothing. Click **Grant requested permissions**.
4. Open a ROM with `Ctrl+L`.

Your terminal now shows:

```text
Hello, world!
```

Congratulations, you've written your first plugin!

> [!TIP]
> To test faster, load the plugin and the ROM from the command line. This skips the permission window, which is fine for your own plugin:
>
> ```sh
> r-snes --load-plugin-noconfirm hello.lua path/to/game.sfc
> ```

## Step 2: Say goodbye

`init` has a counterpart: `exit` runs once when the plugin stops, that is when you close the game. Add it to the table:

```lua
return {
    permissions = {},

    init = function()
        print("Hello, world!")
    end,

    exit = function()
        print("Goodbye!")
    end,
}
```

Load the plugin again, open a ROM, then press `Esc` to close it: `Goodbye!` shows up in the terminal.

## Step 3: React to a key press

A plugin can define an **action** that the player triggers by pressing `R`. Actions go in the `actions` table, under the name `default`:

```lua
local presses = 0

return {
    permissions = {},

    init = function()
        print("Hello, world!")
    end,

    actions = {
        default = function()
            presses = presses + 1
            print("R pressed " .. presses .. " times")
        end,
    },
}
```

Load the plugin again and open a ROM: `Hello, world!` shows up as before. Then, each time you press `R`, the plugin counts the presses:

```text
Hello, world!
R pressed 1 times
R pressed 2 times
R pressed 3 times
```

Notice the `local presses = 0` at the top: you can write any Lua code before the `return`, such as variables and helper functions. The plugin's functions can use them later.

## Step 4: Read the CPU state

So far the plugin can't see anything inside the console. Let's display the CPU's **program counter** (`PB:PC`, the address of the instruction being run) and **accumulator** (`A`).

That requires the `internal.cpu.registers` permission. Ask for it in the `permissions` table, following the [permission tree](plugin-api.md#actual-permission-tree):

```lua
-- Formats a number as hexadecimal with the given number of digits
local function hex(n, digits)
    local s = ""
    for _ = 1, digits do
        local d = n % 16
        s = string.sub("0123456789ABCDEF", d + 1, d + 1) .. s
        n = n // 16
    end
    return s
end

return {
    permissions = {
        internal = {
            cpu = { "registers" },
        },
    },

    actions = {
        default = function()
            local cpu = rsnes.cpu
            print("PC = $" .. hex(cpu.pb, 2) .. ":" .. hex(cpu.pc, 4) .. ", A = $" .. hex(cpu.a, 4))
        end,
    },
}
```

The `hex` helper turns register values into the hexadecimal notation usually used for SNES addresses (`$80F4` rather than `33012`).

Once the permission is granted, R-SNES gives the plugin a global `rsnes` table, with an `rsnes.cpu` table inside it to read the registers. Load this version: the permission window now lists `registers` under `internal` and `cpu`. Grant it, open a ROM and press `R` a few times:

```text
PC = $00:8123, A = $0000
PC = $00:80F4, A = $1F00
```

> [!NOTE]
> Without the permission, `rsnes.cpu` simply doesn't exist, and pressing `R` fails with an error about indexing a `nil` value. That's the sandbox at work: a plugin only gets what it asked for and you granted.

## Step 5: Run code automatically

Actions run when the player asks. **Autoactions** run on their own. `on_interval` calls a function every few seconds of emulated time (it stops while the game is paused):

```lua
-- (keep the hex helper from step 4 here)

return {
    permissions = {
        internal = {
            cpu = { "registers" },
        },
    },

    autoactions = {
        on_interval = {
            seconds = 2,
            action = function(elapsed_seconds)
                print(math.floor(elapsed_seconds) .. "s: PC = $" .. hex(rsnes.cpu.pb, 2) .. ":" .. hex(rsnes.cpu.pc, 4))
            end,
        },
    },
}
```

The other autoactions are described in the [Plugin API reference](plugin-api.md#autoactions): `on_instr` runs before every CPU instruction, and `on_real_interval` works like `on_interval` but counts real time.

## The complete plugin

Here's everything together:

```lua
-- hello.lua: a small plugin to discover the R-SNES plugin API

local presses = 0

-- Formats a number as hexadecimal with the given number of digits
local function hex(n, digits)
    local s = ""
    for _ = 1, digits do
        local d = n % 16
        s = string.sub("0123456789ABCDEF", d + 1, d + 1) .. s
        n = n // 16
    end
    return s
end

return {
    permissions = {
        internal = {
            cpu = { "registers" },
        },
    },

    init = function()
        print("Hello, world!")
    end,

    exit = function()
        print("Goodbye! R was pressed " .. presses .. " times")
    end,

    actions = {
        default = function()
            presses = presses + 1
            local cpu = rsnes.cpu
            print("PC = $" .. hex(cpu.pb, 2) .. ":" .. hex(cpu.pc, 4) .. ", A = $" .. hex(cpu.a, 4))
        end,
    },

    autoactions = {
        on_interval = {
            seconds = 2,
            action = function(elapsed_seconds)
                print(math.floor(elapsed_seconds) .. "s: PC = $" .. hex(rsnes.cpu.pb, 2) .. ":" .. hex(rsnes.cpu.pc, 4))
            end,
        },
    },
}
```

## Going further

- The [Plugin API reference](plugin-api.md) lists every permission and every `rsnes.*` function: reading and writing memory with `rsnes.bus`, changing colours with `rsnes.ppu`, pressing buttons with `rsnes.input`, reading and writing files…
- The [example plugins](using-plugins.md#example-plugins) show complete, real plugins.
- Made something cool? Share it in the [discussions](https://github.com/r-snes/r-snes/discussions)!
