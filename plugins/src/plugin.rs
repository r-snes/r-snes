// uncomment below if code coverage is getting too low, as
// it would be "fine" to count GUI code towards coverage
// #[cfg(not(tarpaulin_include))]
pub mod gui;

use crate::perm_tree::{PermTreeNode, RSnesPermissions};
use crate::plugin::gui::PluginPermRequest;

use std::io::Read;
use std::path::{Path, PathBuf};

use common::snes_address::SnesAddress;
use piccolo as picc;
use piccolo::io as p_io;
use std::fs;

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

    /// Runtime scheduling cursor for `autoactions.on_interval`
    pub next_interval_master_cycle: Option<u64>,
}

/// The data described in the lua table returned by
/// the plugin file
#[derive(Debug, Default)]
pub struct PluginTable {
    pub perms: RSnesPermissions,

    /// Actions which can be run manually by the user
    pub actions: PluginActions,

    /// Actions which are run automatically on certain events
    pub autoactions: PluginAutoActions,

    /// The lua function that will be run when the plugin is successfully
    /// loaded, right after the user accepted the permission request
    pub init: Option<picc::StashedClosure>,

    /// The lua function that will be run when the plugin is
    /// unloaded from the emulator
    pub exit: Option<picc::StashedClosure>,
}

/// The plugin "actions" (lua functions) which can be manually
/// triggered by the user
///
/// (for now we only accept a single action)
#[derive(Debug, Default)]
pub struct PluginActions {
    /// The "default" action of the plugin, which can be called manually
    /// by the user as many times as they want
    pub default: Option<picc::StashedClosure>,
}

#[derive(Debug)]
pub struct IntervalAction {
    pub interval_seconds: f64,
    pub action: picc::StashedClosure,
}

/// Plugin "autoactions" (lua functions) which are to be run
/// automatically on certain events
#[derive(Debug, Default)]
pub struct PluginAutoActions {
    pub on_instr: Option<picc::StashedClosure>,

    /// Fires at a fixed interval of emulated time (see [`IntervalAction`]).
    pub on_interval: Option<IntervalAction>,
}

impl<'gc> picc::FromValue<'gc> for PluginTable {
    fn from_value(
        ctx: picc::Context<'gc>,
        value: picc::Value<'gc>,
    ) -> Result<Self, picc::TypeError> {
        use picc::*;

        let picc::Value::Table(tab) = value else {
            return Err(picc::TypeError {
                expected: "table",
                found: value.type_name(),
            });
        };

        let mut ret = Self::default();

        for (key, value) in tab {
            let Value::String(key) = key else {
                eprintln!("found unexpected non-string key [{}]", key.display());
                continue;
            };

            match key.as_bytes() {
                b"init" => {
                    ret.init = match value {
                        Value::Function(Function::Closure(c)) => Some(ctx.stash(c)),
                        v => {
                            return Err(picc::TypeError {
                                expected: "init function or nil",
                                found: v.type_name(),
                            });
                        }
                    }
                }
                b"exit" => {
                    ret.exit = match value {
                        Value::Function(Function::Closure(c)) => Some(ctx.stash(c)),
                        v => {
                            return Err(picc::TypeError {
                                expected: "exit function or nil",
                                found: v.type_name(),
                            });
                        }
                    }
                }
                b"permissions" => {
                    ret.perms = RSnesPermissions::from_lua(ctx, value).ok_or(picc::TypeError {
                        expected: "permission table",
                        found: "nil",
                    })?
                }

                b"actions" => ret.actions = FromValue::from_value(ctx, value)?,
                b"autoactions" => ret.autoactions = FromValue::from_value(ctx, value)?,

                _ => eprintln!(
                    "found unknow key in plugin table: [{:?}]",
                    key.debug_lossy()
                ),
            }
        }

        Ok(ret)
    }
}

impl<'gc> picc::FromValue<'gc> for PluginActions {
    fn from_value(
        ctx: picc::Context<'gc>,
        value: picc::Value<'gc>,
    ) -> Result<Self, picc::TypeError> {
        use picc::*;

        let picc::Value::Table(tab) = value else {
            return Err(picc::TypeError {
                expected: "table",
                found: value.type_name(),
            });
        };

        let mut ret = Self::default();

        for (key, value) in tab {
            let Value::String(key) = key else {
                eprintln!("found unexpected non-string key [{}]", key.display());
                continue;
            };

            match key.as_bytes() {
                b"default" => {
                    ret.default = match value {
                        Value::Function(Function::Closure(c)) => Some(ctx.stash(c)),
                        v => {
                            return Err(picc::TypeError {
                                expected: "default function or nil",
                                found: v.type_name(),
                            });
                        }
                    }
                }
                _ => eprintln!(
                    "found unknow key in plugin table: [{:?}]",
                    key.debug_lossy()
                ),
            }
        }

        Ok(ret)
    }
}

