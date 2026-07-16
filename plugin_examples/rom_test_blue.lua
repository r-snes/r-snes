-- CGRAM colours are in BGR555, with red in least significant
-- bits, and blue in most signficant bits
colors = {
    {
        -- (B)01000 (G)00000 (B)00000 = 8192: dark blue
        bg = 8192,
        -- (B)11111 (G)11000 (B)11000 = 32536: light blue
        fg = 32536,
    },
    {
        -- (B)00000 (G)01000 (B)00000 = 8192: dark green
        bg = 512,
        -- (B)11000 (G)11111 (B)11000 = 32536: light green
        fg = 25592,
    },
    {
        -- (B)00000 (G)00000 (B)01000 = 8192: dark red
        bg = 8,
        -- (B)11000 (G)11000 (B)11111 = 32536: light red
        fg = 25375,
    },
}

return {
    permissions = {
        internal = {
            ppu = { "display" },
        },
    },

    init = function()
        color_index = 1
    end,

    actions = {
        default = function()
            color_index = (color_index + 1)
            rsnes.ppu.write_cgram(0, colors[color_index].bg)
            rsnes.ppu.write_cgram(1, colors[color_index].fg)

            color_index = math.fmod(color_index, #colors)
        end,
    },
}
