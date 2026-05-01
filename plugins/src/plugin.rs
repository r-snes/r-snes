use crate::perm_tree::{
    RSnesPermissions,
    PermTreeNode,
};

use std::io::Read;
use std::path::{Path, PathBuf};

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
    pub path: Option<PathBuf>,
    pub table: PluginTable,
}

/// The data described in the lua table returned by
/// the plugin file
#[derive(Debug)]
pub struct PluginTable {
    pub perms: RSnesPermissions,

    /// The lua function that will be run when the plugin is successully
    /// loaded, right after the user accepted the permission request
    pub init: Option<picc::StashedClosure>,
}

impl<'gc> picc::FromValue<'gc> for PluginTable {
    fn from_value(ctx: picc::Context<'gc>, value: picc::Value<'gc>) -> Result<Self, picc::TypeError> {
        use picc::*;

        let picc::Value::Table(tab) = value else {
            return Err(picc::TypeError {
                expected: "table",
                found: value.type_name()
            });
        };

        let perms = RSnesPermissions::from_lua(ctx, tab.get_value(ctx, "permissions"))
            .ok_or(picc::TypeError {
                expected: "permission table",
                found: "nil",
            })?;
        tab.set_field(ctx, "permissions", picc::Value::Nil);

        let init = match tab.get_value(ctx, "init") {
            Value::Nil => None,
            Value::Function(Function::Closure(c)) => Some(ctx.stash(c)),
            v => return Err(picc::TypeError {
                expected: "init function or nil",
                found: v.type_name(),
            }),
        };
        tab.set_field(ctx, "init", picc::Value::Nil);

        // we have removed all expected fields, if any remains, warn for it
        for (key, value) in tab {
            eprintln!("found unused KV pair: ({:?}, {:?})", key, value);
        }

        Ok(Self { perms, init })
    }
}

impl Plugin {
    /// Loads a plugin from the file passed as parameter
    pub fn load_from_file(path: &Path) -> Result<Self, PluginLoadError> {
        let file = fs::File::open(path).map_err(PluginLoadError::OpenError)?;
        let mut file = p_io::buffered_read(file).map_err(PluginLoadError::BufCreationError)?;
        let mut source = Vec::new();
        file.read_to_end(&mut source).map_err(PluginLoadError::ReadError)?;

        Self::load_from_raw(source.as_slice(), Some(path.to_path_buf()))
    }

    pub fn load_from_raw(file: &[u8], path: Option<std::path::PathBuf>) -> Result<Self, PluginLoadError> {
        let mut lua = picc::Lua::full();

        // Enter a context
        let plugin = lua.try_enter(|ctx| {
            // Run the lua script in the global context
            let closure = picc::Closure::load(ctx, path.as_ref().map(|p| p.to_str()).flatten(), file)?;

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
            path,
        })
    }

    pub fn perm_request<'a>(&'a self) -> PluginPermRequest<'a> {
        PluginPermRequest {
            plugin: self,
            allow_all: false,
        }
    }

    pub fn run_init(&mut self) -> Result<(), picc::ExternError> {
        if let Some(ref init) = self.table.init {
            Self::run_lua::<_, ()>(&mut self.lua, init)?;
        }
        Ok(())
    }

    pub fn run_lua<F, R>(lua: &mut picc::Lua, stashed: &F) -> Result<R, picc::ExternError>
    where
        F: for<'gc> picc::stash::Fetchable<Fetched<'gc>: Into<picc::Function<'gc>>>,
        R: for<'gc> picc::FromMultiValue<'gc>
    {
        let ex = lua.enter(|ctx| {
            let func = ctx.fetch(stashed);
            let ex = piccolo::Executor::start(ctx, func.into(), ());

            ctx.stash(ex)
        });

        lua.execute(&ex)
    }
}

pub struct PluginPermRequest<'a> {
    pub plugin: &'a Plugin,
    pub allow_all: bool,
}

impl<'a> PluginPermRequest<'a> {
    pub fn show_gui(&mut self, ui: &mut egui::Ui) {
        let close = |ui: &mut egui::Ui| {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        };

        ui.label("This is still very much a work in progress");

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("Grant requested permissions").clicked() {
                self.allow_all = true;
                close(ui);
            }
            if ui.button("Cancel plugin execution").clicked() {
                self.allow_all = false;
                close(ui);
            }
        });

        ui.collapsing("we can even have collapsing content", |ui| {
            ui.label("peekaboo!");
        });
    }
}

#[cfg(test)]
mod tests {
    use piccolo::ExternError;

    use super::*;

    #[test]
    #[cfg(target_family = "unix")]
    fn load_from_file() {
        let plugin = Plugin::load_from_file(&Path::new("/dev/null"));

        assert!(
            matches!(plugin, Err(PluginLoadError::PluginTabError(_))),
            "loading from empty file should fail when reading the plugin tab",
        );
    }

    #[test]
    fn load_empty_plugin() {
        let plugin = Plugin::load_from_raw(b"return { permissions = {}}", None).unwrap();

        // nothing else to assert yet, we just expect the plugin to load properly
    }

    #[test]
    fn invalid_plugin_table() {
        let plugin = Plugin::load_from_raw(b"return 42", None);

        assert!(
            matches!(plugin, Err(PluginLoadError::PluginTabError(_))),
            "load should fail: got a int instead of a table",
        );
    }

    #[test]
    fn invalid_perm_table() {
        let plugin = Plugin::load_from_raw(b"return { permissions = 42 }", None);

        assert!(
            matches!(plugin, Err(PluginLoadError::PluginTabError(_))),
            "load should fail: got a int instead of a perm table",
        );
    }
}
