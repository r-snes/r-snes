function print_program()
    print("=== Program at i_count=" .. i_count .. " ===")

    for addr,opcode in pairs(program) do
        print(addr, opcode, program_read[addr])
    end
    print("==============")
end

return {
    permissions = {
        internal = { "cpu", "bus" },
    },

    init = function()
        print("Init spctest debug plugin")

        program = {}
        program_read = {}
        setmetatable(program_read, {
            __index = function() return 0 end,
        })
        i_count = 0

        track = false
    end,

    exit = function()
        print_program()
        print("A was " .. rsnes.cpu.A)
    end,

    actions = {
        default = function()
            track = not track
            rsnes.bus.write(0x2140, 0xaa)
        end,
    },

    autoactions = {
        on_instr = function(opcode, pb, pc)
            if not track then
                return
            end

            i_count = i_count + 1
            local addr = (pb << 16) + pc

            program[addr] = opcode
            program_read[addr] = program_read[addr] + 1
        end,
    },
}
