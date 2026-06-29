use super::RSnes;

use cpu::cpu::CPU;
use piccolo::Callback;
use piccolo::CallbackReturn;
use piccolo::Context;
use piccolo::IntoMultiValue;
use piccolo::IntoValue;
use piccolo::Table;
use piccolo::Value;
use piccolo::error::LuaError;
use plugins::perm_tree::FileSystemPermissions;
use plugins::perm_tree::FileWritePermissions;
use plugins::perm_tree::filesystem::FileWriteOptions;
use plugins::perm_tree::filesystem::OverwriteMode;
use plugins::permission::Permission;
use plugins::permission::helpers::AllOr;
use plugins::plugin::Plugin;
use std::cell::RefCell;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

impl RSnes {
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
                let (_, key): (Table, piccolo::String) = stack.consume(ctx)?;
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
                    Some(Value::String(s)) if s.as_bytes() == b"set" => |i| SeekFrom::Start(i as u64),
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
