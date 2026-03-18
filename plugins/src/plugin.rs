use std::io::Read;

use std::fs as fs;
use piccolo as picc;
use piccolo::io as p_io;

#[derive(Debug)]
pub enum PluginLoadError {
    OpenError(std::io::Error),
    BufCreationError(std::io::Error),
    ReadError(std::io::Error),
    LuaError(picc::error::ExternError),
    PluginTabError(picc::error::ExternError),
}

pub struct Plugin {
    pub lua: picc::Lua,
    pub path: std::path::PathBuf,
    pub table: PluginTable,
}

/// The data described in the lua table returned by
/// the plugin file
#[derive(Debug)]
pub struct PluginTable { }

impl<'gc> picc::FromValue<'gc> for PluginTable {
    fn from_value(_: picc::Context<'gc>, _: picc::Value<'gc>) -> Result<Self, picc::TypeError> {
        Ok(Self { })
    }
}

impl Plugin {
    pub fn load(path: &std::path::Path) -> Result<Self, PluginLoadError> {
        let file = fs::File::open(path).map_err(PluginLoadError::OpenError)?;
        let mut file = p_io::buffered_read(file).map_err(PluginLoadError::BufCreationError)?;
        let mut source = Vec::new();
        file.read_to_end(&mut source).map_err(PluginLoadError::ReadError)?;

        let mut lua = picc::Lua::full();

        // Enter a context
        let plugin = lua.try_enter(|ctx| {
            // Run the lua script in the global context
            let closure = picc::Closure::load(ctx, path.to_str(), source.as_slice())?;

            // Create an executor that will run the lua script
            let ex = picc::Executor::start(ctx, closure.into(), ());

            // Return the executor to ouside the scope. We must stash
            // it to allow it to escape the scope.
            Ok(ctx.stash(ex))
        }).map_err(PluginLoadError::LuaError)?;

        let table = lua.execute::<PluginTable>(&plugin)
            .map_err(PluginLoadError::PluginTabError)?;

        Ok(Self {
            lua,
            table,
            path: path.to_path_buf(),
        })
    }
}
