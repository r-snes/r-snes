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
        if res then
            ok = ok + 1
        else
            ko = ko + 1
        end
    end

    local total = ok + ko
    print(ok .. "/" .. total .. " passed (" .. (ok*100)/total .. "%), " .. ko .. " failures")

    save_results_to_file(ok, ko)
end

function save_results_to_file(ok, ko)
    local filename = nil
    if test_idx == 0x453 then
        filename = files.basic
    else
        filename = files.full
    end

    save_results_to(filename, ok, ko)
    print("Saved results to " .. filename)
end

-- with minimal perms
function save_results_to(filename, ok, ko)
    local total = ok + ko
    local file = rsnes.fs.files[filename]

    file.clear()
    file.write(ok, "/", total, " passed (", (ok*100)/total, "%), ", ko, " failures\n")
    if ko == 0 then
        return
    end

    file.write("All failures listed below 1 per line:\n")
    for i = 0,(test_idx-1) do
        if not test_results[i] then
            file.write(i, "\n")
        end
    end
end

-- we want to open files in "start" mode by default
-- because we're only going to write one file per
-- plugin run, so "truncate" would clear both files
-- and only write to one, erasing the contents of the other.
-- so instead we manually call file.clear() to clear
-- the file we are going to write to
write_opts = {
    mode = "start",
    create = true,
}

files = {
    basic = "cpu/cputest-basic-report.txt",
    full = "cpu/cputest-full-report.txt",
}

return {
    permissions = {
        internal = { "cpu", "input" },
        external = {
            filesystem = {
                write = {
                    [files.basic] = write_opts,
                    [files.full] = write_opts,
                }
            }
        }
    },

    init = function()
        test_results = {}
        test_idx = 0

        for file,val in pairs(rsnes.fs.files) do
            print("opened file:", file)
        end
    end,

    exit = print_results,

    actions = {
        default = function()
            print("Current PB:PC: " .. rsnes.cpu.pb .. ":" .. rsnes.cpu.pc .. "; test_idx: " .. test_idx)

            print_results()
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
