return {
    permissions = "all",
    init = function()
        print("loaded rsnes? type: " .. type(rsnes))
        print("loaded regs? type: " .. type(rsnes.cpu))

        print("plugin: addrbus is currently " .. rsnes.cpu.bus_bank .. ":" .. rsnes.cpu.bus_addr)
        print("plugin: PB:PC is currently " .. rsnes.cpu.pb .. ":" .. rsnes.cpu.pc)
        rsnes.cpu.pb = 0xaa
        print("plugin: PB:PC is currently " .. rsnes.cpu.pb .. ":" .. rsnes.cpu.pc)
    end,

    actions = {
        default = function()
            print("ran plugin default action, addr: " .. rsnes.cpu.bus_addr)
        end,
    }
}
