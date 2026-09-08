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

return {
    permissions = {
        internal = {
            bus = { "read" },
            input = "all",
        },
    },

    actions = {
        default = function()
            rsnes.input.release_left()
            rsnes.input.release_right()

            -- paddle_x is the left edge of the paddle, and the paddle is
            -- 30 pixels wide, so we offset by 15 to get the center position
            local px = rsnes.bus.read(paddle_x) + 15
            local bx = rsnes.bus.read(ball_posx)

            if px > bx then
                rsnes.input.press_left()
            elseif px < bx then
                rsnes.input.press_right()
            end
        end,
    },
}