impl<'gc> picc::FromValue<'gc> for PluginAutoActions {
    fn from_value(
        ctx: picc::Context<'gc>,
        value: picc::Value<'gc>,
    ) -> Result<Self, picc::TypeError> {
        use picc::*;

        let picc::Value::Table(tab) = value else {
            return Err(picc::TypeError {
                expected: "table",
                found: value.type_name(),
            });
        };

        let mut ret = Self::default();

        for (key, value) in tab {
            let Value::String(key) = key else {
                eprintln!("found unexpected non-string key [{}]", key.display());
                continue;
            };

            match key.as_bytes() {
                b"on_instr" => {
                    ret.on_instr = match value {
                        Value::Function(Function::Closure(c)) => Some(ctx.stash(c)),
                        v => {
                            return Err(picc::TypeError {
                                expected: "on_instr function or nil",
                                found: v.type_name(),
                            });
                        }
                    }
                }
                b"on_interval" => {
                    ret.on_interval = Some(IntervalAction::from_value(ctx, value)?);
                }
                _ => eprintln!(
                    "found unknow key in plugin table: [{:?}]",
                    key.debug_lossy()
                ),
            }
        }

        Ok(ret)
    }
}

impl<'gc> picc::FromValue<'gc> for IntervalAction {
    fn from_value(
        ctx: picc::Context<'gc>,
        value: picc::Value<'gc>,
    ) -> Result<Self, picc::TypeError> {
        use picc::*;

        let Value::Table(tab) = value else {
            return Err(picc::TypeError {
                expected: "on_interval table ({ seconds = <number>, action = <function> })",
                found: value.type_name(),
            });
        };

        let mut interval_seconds = None;
        let mut action = None;

        for (key, value) in tab {
            let Value::String(key) = key else {
                eprintln!("found unexpected non-string key [{}]", key.display());
                continue;
            };

            match key.as_bytes() {
                b"seconds" => {
                    interval_seconds = match value {
                        Value::Integer(i) => Some(i as f64),
                        Value::Number(n) => Some(n),
                        v => {
                            return Err(picc::TypeError {
                                expected: "seconds: number",
                                found: v.type_name(),
                            });
                        }
                    };
                }
                b"action" => {
                    action = match value {
                        Value::Function(Function::Closure(c)) => Some(ctx.stash(c)),
                        v => {
                            return Err(picc::TypeError {
                                expected: "action: function",
                                found: v.type_name(),
                            });
                        }
                    };
                }
                _ => eprintln!(
                    "found unknow key in on_interval table: [{:?}]",
                    key.debug_lossy()
                ),
            }
        }

        Ok(Self {
            interval_seconds: interval_seconds.ok_or(picc::TypeError {
                expected: "seconds field",
                found: "nil",
            })?,
            action: action.ok_or(picc::TypeError {
                expected: "action field",
                found: "nil",
            })?,
        })
    }
}

impl Plugin {
    /// Loads a plugin from the file passed as parameter
    pub fn load_from_file(path: &Path) -> Result<Self, PluginLoadError> {
        let file = fs::File::open(path).map_err(PluginLoadError::OpenError)?;
        let mut file = p_io::buffered_read(file).map_err(PluginLoadError::BufCreationError)?;
        let mut source = Vec::new();
        file.read_to_end(&mut source)
            .map_err(PluginLoadError::ReadError)?;

        Self::load_from_raw(source.as_slice(), Some(path.to_path_buf()))
    }

    pub fn load_from_raw(
        file: &[u8],
        path: Option<std::path::PathBuf>,
    ) -> Result<Self, PluginLoadError> {
        let mut lua = picc::Lua::full();

        // Enter a context
        let plugin = lua
            .try_enter(|ctx| {
                // Run the lua script in the global context
                let closure =
                    picc::Closure::load(ctx, path.as_ref().and_then(|p| p.to_str()), file)?;

                // Create an executor that will run the lua script
                let ex = picc::Executor::start(ctx, closure.into(), ());

                // Return the executor to ouside the scope. We must stash
                // it to allow it to escape the scope.
                Ok(ctx.stash(ex))
            })
            .map_err(PluginLoadError::LuaError)?;

        let table = lua
            .execute::<PluginTable>(&plugin)
            .map_err(PluginLoadError::PluginTabError)?;

        Ok(Self {
            lua,
            table,
            path,
            next_interval_master_cycle: None,
        })
    }

