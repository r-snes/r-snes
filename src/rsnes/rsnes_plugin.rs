use super::RSnesCore;

use common::snes_address::SnesAddress;
use cpu::cpu::CPU;
use piccolo::Callback;
use piccolo::CallbackReturn;
use piccolo::Context;
use piccolo::IntoValue;
use piccolo::Table;
use piccolo::Value;
use piccolo::error::LuaError;
use plugins::perm_tree::BusPermissions;
use plugins::perm_tree::FileSystemPermissions;
use plugins::perm_tree::FileWritePermissions;
use plugins::perm_tree::filesystem::FileWriteOptions;
use plugins::permission::Permission;
use plugins::permission::helpers::AllOr;
use plugins::plugin::Plugin;
use std::cell::RefCell;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::rc::Rc;

impl RSnesCore {
    /// Injects emulator callbacks in the lua VM contained
    /// in the Plugin parameter, taking into account the
    /// permission table of the plugin
    pub fn inject_into_lua(emu: &Rc<RefCell<Self>>, plugin: &mut Plugin) {
        plugin.lua.load_core();
        plugin.lua.enter(|ctx| {
            let rsnes = Table::new(&ctx);
            ctx.set_global("rsnes", rsnes);

            if plugin.table.perms.internal.cpu.registers {
                rsnes.set_field(ctx, "cpu", Self::create_regs_table(ctx, emu.clone()));
            }
            if plugin.table.perms.internal.input {
                rsnes.set_field(ctx, "input", Self::create_input_table(ctx, emu));
            }
            if !plugin.table.perms.internal.bus.is_none() {
                rsnes.set_field(
                    ctx,
                    "bus",
                    Self::create_bus_table(ctx, emu, &plugin.table.perms.internal.bus),
                );
            }
            if !plugin.table.perms.external.filesystem.is_none() {
                rsnes.set_field(
                    ctx,
                    "fs",
                    Self::create_fs_table(ctx, &plugin.table.perms.external.filesystem),
                );
            }
        });
    }

