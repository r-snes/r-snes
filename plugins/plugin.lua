return {
    permissions = { internal = { "cpu", "input" } },

    init = function()
        print("loaded rsnes? type: " .. type(rsnes))
        print("loaded regs? type: " .. type(rsnes.cpu))
        print("loaded regs? type: " .. type(rsnes.input))

        print("plugin: addrbus is currently " .. rsnes.cpu.bus_bank .. ":" .. rsnes.cpu.bus_addr)
        print("plugin: PB:PC is currently " .. rsnes.cpu.pb .. ":" .. rsnes.cpu.pc)
        rsnes.cpu.pb = 0xaa
        print("plugin: PB:PC is currently " .. rsnes.cpu.pb .. ":" .. rsnes.cpu.pc)

        test_results = {}
        test_idx = 0
    end,

    actions = {
        default = function()
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
            -- init_test addr: start of a test mark as success by default
            if (pc == 0x8132) then
                test_results[test_idx] = true
                test_idx = test_idx + 1
                return
            end
            -- fail addr: test completed and failed, mark as failed
            if (pc == 0x81AE) then
                test_results[test_idx - 1] = false
                rsnes.input.press_a() -- we need to press then release to go next
                return
            end
            -- wait_release addr: test ROM is waiting for A to be released
            if (pc == 0x824E) then
                rsnes.input.release_a() -- release A to go to next test
                return
            end
        end,
    },
}
