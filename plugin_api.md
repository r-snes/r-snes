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

## Permission requests

format of the perm table, full perm tree + map between perm requests and injected functions

## Registering actions

quick intro to actions in general?

### Manual actions (user-run)

desc of everything under `actions` in the plugin table (only `default` for now?)

### Autoactions

desc of everything under `autoactions` in the plugin table: `on_instr` and `on_interval`

## R-SNES functions

doc of all functions, like `rsnes.bus.read`, `rsnes.ppu.write_cgram` etc.
