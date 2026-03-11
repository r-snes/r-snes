use std::fs::File;
use std::io::Read;

use piccolo::{
    io::buffered_read,
    Closure,
    Executor,
    Lua,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the Lua file
    let filename = "./plugin.lua";
    let mut file = buffered_read(File::open(filename)?)?;
    let mut source = Vec::new();
    file.read_to_end(&mut source)?;

    // Instantiate the Lua instance
    let mut lua = Lua::full();

    // Enter a context
    let plugin = lua.try_enter(|ctx| {
        // Run the lua script in the global context
        let closure = Closure::load(ctx, Some(filename), source.as_slice())?;

        // Create an executor that will run the lua script
        let ex = Executor::start(ctx, closure.into(), ());

        // Return the executor to ouside the scope. We must stash it to allow it to escape the scope.
        Ok(ctx.stash(ex))
    })?;

    // Run the top-level code of the plugin to get the
    // plugin table.
    // Note that this will panic at runtime: the plugin
    // table is not convertible to a String, we will implement
    // conversion later.
    let plugin_table = lua.execute::<String>(&plugin)?;

    dbg!(plugin_table);

    Ok(())
}
