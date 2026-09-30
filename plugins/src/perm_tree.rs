//! R-SNES permission tree:
//!
//! Permission tree nodes can be constructed from lua values
//! read from plugin files.
//!
//! Full tree:
//! ```txt
//! + internal // access stuff within the emulator
//! | + control // control the emulator, not the components
//! | | + dialog // allows the plugin to show dialog windows
//! | | ` pause // pause/resume game
//! | |
//! | + cpu
//! | | ` registers
//! | |
//! | + ppu // access framebuffer, loaded objects, etc.
//! | | ` display // draw to the framebuffer
//! | |
//! | ` bus // interact with memory
//! |   + read
//! |   ` write
//! |
//! ` external // access to the host system
//!   + filesystem
//!   | + read_file
//!   | ` write_file
//!   |
//!   ` http
//! ```

pub mod filesystem;

use std::{collections::HashMap, path::PathBuf};

use crate::permission::{Permission, helpers::AllOr};
use derive_aliases::derive;

use piccolo::{Context, Value};

/// Trait implemented by all nodes (including non-leaves) of
/// the permission tree
pub trait PermTreeNode: Sized {
    /// Attempt to build this permission tree node from a lua value
    fn from_lua<'gc>(ctx: Context<'gc>, value: Value<'gc>) -> Option<Self>;
}

/// Helper trait to implement [`PermTreeNode`] for leaf nodes of the tree
///
/// Automatically lua string values of `"all"` and `"none"` redirecting
/// to [`Permission::all()`] and [`Permission::none()`] as all nodes of
/// the tree should do.
/// By default, lua values other than these two strings will fail to
/// build the permission node, additional cases can be added by overriding
/// the default implementation of [`from_lua_leaf`].
trait PermTreeLeafNode: Permission {
    /// Fallback method for when the leaf node is constructed with something
    /// other than `"all"` and `"none"`.
    fn from_lua_leaf<'gc>(_: Context<'gc>, _: Value<'gc>) -> Option<Self> {
        None
    }
}

impl<T: PermTreeLeafNode> PermTreeNode for T {
    fn from_lua<'gc>(ctx: Context<'gc>, value: Value<'gc>) -> Option<Self> {
        match value {
            Value::String(s) if s.as_bytes() == b"all" => Some(Self::all()),
            Value::String(s) if s.as_bytes() == b"none" => Some(Self::none()),
            _ => <Self as PermTreeLeafNode>::from_lua_leaf(ctx, value),
        }
    }
}

impl PermTreeLeafNode for bool {}

/// Helper trait to implement [`PermTreeNode`] for types wrapped
/// in a [`AllOr`], respecting the "all" and "none" special values,
/// and wrapping other values in [`AllOr::Inner`] properly
trait PermTreeFromAllOr: Sized + Eq + PartialEq + PartialOrd + Default {
    fn from_lua_inner<'gc>(_: Context<'gc>, _: Value<'gc>) -> Option<Self> {
        None
    }
}

impl<T: PermTreeFromAllOr> PermTreeNode for AllOr<T> {
    fn from_lua<'gc>(ctx: Context<'gc>, value: Value<'gc>) -> Option<Self> {
        match value {
            Value::String(s) if s.as_bytes() == b"all" => Some(Permission::all()),
            Value::String(s) if s.as_bytes() == b"none" => Some(Permission::none()),
            _ => <T as PermTreeFromAllOr>::from_lua_inner(ctx, value).map(AllOr::Inner),
        }
    }
}

/// Root node of the permission tree
#[derive(..PermTree)]
pub struct RSnesPermissions {
    /// access things in the emulator program
    pub internal: InternalPermissions,
    /// access things in the host machine outside the emulator
    pub external: ExternalPermissions,
}