    pub fn perm_request(&self) -> PluginPermRequest<'_> {
        PluginPermRequest {
            plugin: self,
            show_none: false,
            outcome: gui::PermOutcome::Pending,
        }
    }

    /// Run the exit action registered in the plugin table
    pub fn run_exit(&mut self) -> Result<(), picc::ExternError> {
        Self::run_option_lua(&mut self.lua, &self.table.exit)
    }

    /// Run the init action registered in the plugin table
    pub fn run_init(&mut self) -> Result<(), picc::ExternError> {
        Self::run_option_lua(&mut self.lua, &self.table.init)
    }

    /// Run the default action registered in the plugin table
    pub fn run_default(&mut self) -> Result<(), picc::ExternError> {
        Self::run_option_lua(&mut self.lua, &self.table.actions.default)
    }

    /// Run the default action registered in the plugin table
    pub fn run_on_instr(&mut self, opcode: u8, addr: SnesAddress) -> Result<(), picc::ExternError> {
        Self::run_option_lua_with_args(
            &mut self.lua,
            &self.table.autoactions.on_instr,
            (opcode, addr.bank, addr.addr),
        )
    }

    /// Run the on_interval action registered in the plugin table, if any,
    /// passing it the amount of *emulated* time (in seconds, since
    /// `master_cycles` == 0)
    pub fn run_on_interval(&mut self, elapsed_seconds: f64) -> Result<(), picc::ExternError> {
        let Some(interval_action) = self.table.autoactions.on_interval.as_ref() else {
            return Ok(());
        };
        Self::run_lua_with_args(&mut self.lua, &interval_action.action, (elapsed_seconds,))
    }

    /// Run an Option-wrapped stashed lua function, returning Ok(())
    /// in case there was None
    pub fn run_option_lua<F>(
        lua: &mut picc::Lua,
        stashed: &Option<F>,
    ) -> Result<(), picc::ExternError>
    where
        F: for<'gc> picc::stash::Fetchable<Fetched<'gc>: Into<picc::Function<'gc>>>,
    {
        let Some(stashed) = stashed.as_ref() else {
            return Ok(());
        };
        Self::run_lua(lua, stashed)
    }

    /// Run an Option-wrapped stashed lua function, returning Ok(())
    /// in case there was None
    pub fn run_option_lua_with_args<F, A>(
        lua: &mut picc::Lua,
        stashed: &Option<F>,
        args: A,
    ) -> Result<(), picc::ExternError>
    where
        F: for<'gc> picc::stash::Fetchable<Fetched<'gc>: Into<picc::Function<'gc>>>,
        A: for<'gc> picc::IntoMultiValue<'gc>,
    {
        let Some(stashed) = stashed.as_ref() else {
            return Ok(());
        };
        Self::run_lua_with_args(lua, stashed, args)
    }

    pub fn run_lua<F, R>(lua: &mut picc::Lua, stashed: &F) -> Result<R, picc::ExternError>
    where
        F: for<'gc> picc::stash::Fetchable<Fetched<'gc>: Into<picc::Function<'gc>>>,
        R: for<'gc> picc::FromMultiValue<'gc>,
    {
        Self::run_lua_with_args(lua, stashed, ())
    }

    /// Runs a stashed lua function in the given lua context
    pub fn run_lua_with_args<F, R, A>(
        lua: &mut picc::Lua,
        stashed: &F,
        args: A,
    ) -> Result<R, picc::ExternError>
    where
        F: for<'gc> picc::stash::Fetchable<Fetched<'gc>: Into<picc::Function<'gc>>>,
        R: for<'gc> picc::FromMultiValue<'gc>,
        A: for<'gc> picc::IntoMultiValue<'gc>,
    {
        let ex = lua.enter(|ctx| {
            let func = ctx.fetch(stashed).into();
            let ex = piccolo::Executor::start(ctx, func, args);

            ctx.stash(ex)
        });

        lua.execute(&ex)
    }
}

impl std::fmt::Display for PluginLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PluginLoadError::OpenError(e) => write!(f, "failed to open plugin file: {e}"),
            PluginLoadError::BufCreationError(e) => write!(f, "failed to create read buffer: {e}"),
            PluginLoadError::ReadError(e) => write!(f, "failed to read plugin file: {e}"),
            PluginLoadError::LuaError(e) => write!(f, "lua error while loading plugin: {e}"),
            PluginLoadError::PluginTabError(e) => write!(f, "error reading plugin table: {e}"),
        }
    }
}

