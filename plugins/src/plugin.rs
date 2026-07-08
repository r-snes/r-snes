use crate::perm_tree::filesystem::FileWriteOptions;
use crate::perm_tree::{PermTreeNode, RSnesPermissions};
use crate::permission::Permission;
use crate::permission::helpers::AllOr;

use std::io::Read;
use std::path::{Path, PathBuf};

use common::snes_address::SnesAddress;
use egui::text::LayoutJob;
use egui::{CollapsingHeader, RichText, Style, TextFormat, WidgetText};
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

/// Plugin "autoactions" (lua functions) which are to be run
/// automatically on certain events
#[derive(Debug, Default)]
pub struct PluginAutoActions {
    pub on_instr: Option<picc::StashedClosure>,
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

        let mut ret = Self::default();

        for (key, value) in tab {
            let Value::String(key) = key else {
                eprintln!("found unexpected non-string key [{}]", key.display());
                continue;
            };

            match key.as_bytes() {
                b"init" => ret.init = match value {
                    Value::Function(Function::Closure(c)) => Some(ctx.stash(c)),
                    v => return Err(picc::TypeError {
                        expected: "init function or nil",
                        found: v.type_name(),
                    }),
                },
                b"exit" => ret.exit = match value {
                    Value::Function(Function::Closure(c)) => Some(ctx.stash(c)),
                    v => return Err(picc::TypeError {
                        expected: "exit function or nil",
                        found: v.type_name(),
                    }),
                },
                b"permissions" => ret.perms = RSnesPermissions::from_lua(ctx, value)
                    .ok_or(picc::TypeError {
                        expected: "permission table",
                        found: "nil",
                    })?,

                b"actions" => ret.actions = FromValue::from_value(ctx, value)?,
                b"autoactions" => ret.autoactions = FromValue::from_value(ctx, value)?,

                _ => eprintln!("found unknow key in plugin table: [{:?}]", key.debug_lossy()),
            }
        }

        Ok(ret)
    }
}

impl<'gc> picc::FromValue<'gc> for PluginActions {
    fn from_value(ctx: picc::Context<'gc>, value: picc::Value<'gc>) -> Result<Self, picc::TypeError> {
        use picc::*;

        let picc::Value::Table(tab) = value else {
            return Err(picc::TypeError {
                expected: "table",
                found: value.type_name()
            });
        };

        let mut ret = Self::default();

        for (key, value) in tab {
            let Value::String(key) = key else {
                eprintln!("found unexpected non-string key [{}]", key.display());
                continue;
            };

            match key.as_bytes() {
                b"default" => ret.default = match value {
                    Value::Function(Function::Closure(c)) => Some(ctx.stash(c)),
                    v => return Err(picc::TypeError {
                        expected: "default function or nil",
                        found: v.type_name(),
                    }),
                },
                _ => eprintln!("found unknow key in plugin table: [{:?}]", key.debug_lossy()),
            }
        }

        Ok(ret)
    }
}

