# Test fixtures

The `spc_tests*.spc` files in this folder are built from the `spctest` sources of [gilyon/snes-tests](https://github.com/gilyon/snes-tests), assembled with [spcasm](https://github.com/kleinesfilmroellchen/spcasm) (`-f plain`).

The `*_continue.spc` variants include a one-line patch (`jmp fail` → `call fail` in `spc_common.inc`) so that a run reports every failing test instead of halting on the first one.

## License

snes-tests is distributed under the MIT license, the same license as this project: these files are covered by the [LICENSE](../../../LICENSE) file at the root of this repository.