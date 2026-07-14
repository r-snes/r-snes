return {
    permissions = {
        internal = {
            ppu = { "display" },
        },
    },

    actions = {
        default = function()
            -- CGRAM colours are in BGR555, with red in least significant
            -- bits, and blue in most signficant bits

            -- (B)01000 (G)00000 (B)00000 = 8192: dark blue
            rsnes.ppu.write_cgram(0, 8192)
            -- (B)11111 (G)11000 (B)11000 = 32536: light blue
            rsnes.ppu.write_cgram(1, 32536)
        end,
    },
}
