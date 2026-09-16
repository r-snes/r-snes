--
-- Autoplayer for https://github.com/alekmaul/pvsneslib/tree/master/snes-examples/systems/games/breakout
-- which you can get directly as a ROM by downloading the zip of a release,
-- for example: https://github.com/alekmaul/pvsneslib/releases/tag/4.6.0,
-- and running snes-examples/bin/breakout.sfc from the unzipped archived
--

local paddle_x = 0x7E3283
local ball_velx = 0x7E3285
local ball_vely = 0x7E3287
local ball_posx = 0x7E3289
local ball_posy = 0x7E328B

local autoplay_enable = true

return {
    permissions = {
        internal = {
            bus = { "read" },
            input = "all",
        },
    },

    actions = {
        default = function()
            if autoplay_enable then
                rsnes.input.release_left()
                rsnes.input.release_right()
            end
            autoplay_enable = not autoplay_enable
        end,
    },

    autoactions = {
        on_interval = {
            seconds = 1 / 100,
            action = function()
                if not autoplay_enable then
                    return
                end
                rsnes.input.release_left()
                rsnes.input.release_right()

                -- paddle_x is the left edge of the paddle, and the paddle is
                -- 30 pixels wide, so we offset by 15 tojget the center position
                local px = rsnes.bus.read(paddle_x) + 15
                local bx = rsnes.bus.read(ball_posx)

                if px > bx then
                    rsnes.input.press_left()
                elseif px < bx then
                    rsnes.input.press_right()
                end
            end,
        }
    },
}
