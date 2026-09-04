local interval_count = 0

return {
    permissions = {
        internal = { "input" }, --placeholder permission for testing purposes
    },

    init = function()
        interval_count = 0
    end,

    autoactions = {
        on_interval = {
            seconds = 5,
            action = function(elapsed_seconds)
                interval_count = interval_count + 1
                print("[periodic] interval #" .. interval_count .. " at t=" .. elapsed_seconds .. "s (emulated)")
            end,
        },
    },
}
