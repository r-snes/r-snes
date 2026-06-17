use super::RSnes;

use cpu::cpu::CPU;
use piccolo::Callback;
use piccolo::Context;
use piccolo::Table;
use piccolo::Value;
use plugins::plugin::Plugin;
use std::cell::RefCell;
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
}
