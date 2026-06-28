use super::RSnes;

use cpu::cpu::CPU;
use piccolo::Callback;
use piccolo::Context;
use piccolo::Table;
use piccolo::Value;
use plugins::perm_tree::FileSystemPermissions;
use plugins::perm_tree::FileWritePermissions;
use plugins::perm_tree::filesystem::FileWriteOptions;
use plugins::perm_tree::filesystem::OverwriteMode;
use plugins::permission::Permission;
use plugins::permission::helpers::AllOr;
use plugins::plugin::Plugin;
use std::cell::RefCell;
use std::fs::OpenOptions;
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
                        Ok(piccolo::CallbackReturn::Return)
                    }),
                );
            }
            Err(err) => {
                ret.set_field(
                    ctx,
                    "error",
                    Value::String(piccolo::String::from_buffer(
                        ctx.mutation(),
                        err.kind().to_string().into_boxed_str().into_boxed_bytes(),
                    )),
                );
            }
        }

        ret
    }

    fn add_read_perms<'gc>(_: Context<'gc>, _: Table<'gc>, _: &bool) {
        eprintln!("todo: handle read permissions")
    }
}
