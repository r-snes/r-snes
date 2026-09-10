local emu_count = 0
local real_count = 0

return {
    permissions = {
        internal = { "input" }, -- adjust/add perms once these do real work
    },

    init = function()
        emu_count = 0
        real_count = 0
    end,

    autoactions = {
        -- scales with emulation speed, pauses when emulation pauses
        on_interval = {
            seconds = 5,
            action = function(elapsed_seconds)
                emu_count = emu_count + 1
                print("[emulated] tick #" .. emu_count .. " at t=" .. elapsed_seconds .. "s")
            end,
        },

        -- fires on real wall-clock seconds, independent of emulation speed
        on_real_interval = {
            seconds = 5,
            action = function(elapsed_seconds)
                real_count = real_count + 1
                print("[real]     tick #" .. real_count .. " at t=" .. elapsed_seconds .. "s")
            end,
        },
    },
}
