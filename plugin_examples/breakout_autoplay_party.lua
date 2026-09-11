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

local function bgr555(b, g, r)
    return ((b & 31) << 10) | ((g & 31) << 5) | (r & 31)
end
local r, g, b = 0, 0, 20
-- loops through the following colors:
-- - 00, 00, 20
-- - 00, 20, 20
-- - 00, 20, 00
-- - 20, 20, 00
-- - 20, 00, 00
-- - 20, 00, 20
-- - 00, 00, 20
-- ...
local function loop_colors()
    if r == 0 and g < 20 and b == 20 then
        g = g + 1
    elseif r == 0 and g == 20 and b > 0 then
        b = b - 1
    elseif r < 20 and g == 20 and b == 0 then
        r = r + 1
    elseif r == 20 and g > 0 and b == 0 then
        g = g - 1
    elseif r == 20 and g == 0 and b < 20 then
        b = b + 1
    elseif r > 0 and g == 0 and b == 20 then
        r = r - 1
    end
end

return {
    permissions = {
        internal = {
            bus = { "read" },
            ppu = "all",
            input = "all",
        },
    },

    actions = {
        default = function()
            if autoplay_enable then
                rsnes.input.release_left()
                rsnes.input.release_right()
                rsnes.ppu.write_cgram(0, 0)
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

                loop_colors()
                rsnes.ppu.write_cgram(0, bgr555(b, g, r))

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