    /// Creates a lua table which gives read-only access to fields
    /// of the CPU: registers and address bus.
    ///
    /// The returned table uses a metatable to catch read and
    /// write "requests" instead of exposing the CPU fields direcly,
    /// which also means this table is "always up to date", since it
    /// dynamically reads from the CPU when a field is read from it.
    fn create_regs_table<'gc>(ctx: Context<'gc>, emu: Rc<RefCell<Self>>) -> Table<'gc> {
        let ret = Table::new(&ctx);
        let mt = Table::new(&ctx);
        ret.set_metatable(ctx.mutation(), Some(mt));

        mt.set_field(ctx, "__metatable", "private CPU regs metatable");
        mt.set_field(
            ctx,
            "__newindex",
            Callback::from_fn(ctx.mutation(), |ctx, _, mut stack| {
                let _: (Table, Value, Value) = stack.consume(ctx)?;
                println!("user code tried to write to cpu regs");
                Ok(piccolo::CallbackReturn::Return)
            }),
        );
        mt.set_field(
            ctx,
            "__index",
            Callback::from_fn(ctx.mutation(), move |ctx, _, mut stack| {
                let (_, key): (Table, Value) = stack.consume(ctx)?;
                let Value::String(key) = key else {
                    stack.replace(ctx, Value::Nil);
                    return Ok(piccolo::CallbackReturn::Return);
                };
                let cpu: &CPU = &emu.borrow().cpu;

                let val = match key.as_bytes() {
                    b"pc" | b"PC" => Value::Integer(cpu.regs().PC.into()),
                    b"pb" | b"PB" => Value::Integer(cpu.regs().PB.into()),
                    b"a" | b"A" => Value::Integer(cpu.regs().A.into()),
                    b"x" | b"X" => Value::Integer(cpu.regs().X.into()),
                    b"y" | b"Y" => Value::Integer(cpu.regs().Y.into()),
                    b"d" | b"D" => Value::Integer(cpu.regs().D.into()),
                    b"db" | b"DB" => Value::Integer(cpu.regs().DB.into()),
                    b"s" | b"S" => Value::Integer(cpu.regs().S.into()),
                    b"p" | b"P" => Value::Integer(u8::from(cpu.regs().P).into()),

                    b"e" | b"E" => Value::Boolean(cpu.regs().E),

                    b"bus_addr" => Value::Integer(cpu.addr_bus().addr.into()),
                    b"bus_bank" => Value::Integer(cpu.addr_bus().bank.into()),

                    _ => Value::Nil,
                };

                stack.replace(ctx, val);
                Ok(piccolo::CallbackReturn::Return)
            }),
        );

        ret
    }

    fn create_input_table<'gc>(ctx: Context<'gc>, emu: &Rc<RefCell<Self>>) -> Table<'gc> {
        let ret = Table::new(ctx.mutation());

        let clone = emu.clone();
        ret.set_field(
            ctx,
            "press_a",
            Callback::from_fn(ctx.mutation(), move |_, _, _| {
                let mut emu = clone.borrow_mut();

                emu.bus.io.hvbjoy = 0;
                emu.bus.io.joy1 = !0;
                Ok(piccolo::CallbackReturn::Return)
            }),
        );

        let clone = emu.clone();
        ret.set_field(
            ctx,
            "release_a",
            Callback::from_fn(ctx.mutation(), move |_, _, _| {
                let mut emu = clone.borrow_mut();

                emu.bus.io.hvbjoy = 0;
                emu.bus.io.joy1 = 0;
                Ok(piccolo::CallbackReturn::Return)
            }),
        );

        ret
    }

    fn create_bus_table<'gc>(
        ctx: Context<'gc>,
        emu: &Rc<RefCell<Self>>,
        bus_perms: &BusPermissions,
    ) -> Table<'gc> {
        let ret = Table::new(ctx.mutation());
        if bus_perms.read {
            let clone = emu.clone();
            ret.set_field(
                ctx,
                "read",
                Callback::from_fn(ctx.mutation(), move |ctx, _, mut stack| {
                    let Some(Value::Integer(addr)) = stack.pop_front() else {
                        return Ok(CallbackReturn::Return);
                    };
                    let addr = SnesAddress::from(addr as usize);
                    let byte = {
                        let mut emu_mut = clone.borrow_mut();
                        let RSnesCore { bus, ppu, apu, .. } = emu_mut.deref_mut();
                        bus.read(addr, ppu, apu)
                    };
                    stack.replace(ctx, Value::Integer(byte as i64));
                    Ok(CallbackReturn::Return)
                }),
            );
        }
        if bus_perms.write {
            let clone = emu.clone();
            ret.set_field(
                ctx,
                "write",
                Callback::from_fn(ctx.mutation(), move |ctx, _, mut stack| {
                    let Some(Value::Integer(addr)) = stack.pop_front() else {
                        stack.replace(ctx, Value::Nil);
                        return Ok(CallbackReturn::Return);
                    };
                    let addr = SnesAddress::from(addr as usize);
                    let byte = match stack.pop_front() {
                        Some(Value::Integer(i)) => i as u8,
                        _ => 0,
                    };
                    let mut emu_mut = clone.borrow_mut();
                    let RSnesCore { bus, ppu, apu, .. } = emu_mut.deref_mut();
                    bus.write(addr, byte, ppu, apu);

                    Ok(CallbackReturn::Return)
                }),
            );
        }

        ret
    }

    fn create_fs_table<'gc>(ctx: Context<'gc>, perms: &FileSystemPermissions) -> Table<'gc> {
        let ret = Table::new(ctx.mutation());

        if !perms.write.is_none() {
            Self::add_write_perms(ctx, ret, &perms.write);
        }
        if !perms.read.is_none() {
            Self::add_read_perms(ctx, ret, &perms.read);
        }

        ret
    }

    fn add_write_perms<'gc>(
        ctx: Context<'gc>,
        tab: Table<'gc>,
        perms: &AllOr<FileWritePermissions>,
    ) {
        match perms {
            AllOr::All => todo!("handle 'all' write perms"),
            AllOr::Inner(FileWritePermissions { files }) => {
                let files_tab = Table::new(ctx.mutation());
                tab.set_field(ctx, "files", files_tab);

                for (filepath, options) in files {
                    files_tab
                        .set(
                            ctx,
                            piccolo::String::from_slice(
                                ctx.mutation(),
                                filepath.as_os_str().as_encoded_bytes(),
                            ), // TODO: windows
                            Self::create_file_write_table(ctx, filepath, *options),
                        )
                        .expect("inserting with a string key cannot fail");
                }
            }
        }
    }

    fn create_file_write_table<'gc>(
        ctx: Context<'gc>,
        filepath: &PathBuf,
        options: FileWriteOptions,
    ) -> Table<'gc> {
        let ret = Table::new(ctx.mutation());

        match OpenOptions::from(options).open(filepath) {
            Ok(file) => {
                let file = Rc::new(RefCell::new(file));

                let write_clone = file.clone();
                ret.set_field(
                    ctx,
                    "write",
                    Callback::from_fn(ctx.mutation(), move |_ctx, _, mut stack| {
                        let mut file_mut = write_clone.borrow_mut();

                        stack[..].reverse();
                        while let Some(value) = stack.pop_back() {
                            match value {
                                Value::String(s) => file_mut.write_all(s.as_bytes()).unwrap(),
                                Value::Integer(i) => {
                                    file_mut.write_all(i.to_string().as_bytes()).unwrap()
                                }
                                Value::Number(f) => {
                                    file_mut.write_all(f.to_string().as_bytes()).unwrap()
                                }
                                _ => {}
                            }
                        }
                        Ok(CallbackReturn::Return)
                    }),
                );

                if options.can_seek() {
                    Self::add_write_seek_perms(ctx, ret, &file);
                }
            }
            Err(err) => {
                ret.set_field(ctx, "error", err.kind().to_string().into_value(ctx));
            }
        }

        ret
    }

    fn add_write_seek_perms<'gc>(
        ctx: Context<'gc>,
        file_tab: Table<'gc>,
        file: &Rc<RefCell<File>>,
    ) {
        let truncate_clone = file.clone();
        file_tab.set_field(
            ctx,
            "truncate",
            Callback::from_fn(ctx.mutation(), move |ctx, _, mut stack| {
                let Some(Value::Integer(i @ 0..)) = stack.pop_front() else {
                    return Err(piccolo::Error::Lua(LuaError(
                        "invalid parameter to truncate".into_value(ctx),
                    )));
                };
                if let Err(e) = truncate_clone.borrow_mut().set_len(i as u64) {
                    stack.replace(ctx, e.to_string().into_value(ctx));
                }

                Ok(CallbackReturn::Return)
            }),
        );

        let clear_clone = file.clone();
        file_tab.set_field(
            ctx,
            "clear",
            Callback::from_fn(ctx.mutation(), move |ctx, _, mut stack| {
                let mut file = clear_clone.borrow_mut();

                let res = file.set_len(0).and_then(|()| file.seek(SeekFrom::Start(0)));
                if let Err(e) = res {
                    stack.replace(ctx, e.to_string().into_value(ctx));
                }

                Ok(CallbackReturn::Return)
            }),
        );

        let seek_clone = file.clone();
        file_tab.set_field(
            ctx,
            "seek",
            Callback::from_fn(ctx.mutation(), move |ctx, _, mut stack| {
                let seek_mode = match stack.pop_front() {
                    None | Some(Value::Nil) => SeekFrom::Current,
                    Some(Value::String(s)) if s.as_bytes() == b"cur" => SeekFrom::Current,
                    Some(Value::String(s)) if s.as_bytes() == b"set" => {
                        |i| SeekFrom::Start(i as u64)
                    }
                    Some(Value::String(s)) if s.as_bytes() == b"end" => SeekFrom::End,
                    _ => {
                        return Err(piccolo::Error::Lua(LuaError(
                            "invalid seek mode passed to seek".into_value(ctx),
                        )));
                    }
                };
                let offs = match stack.pop_front() {
                    Some(Value::Integer(i)) => i,
                    None | Some(Value::Nil) => 0,
                    _ => {
                        return Err(piccolo::Error::Lua(LuaError(
                            "invalid offset passed to seek".into_value(ctx),
                        )));
                    }
                };
                match seek_clone.borrow_mut().seek(seek_mode(offs)) {
                    Ok(new_offs) => stack.replace(ctx, (new_offs as i64).into_value(ctx)),
                    Err(e) => stack.replace(ctx, (Value::Nil, e.to_string().into_value(ctx))),
                }
                Ok(CallbackReturn::Return)
            }),
        );
    }

    fn add_read_perms<'gc>(_: Context<'gc>, _: Table<'gc>, _: &bool) {
        eprintln!("todo: handle read permissions")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::rsnes::tests::make_rsnes;
    use common::snes_addr;
    use cpu::registers::Registers;
    use piccolo::{Executor, Function, StashedExecutor, StashedTable, StashedValue, meta_ops};
    use plugins::perm_tree::RSnesPermissions;

    #[test]
    fn cpu_regs_perms() {
        let mut core = make_rsnes();
        core.cpu = CPU::new(Registers {
            A: 1000,
            D: 4000,
            DB: 8,
            E: false,
            P: 123.into(),
            PB: 100,
            PC: 8000,
            S: 1600,
            X: 4242,
            Y: 34,
        });
        let core = Rc::new(RefCell::new(core));

        let mut plugin = Plugin::load_from_raw(
            br#"return {
                permissions = {
                    internal = {
                        cpu = { "registers" }
                    }
                }
            }"#,
            None,
        )
        .unwrap();
        {
            let mut res_perms = RSnesPermissions::none();
            res_perms.internal.cpu.registers = true;
            assert_eq!(plugin.table.perms, res_perms);
        }

        let initial_globals_len = plugin.lua.enter(|ctx| ctx.globals().iter().count());

        RSnesCore::inject_into_lua(&core, &mut plugin);
        let cpu_tab = plugin.lua.enter(|ctx| {
            assert_eq!(
                ctx.globals().iter().count(),
                initial_globals_len + 1,
                "only 1 global should have been added",
            );

            let rsnes: Table = ctx.get_global("rsnes").unwrap();
            assert_eq!(rsnes.iter().count(), 1, "only cpu table should be loaded",);

            let cpu: Table = rsnes.get(ctx, "cpu").unwrap();
            assert_eq!(
                cpu.iter().count(),
                0,
                "regs table shouldn't have concrete fields"
            );

            ctx.stash(cpu)
        });

        fn get_reg_<'a, T>(
            plugin: &'a mut Plugin,
            cpu_tab: &'a StashedTable,
            key: T,
        ) -> StashedValue
        where
            T: for<'gc> IntoValue<'gc>,
        {
            enum A {
                Val(StashedValue),
                Ex(StashedExecutor),
            }
            let a = plugin.lua.enter(|ctx| {
                let cpu = ctx.fetch(cpu_tab);
                let meta_res =
                    meta_ops::index(ctx, Value::Table(cpu), IntoValue::into_value(key, ctx))
                        .unwrap();

                match meta_res {
                    meta_ops::MetaResult::Value(val) => A::Val(ctx.stash(val)),
                    meta_ops::MetaResult::Call(call) => A::Ex(ctx.stash(Executor::start(
                        ctx,
                        call.function,
                        dbg!((call.args[0], call.args[1])),
                    ))),
                }
            });

            match a {
                A::Val(val) => val,
                A::Ex(ex) => {
                    plugin.lua.finish(&ex).unwrap();
                    plugin.lua.enter(|ctx| {
                        ctx.stash(ctx.fetch(&ex).take_result::<Value>(ctx).unwrap().unwrap())
                    })
                }
            }
        }
        let mut get_reg = |key| get_reg_(&mut plugin, &cpu_tab, key);

        assert!(matches!(get_reg("A"), StashedValue::Integer(1000)));
        assert!(matches!(get_reg("D"), StashedValue::Integer(4000)));
        assert!(matches!(get_reg("DB"), StashedValue::Integer(8)));
        assert!(matches!(get_reg("E"), StashedValue::Boolean(false)));
        assert!(matches!(get_reg("P"), StashedValue::Integer(123)));
        assert!(matches!(get_reg("PB"), StashedValue::Integer(100)));
        assert!(matches!(get_reg("PC"), StashedValue::Integer(8000)));
        assert!(matches!(get_reg("S"), StashedValue::Integer(1600)));
        assert!(matches!(get_reg("X"), StashedValue::Integer(4242)));
        assert!(matches!(get_reg("Y"), StashedValue::Integer(34)));

        assert!(matches!(get_reg("bus_addr"), StashedValue::Integer(_)));
        assert!(matches!(get_reg("bus_bank"), StashedValue::Integer(_)));

        // indexing non-existant fields yields `nil`
        assert!(matches!(get_reg("foobar"), StashedValue::Nil));
        assert!(matches!(
            get_reg_(&mut plugin, &cpu_tab, 42),
            StashedValue::Nil
        ));

        // try to overwrite a register
        let ex = plugin.lua.enter(|ctx| {
            let cpu = ctx.fetch(&cpu_tab);
            meta_ops::new_index(
                ctx,
                Value::Table(cpu),
                "A".into_value(ctx),
                11111.into_value(ctx),
            )
            .unwrap()
            .map(|call| {
                ctx.stash(Executor::start(
                    ctx,
                    call.function,
                    (call.args[0], call.args[1], call.args[2]),
                ))
            })
        });
        ex.inspect(|ex| {
            plugin.lua.execute::<()>(ex).unwrap();
        });
        assert!(
            matches!(
                get_reg_(&mut plugin, &cpu_tab, "A"),
                StashedValue::Integer(1000)
            ),
            "A value should have been left unmodified"
        );
    }

    #[test]
    fn input() {
        let core = Rc::new(RefCell::new(make_rsnes()));

        let mut plugin = Plugin::load_from_raw(
            br#"return {
                permissions = {
                    internal = {
                        input = "all"
                    }
                }
            }"#,
            None,
        )
        .unwrap();
        {
            let mut res_perms = RSnesPermissions::none();
            res_perms.internal.input = true;
            assert_eq!(plugin.table.perms, res_perms);
        }

        let initial_globals_len = plugin.lua.enter(|ctx| ctx.globals().iter().count());

        RSnesCore::inject_into_lua(&core, &mut plugin);
        let (press_a, release_a) = plugin.lua.enter(|ctx| {
            assert_eq!(
                ctx.globals().iter().count(),
                initial_globals_len + 1,
                "only 1 global should have been added",
            );

            let rsnes: Table = ctx.get_global("rsnes").unwrap();
            assert_eq!(rsnes.iter().count(), 1, "only input table should be loaded",);

            let input: Table = rsnes.get(ctx, "input").unwrap();

            let press = input.get::<_, Function>(ctx, "press_a").unwrap();
            let release = input.get::<_, Function>(ctx, "release_a").unwrap();
            (ctx.stash(press), ctx.stash(release))
        });

        let mut run_lua = |f| {
            Plugin::run_lua::<_, ()>(&mut plugin.lua, f).unwrap();
        };

        core.borrow_mut().bus.io.joy1 = 0;
        assert!(core.borrow().bus.io.joy1 & (1 << 7) == 0);
        run_lua(&press_a);
        assert!(core.borrow().bus.io.joy1 & (1 << 7) != 0);
        run_lua(&press_a);
        assert!(core.borrow().bus.io.joy1 & (1 << 7) != 0);
        run_lua(&release_a);
        assert!(core.borrow().bus.io.joy1 & (1 << 7) == 0);
        run_lua(&press_a);
        assert!(core.borrow().bus.io.joy1 & (1 << 7) != 0);
        run_lua(&release_a);
        assert!(core.borrow().bus.io.joy1 & (1 << 7) == 0);
        run_lua(&release_a);
        assert!(core.borrow().bus.io.joy1 & (1 << 7) == 0);
    }

    #[test]
    fn bus_inject_count() {
        let core = Rc::new(RefCell::new(make_rsnes()));

        let mut plugin_read = Plugin::load_from_raw(
            br#"
            return {
                permissions = {
                    internal = {
                        bus = { "read" },
                    }
                }
            }
            "#,
            None,
        )
        .unwrap();
        let mut plugin_write = Plugin::load_from_raw(
            br#"
            return {
                permissions = {
                    internal = {
                        bus = { "write" },
                    }
                }
            }
            "#,
            None,
        )
        .unwrap();
        let mut plugin_readwrite = Plugin::load_from_raw(
            br#"
            return {
                permissions = {
                    internal = {
                        bus = "all",
                    }
                }
            }
            "#,
            None,
        )
        .unwrap();

        for (p, count) in [
            (&mut plugin_read, 1),
            (&mut plugin_write, 1),
            (&mut plugin_readwrite, 2),
        ] {
            let initial_globals_len = p.lua.enter(|ctx| ctx.globals().iter().count());

            RSnesCore::inject_into_lua(&core, p);
            p.lua.enter(|ctx| {
                assert_eq!(
                    ctx.globals().iter().count(),
                    initial_globals_len + 1,
                    "only 1 global should have been added",
                );

                let rsnes: Table = ctx.get_global("rsnes").unwrap();
                assert_eq!(rsnes.iter().count(), 1, "only bus table should be loaded",);

                let bus: Table = rsnes.get(ctx, "bus").unwrap();
                assert_eq!(
                    bus.iter().count(),
                    count,
                    "bus table should have {count} elements"
                );
            })
        }
    }

    #[test]
    fn bus_read_write() {
        let mut core = make_rsnes();
        let RSnesCore { bus, ppu, apu, .. } = &mut core;
        bus.write(snes_addr!(0x7F:0x1234), 0x44, ppu, apu);
        let core = Rc::new(RefCell::new(core));
        let mut plugin = Plugin::load_from_raw(
            br#"
            return {
                permissions = {
                    internal = {
                        bus = "all",
                    }
                },

                init = function()
                    read_global = rsnes.bus.read(0x7F1234)
                end,

                exit = function()
                    rsnes.bus.write(0x7F1234, 0x66)
                    rsnes.bus.write(0x7F1235, 0x35)
                end
            }
            "#,
            None,
        )
        .unwrap();

        RSnesCore::inject_into_lua(&core, &mut plugin);

        plugin.run_init().unwrap();
        plugin.lua.enter(|ctx| {
            assert!(matches!(
                ctx.globals().get_value(ctx, "read_global"),
                Value::Integer(0x44)
            ));
        });

        plugin.run_exit().unwrap();
        let mut emu_mut = core.borrow_mut();
        let RSnesCore { bus, ppu, apu, .. } = emu_mut.deref_mut();
        assert_eq!(bus.read(snes_addr!(0x7F:0x1234), ppu, apu), 0x66);
        assert_eq!(bus.read(snes_addr!(0x7F:0x1235), ppu, apu), 0x35);
    }
}
