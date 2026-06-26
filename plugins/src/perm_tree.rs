pub mod filesystem;

use std::{collections::HashMap, path::PathBuf};

use crate::permission::{Permission, helpers::AllOr};
use derive_aliases::derive;

use piccolo::{Context, Value};

/// + internal // access stuff within the emulator
/// | + control // control the emulator, not the components
/// | | + dialog // allows the plugin to show dialog windows
/// | | ` pause // pause/resume game
/// | |
/// | + cpu
/// | | ` registers
/// | |
/// | + ppu // access framebuffer, loaded objects, etc.
/// | | ` display // draw to the framebuffer
/// | |
/// | ` bus // interact with memory
/// |   + read
/// |   ` write
/// |
/// ` external // access to the host system
///   + filesystem
///   | + read_file
///   | ` write_file
///   |
///   ` http

/// Permission tree nodes can be constructed from lua values
/// read from plugin files.
pub trait PermTreeNode: Sized {
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

#[derive(..PermTree)]
pub struct RSnesPermissions {
    pub internal: InternalPermissions,
    pub external: ExternalPermissions,
}

#[derive(..PermTree)]
pub struct InternalPermissions {
    pub control: ControlPermissions,
    pub cpu: CpuPermissions,
    pub ppu: PpuPermissions,
    pub bus: BusPermissions,
    pub input: bool,
}

#[derive(..PermTree)]
pub struct ControlPermissions {
    pub dialog: bool,
    pub pause: bool,
}

#[derive(..PermTree)]
pub struct CpuPermissions {
    pub registers: bool,
}

#[derive(..PermTree)]
pub struct PpuPermissions {
    pub display: bool,
}

#[derive(..PermTree)]
pub struct BusPermissions {
    pub read: bool,
    pub write: bool,
}

#[derive(..PermTree)]
pub struct ExternalPermissions {
    pub filesystem: FileSystemPermissions,
    pub http: bool,
}

#[derive(..PermTree)]
pub struct FileSystemPermissions {
    pub read: bool,
    pub write: AllOr<FileWritePermissions>,
}

#[derive(Default, PartialEq, Eq, Debug)]
pub struct FileWritePermissions {
    pub files: HashMap<PathBuf, self::filesystem::FileWriteOptions>,
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
                        read = "all",
                        write = "none",
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
                    read: true,
                    write: Permission::none(),
                },
                ..Permission::none()
            },
        };

        assert!(tree == expected_tree);
    }
}
