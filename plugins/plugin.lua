return {
    permissions = "all",
    init = function()
        i = 10
        print("plugin initialised: i =", i)
    end,

    actions = {
        default = function()
            i = i + 1
            print("ran plugin default action, i =", i)
        end,
    }
}
