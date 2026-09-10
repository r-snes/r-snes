local real_count = 0

return {
    permissions = {
        internal = { "input" },
    },

    init = function()
        real_count = 0
    end,

    autoactions = {
        -- fires on real wall-clock seconds, independent of emulation speed
        on_real_interval = {
            seconds = 5,
            action = function(elapsed_seconds)
                real_count = real_count + 1
                print("[real] tick #" .. real_count .. " at t=" .. elapsed_seconds .. "s")
            end,
        },
    },
}