impl<'gc> picc::FromValue<'gc> for PluginAutoActions {
    fn from_value(ctx: picc::Context<'gc>, value: picc::Value<'gc>) -> Result<Self, picc::TypeError> {
        use picc::*;

        let picc::Value::Table(tab) = value else {
            return Err(picc::TypeError {
                expected: "table",
                found: value.type_name()
            });
        };

        let mut ret = Self::default();

        for (key, value) in tab {
            let Value::String(key) = key else {
                eprintln!("found unexpected non-string key [{}]", key.display());
                continue;
            };

            match key.as_bytes() {
                b"on_instr" => ret.on_instr = match value {
                    Value::Function(Function::Closure(c)) => Some(ctx.stash(c)),
                    v => return Err(picc::TypeError {
                        expected: "on_instr function or nil",
                        found: v.type_name(),
                    }),
                },
                _ => eprintln!("found unknow key in plugin table: [{:?}]", key.debug_lossy()),
            }
        }

        Ok(ret)
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
            show_none: false,
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

    /// Run an Option-wrapped stashed lua function, returning Ok(())
    /// in case there was None
    pub fn run_option_lua<F>(
        lua: &mut picc::Lua,
        stashed: &Option<F>
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
        args: A
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

    pub fn run_lua<F, R>(
        lua: &mut picc::Lua,
        stashed: &F
    ) -> Result<R, picc::ExternError>
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
        args: A
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

pub struct PluginPermRequest<'a> {
    pub plugin: &'a Plugin,
    pub allow_all: bool,

    pub show_none: bool,
}

impl<'a> PluginPermRequest<'a> {
    fn all() -> RichText {
        RichText::new("all").strong()
    }
    fn none() -> RichText {
        RichText::new("none").italics()
    }
    fn perm_label(perm: &impl Permission, name: &str) -> impl Into<WidgetText> {
        let mut job = LayoutJob::default();
        job.append(name, 0.0, Default::default());
        if perm.is_none() {
            job.append(
                "none",
                12.0,
                TextFormat {
                    italics: true,
                    ..Default::default()
                },
            );
        }
        if perm.is_all() {
            job.append(
                "all",
                12.0,
                TextFormat {
                    color: Style::default().visuals.strong_text_color(),
                    italics: true,
                    ..Default::default()
                },
            );
        }

        job
    }
    fn perm_collapsing_header(
        perm: &impl Permission,
        name: &str,
    ) -> CollapsingHeader {
        CollapsingHeader::new(Self::perm_label(perm, name))
    }
    fn force_show_perm_collapsible<T: Permission>(
        &self,
        ui: &mut egui::Ui,
        perm: &T,
        label: &str,
        draw_content: impl FnOnce(&mut egui::Ui, &T),
    ) {
        self.show_perm(ui, perm, |ui, perm| {
            Self::perm_collapsing_header(perm, label)
                .default_open(true)
                .show(ui, |ui| draw_content(ui, perm));
        });
    }
    fn show_perm_collapsible<T: Permission>(
        &self,
        ui: &mut egui::Ui,
        perm: &T,
        label: &str,
        draw_content: impl FnOnce(&mut egui::Ui, &T),
    ) {
        self.show_perm(ui, perm, |ui, perm| {
            Self::perm_collapsing_header(perm, label)
                .default_open(!perm.is_all() && !perm.is_none())
                .show(ui, |ui| draw_content(ui, perm));
        });
    }
    fn show_perm<T: Permission>(
        &self,
        ui: &mut egui::Ui,
        perm: &T,
        add_content: impl FnOnce(&mut egui::Ui, &T),
    ) {
        if !perm.is_none() || self.show_none {
            add_content(ui, perm);
        }
    }
    fn show_perm_bool(&self, ui: &mut egui::Ui, perm: bool, label: &str) {
        self.show_perm(ui, &perm, |ui, perm| {
            ui.label(Self::perm_label(perm, label));
        });
    }
    pub fn show_gui(&mut self, ui: &mut egui::Ui) {
        let close = |ui: &mut egui::Ui| {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        };

        ui.separator();

        ui.label(RichText::new("Requested permissions:").heading());
        let perms = &self.plugin.table.perms;
        if perms.is_none() {
            ui.indent((), |ui| ui.label(Self::none()));
        } else {
            self.force_show_perm_collapsible(ui, &perms.internal, "Internal", |ui, internal| {
                self.show_perm_collapsible(ui, &internal.cpu, "CPU", |ui, cpu| {
                    self.show_perm_bool(ui, cpu.registers, "Registers");
                });
                self.show_perm_collapsible(ui, &internal.bus, "Bus", |ui, bus| {
                    self.show_perm_bool(ui, bus.read, "Read");
                    self.show_perm_bool(ui, bus.write, "Write");
                });
                self.show_perm_collapsible(ui, &internal.ppu, "PPU", |ui, ppu| {
                    self.show_perm_bool(ui, ppu.display, "Display");
                });
                self.show_perm_bool(ui, internal.input, "Input");
                self.show_perm_collapsible(ui, &internal.control, "Control", |ui, control| {
                    self.show_perm_bool(ui, control.pause, "Pause");
                    self.show_perm_bool(ui, control.dialog, "Dialog");
                });
            });

            self.force_show_perm_collapsible(ui, &perms.external, "External", |ui, external| {
                self.show_perm_bool(ui, external.http, "HTTP");
                self.show_perm_collapsible(ui, &external.filesystem, "Filesystem", |ui, fs| {
                    self.show_perm_bool(ui, fs.read, "Read");
                    self.show_perm_collapsible(ui, &fs.write, "Write", |ui, write| match write {
                        AllOr::All => {
                            ui.label(RichText::new("all").strong());
                        }
                        AllOr::Inner(files) => {
                            for (file, options) in files.files.iter() {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 0.;
                                    ui.label(RichText::new(format!("{file:?}")).monospace());
                                    let label = match options {
                                        FileWriteOptions::NewOnly => "NewOnly",
                                        FileWriteOptions::CanOverwrite { create, mode } => {
                                            &format!(
                                                ": {}{mode:?}",
                                                if *create { "Create + " } else { "" }
                                            )
                                        }
                                    };
                                    ui.label(label);
                                });
                            }
                        }
                    })
                });
            });
        }

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_none, "Show 'none' fields");
        });

        ui.add_space(16.0);

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
        let plugin = Plugin::load_from_file(&Path::new("/dev/null"));

        assert!(
            matches!(plugin, Err(PluginLoadError::PluginTabError(_))),
            "loading from empty file should fail when reading the plugin tab",
        );
    }

    #[test]
    fn load_empty_plugin() {
        let plugin = Plugin::load_from_raw(b"return { permissions = {}}", None).unwrap();

        assert!(plugin.table.perms.is_none(), "empty perm table gives 0 permission");
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
        ).unwrap();

        plugin.run_init().unwrap();

        plugin.run_default().unwrap();
        plugin.run_default().unwrap();
        plugin.run_default().unwrap();

        // running on_instr should be a no-op
        plugin.run_on_instr(0, snes_addr!(0:0)).unwrap();

        plugin.lua.enter(|ctx| {
            assert!(matches!(ctx.get_global_value("i"), Value::Integer(13)));
            assert!(matches!(ctx.get_global_value("done"), Value::Boolean(false)));
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
        ).unwrap();

        plugin.run_init().unwrap();

        plugin.run_on_instr(0xfb, snes_addr!(0:0)).unwrap();
        plugin.run_on_instr(0x00, snes_addr!(0:0)).unwrap();
        plugin.run_on_instr(0x30, snes_addr!(0:0)).unwrap();
        plugin.run_on_instr(0xfb, snes_addr!(0:0)).unwrap();
        plugin.run_on_instr(0xff, snes_addr!(0:0)).unwrap();

        plugin.lua.enter(|ctx| {
            assert!(matches!(ctx.get_global_value("xce_counter"), Value::Integer(2)));
        });

        // run exit should be a no-op as there's none
        plugin.run_exit().unwrap();
    }
}
