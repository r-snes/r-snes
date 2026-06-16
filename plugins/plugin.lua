return {
    permissions = "all",
    init = function()
        print("loaded rsnes? type: " .. type(rsnes))
        print("loaded regs? type: " .. type(regs))

        print("plugin: addrbus is currently " .. regs.bus_bank .. ":" .. regs.bus_addr)
        print("plugin: PB:PC is currently " .. regs.pb .. ":" .. regs.pc)
        regs.pb = 0xaa
        print("plugin: PB:PC is currently " .. regs.pb .. ":" .. regs.pc)
    end,

    actions = {
        default = function()
            print("ran plugin default action")
        end,
    }
}