/// Internal permissions: give access to emulated hardware
/// and control the emulator program
#[derive(..PermTree)]
pub struct InternalPermissions {
    /// control how the emulator is running
    pub control: ControlPermissions,
    /// access to `rsnes.cpu`
    pub cpu: CpuPermissions,
    /// access to `rsnes.ppu`
    pub ppu: PpuPermissions,
    /// access to `rsnes.bus`
    pub bus: BusPermissions,
    /// access to `rsnes.input`
    pub input: bool,
}

/// Control permissions: allows control of the emulator itself
#[derive(..PermTree)]
pub struct ControlPermissions {
    /// allows the plugin to show dialog windows over the
    /// game screen
    pub dialog: bool,
    /// allow the plugin to pause/resume game execution
    pub pause: bool,
}

/// Permissions to access the CPU
#[derive(..PermTree)]
pub struct CpuPermissions {
    /// gives access to all registers and address bus
    /// in `rsnes.cpu`
    pub registers: bool,
}

/// Permissions to access the PPU
#[derive(..PermTree)]
pub struct PpuPermissions {
    /// access to `ppu.write_cgram`
    pub display: bool,
}

/// Permissions to read/write the global address space
#[derive(..PermTree)]
pub struct BusPermissions {
    /// access to `rsnes.bus.read`
    pub read: bool,
    /// access to `rsnes.bus.write`
    pub write: bool,
}

/// All external permissions: access to the host machine
#[derive(..PermTree)]
pub struct ExternalPermissions {
    /// access to the host filesystem
    pub filesystem: FileSystemPermissions,
    /// access to http/https requests
    pub http: bool,
}

/// All filesystem access
#[derive(..PermTree)]
pub struct FileSystemPermissions {
    /// access to individual files
    pub files: AllOr<FilePermissions>,
}

/// Access to individual files
#[derive(Default, PartialEq, Eq, Debug)]
pub struct FilePermissions {
    /// Files for which read/write permissions are requested
    pub files: HashMap<PathBuf, self::filesystem::FileReadWriteOptions>,
}

#[cfg(test)]
mod test {
    use super::*;
    use piccolo::{Closure, Executor, Lua, Value};

    pub(super) fn build_from_lua<T, F>(lua_str: &str, f: F) -> T
    where
        F: for<'gc> FnOnce(Context<'gc>, Value<'gc>) -> T,
    {
        let mut lua = Lua::empty();

        let ex = lua
            .try_enter(|ctx| {
                let closure = Closure::load(ctx, None, format!("return {}", lua_str).as_bytes())?;
                let ex = Executor::start(ctx, closure.into(), ());

                Ok(ctx.stash(ex))
            })
            .expect("a valid executor");

        lua.finish(&ex).expect("successful execution");
        lua.enter(|ctx| {
            let ex = ctx.fetch(&ex);
            let val: Value = ex
                .take_result(ctx)
                .expect("correct executor mode")
                .expect("no lua error");

            f(ctx, val)
        })
    }

    fn build_perm_tree(lua_str: &str) -> RSnesPermissions {
        build_from_lua(lua_str, RSnesPermissions::from_lua).expect("valid construction")
    }

    #[test]
    fn from_lua_all() {
        let tree = build_perm_tree(r#""all""#);

        assert!(tree.is_all());
    }

    #[test]
    fn detailed_tree_construction() {
        let tree = build_perm_tree(
            r#"{
                internal = {
                    control = "all",
                    bus = { "read" },
                    "cpu",
                },
                external = {
                    filesystem = {
                        files = "all",
                    },
                },
            }"#,
        );

        let expected_tree = RSnesPermissions {
            internal: InternalPermissions {
                control: ControlPermissions::all(),
                bus: BusPermissions {
                    read: true,
                    write: false,
                },
                cpu: CpuPermissions::all(),
                ..Permission::none()
            },
            external: ExternalPermissions {
                filesystem: FileSystemPermissions {
                    files: Permission::all(),
                },
                ..Permission::none()
            },
        };

        assert!(tree == expected_tree);
    }
}
