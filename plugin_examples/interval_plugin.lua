local tick_count = 0

return {
    permissions = {
        internal = { "input" }, --placeholder permission for testing purposes
    },

    init = function()
        tick_count = 0
    end,

    autoactions = {
        on_tick = {
            seconds = 3,
            action = function(elapsed_seconds)
                tick_count = tick_count + 1
                print("[periodic] tick #" .. tick_count .. " at t=" .. elapsed_seconds .. "s (emulated)")
            end,
        },
    },
}
