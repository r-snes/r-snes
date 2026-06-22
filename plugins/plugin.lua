function print_results()
    -- $0:81A2 is the address of the @end label of both test ROMs,
    -- which is a JMP instruction that jumps to itself (creating an
    -- infinite loop)
    if test_idx == 0x453 and rsnes.cpu.pb == 0 and rsnes.cpu.pc == 0x81A2 then
        print("Successfully reached end of cputest-basic, ran 0x453 (1107) tests")
    elseif test_idx == 0x64A and rsnes.cpu.pb == 0 and rsnes.cpu.pc == 0x81A2 then
        print("Successfully reached end of cputest-full, ran 0x64A (1610) tests")
    else
        print("Early exit: ROM did not complete")
        return
    end

    local ok=0
    local ko=0

    for i=0,(test_idx-1) do
        local res = test_results[i]
        -- print(i, res)
        if res then
            ok = ok + 1
        else
            ko = ko + 1
        end
    end
    print(ok .. " successes; " .. ko .. " failures")
end

return {
    permissions = { internal = { "cpu", "input" } },

    init = function()
        print("loaded rsnes? type: " .. type(rsnes))
        print("loaded regs? type: " .. type(rsnes.cpu))
        print("loaded regs? type: " .. type(rsnes.input))
        print("loaded require? type: " .. type(require))

        print("plugin: addrbus is currently " .. rsnes.cpu.bus_bank .. ":" .. rsnes.cpu.bus_addr)
        print("plugin: PB:PC is currently " .. rsnes.cpu.pb .. ":" .. rsnes.cpu.pc)
        rsnes.cpu.pb = 0xaa
        print("plugin: PB:PC is currently " .. rsnes.cpu.pb .. ":" .. rsnes.cpu.pc)

        test_results = {}
        test_idx = 0
    end,

    exit = print_results,

    actions = {
        default = function()
            print("Current PB:PC: " .. rsnes.cpu.pb .. ":" .. rsnes.cpu.pc .. "; test_idx: " .. test_idx)
        end,
    },

    autoactions = {
        on_instr = function(opcode, pb, pc)
            if pb ~= 0 then return end

            -- init_test addr: start of a test mark as success by default
            if pc == 0x8132 then
                test_results[test_idx] = true
                test_idx = test_idx + 1
                return
            end

            -- fail addr: test completed and failed, mark as failed
            if pc == 0x81AE then
                test_results[test_idx - 1] = false
                rsnes.input.press_a() -- we need to press then release to go next
                return
            end

            -- wait_release addr: test ROM is waiting for A to be released
            if pc == 0x824E then
                rsnes.input.release_a() -- release A to go to next test
                return
            end
        end,
    },
}
