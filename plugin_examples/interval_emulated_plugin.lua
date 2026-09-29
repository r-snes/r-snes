local emu_count = 0

return {
    permissions = {
        internal = { "input" },
    },

    init = function()
        emu_count = 0
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
    },
}