impl std::error::Error for PluginLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PluginLoadError::OpenError(e)
            | PluginLoadError::BufCreationError(e)
            | PluginLoadError::ReadError(e) => Some(e),
            // ExternError isn't guaranteed to impl Error; keep it in the Display
            // message above rather than exposing it as a source.
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use common::snes_addr;

    use crate::permission::Permission;

    use super::*;

    #[test]
    #[cfg(target_family = "unix")]
    fn load_from_file() {
        let plugin = Plugin::load_from_file(Path::new("/dev/null"));

        assert!(
            matches!(plugin, Err(PluginLoadError::PluginTabError(_))),
            "loading from empty file should fail when reading the plugin tab",
        );
    }

    #[test]
    fn load_empty_plugin() {
        let plugin = Plugin::load_from_raw(b"return { permissions = {}}", None).unwrap();

        assert!(
            plugin.table.perms.is_none(),
            "empty perm table gives 0 permission"
        );
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

    #[test]
    fn basic_plugin_increment() {
        use piccolo::Value;

        let mut plugin = Plugin::load_from_raw(
            br#"
            return {
                permissions = "all",

                init = function()
                    i = 10
                   done = false
                end,

                exit = function()
                    done = true
                end,

                actions = {
                    default = function()
                        i = i + 1
                    end,
                },
            }"#,
            None,
        )
        .unwrap();

        plugin.run_init().unwrap();

        plugin.run_default().unwrap();
        plugin.run_default().unwrap();
        plugin.run_default().unwrap();

        // running on_instr should be a no-op
        plugin.run_on_instr(0, snes_addr!(0:0)).unwrap();

        plugin.lua.enter(|ctx| {
            assert!(matches!(ctx.get_global_value("i"), Value::Integer(13)));
            assert!(matches!(
                ctx.get_global_value("done"),
                Value::Boolean(false)
            ));
        });

        // after running exit we should see `done` set to true
        plugin.run_exit().unwrap();
        plugin.lua.enter(|ctx| {
            assert!(matches!(ctx.get_global_value("done"), Value::Boolean(true)));
        });
    }

    #[test]
    fn basic_autoactions() {
        use piccolo::Value;

        let mut plugin = Plugin::load_from_raw(
            br#"
            return {
                permissions = "all",

                init = function()
                   xce_counter = 0
                end,

                autoactions = {
                    on_instr = function(opcode, pb, pc)
                        if opcode == 0xFB then
                            xce_counter = xce_counter + 1
                        end
                    end,
                },
            }"#,
            None,
        )
        .unwrap();

        plugin.run_init().unwrap();

        plugin.run_on_instr(0xfb, snes_addr!(0:0)).unwrap();
        plugin.run_on_instr(0x00, snes_addr!(0:0)).unwrap();
        plugin.run_on_instr(0x30, snes_addr!(0:0)).unwrap();
        plugin.run_on_instr(0xfb, snes_addr!(0:0)).unwrap();
        plugin.run_on_instr(0xff, snes_addr!(0:0)).unwrap();

        plugin.lua.enter(|ctx| {
            assert!(matches!(
                ctx.get_global_value("xce_counter"),
                Value::Integer(2)
            ));
        });

        // run exit should be a no-op as there's none
        plugin.run_exit().unwrap();
    }

    #[test]
    fn on_interval_parses_and_runs() {
        use piccolo::Value;

        let mut plugin = Plugin::load_from_raw(
            br#"
            return {
                permissions = "all",

                init = function()
                    interval_count = 0
                    last_elapsed = -1
                end,

                autoactions = {
                    on_interval = {
                        seconds = 5,
                        action = function(elapsed_seconds)
                            interval_count = interval_count + 1
                            last_elapsed = elapsed_seconds
                        end,
                    },
                },
            }"#,
            None,
        )
        .unwrap();

        assert!(plugin.next_interval_master_cycle.is_none());
        assert_eq!(
            plugin
                .table
                .autoactions
                .on_interval
                .as_ref()
                .map(|t| t.interval_seconds),
            Some(5.0)
        );

        plugin.run_init().unwrap();

        plugin.run_on_interval(5.0).unwrap();
        plugin.run_on_interval(10.0).unwrap();

        plugin.lua.enter(|ctx| {
            assert!(matches!(
                ctx.get_global_value("interval_count"),
                Value::Integer(2)
            ));
            assert!(matches!(
                ctx.get_global_value("last_elapsed"),
                Value::Number(n) if n == 10.0
            ));
        });
    }

    #[test]
    fn on_interval_missing_fields_error() {
        let missing_seconds = Plugin::load_from_raw(
            br#"return {
                permissions = "all",
                autoactions = { on_interval = { action = function() end } },
            }"#,
            None,
        );
        assert!(
            matches!(missing_seconds, Err(PluginLoadError::PluginTabError(_))),
            "on_interval without `seconds` should fail to load",
        );

        let missing_action = Plugin::load_from_raw(
            br#"return {
                permissions = "all",
                autoactions = { on_interval = { seconds = 5 } },
            }"#,
            None,
        );
        assert!(
            matches!(missing_action, Err(PluginLoadError::PluginTabError(_))),
            "on_interval without `action` should fail to load",
        );
    }
}
