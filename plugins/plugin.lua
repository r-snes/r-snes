return {
    permissions = "all",
    init = function()
        print("loaded rsnes? type: " .. type(rsnes))
        print("loaded regs? type: " .. type(rsnes.cpu))

        print("plugin: addrbus is currently " .. rsnes.cpu.bus_bank .. ":" .. rsnes.cpu.bus_addr)
        print("plugin: PB:PC is currently " .. rsnes.cpu.pb .. ":" .. rsnes.cpu.pc)
        rsnes.cpu.pb = 0xaa
        print("plugin: PB:PC is currently " .. rsnes.cpu.pb .. ":" .. rsnes.cpu.pc)

        instrs = {}

        test_results = {}
        test_idx = 0
    end,

    actions = {
        default = function()
            -- print("program so far:")

            -- for addr,data in pairs(instrs) do
            --     print(addr, data[1] .. "×" .. data[2])
            -- end

            print("test results:")
            local ok=0
            local ko=0

            for i=0,(test_idx-1) do
                local res = test_results[i]
                print(i, res)
                if (res) then
                    ok = ok + 1
                else
                    ko = ko + 1
                end
            end
            print("OK: " .. ok .. ", KO: " .. ko)
        end,
    },

    autoactions = {
        on_instr = function(opcode, pb, pc)
            -- -- we're never interested in anything where pb is not 0
            -- if (pb ~= 0) then return end

            -- init_test addr: start of a test mark as success by default
            if (pc == 0x8132) then
                test_results[test_idx] = true
                test_idx = test_idx + 1
                return
            end
            -- fail addr: test completed and failed, mark as failed
            if (pc == 0x81AE) then
                test_results[test_idx - 1] = false
                return
            end

            -- local addr = (pb << 16) + pc

            -- if (instrs[addr] == nil) then
            --     instrs[addr] = { opcode, 1 }
            -- else
            --     instrs[addr][2] = instrs[addr][2] + 1
            -- end
        end,
    },
}
