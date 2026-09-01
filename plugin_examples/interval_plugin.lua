local INTERVAL_SECONDS = 5

-- Rough average CPU cycles spent per instruction on typical SNES code
local AVG_CYCLES_PER_INSTR = 6

-- Effective average SNES CPU clock (Hz)
local CPU_CLOCK_HZ = 3580000

local INSTR_PER_TICK =
    math.floor((INTERVAL_SECONDS * CPU_CLOCK_HZ) / AVG_CYCLES_PER_INSTR)

local instr_count = 0
local tick_count = 0

local function on_tick()
    tick_count = tick_count + 1
    print("[periodic] tick #" .. tick_count .. " (instr_count reset)")
end

return {
    permissions = {
        internal = { "ppu" },
    },

    init = function()
        instr_count = 0
    end,

    autoactions = {
        on_instr = function(opcode, pb, pc)
            instr_count = instr_count + 1
            if instr_count >= INSTR_PER_TICK then
                instr_count = 0
                on_tick()
            end
        end,
    },
}