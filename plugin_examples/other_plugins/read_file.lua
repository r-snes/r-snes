return {
    permissions = {
        external = {
            filesystem = {
                files = {
                    ["plugin_examples/example.txt"] = "read_only",
                }
            },
        },
    },

    init = function()
        local contents = rsnes.fs.files["plugin_examples/example.txt"].read("a")
        print("======= Read file contents: =======")
        print(contents)
        print("===================================")
    end,
}
