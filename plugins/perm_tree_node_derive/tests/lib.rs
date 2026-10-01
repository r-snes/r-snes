//! integration test crate for PermTreeNode derive

use perm_tree_node_derive::PermTreeNode;
use permission_derive_macro::Permission;

use plugins::{perm_tree::PermTreeNode, permission::Permission};

#[derive(Debug, Eq, PartialEq, PartialOrd, Permission, PermTreeNode)]
struct PermTreeRoot {
    field1: bool,
    node1: Node1,
    node2: Node2,
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Permission, PermTreeNode)]
struct Node1 {
    subnode: SubNode,
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Permission, PermTreeNode)]
struct SubNode {
    subfield1: bool,
    subfield2: bool,
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Permission, PermTreeNode)]
struct Node2 {
    a: bool,
    b: bool,
    c: bool,
}

#[cfg(test)]
mod test {
    use super::*;
    use piccolo::{Closure, Context, Executor, Lua, Value};
    use plugins::perm_tree::PermTreeNode;

    fn build_from_lua<T, F>(lua_str: &str, f: F) -> T
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

    #[test]
    fn build_all_none() {
        assert_eq!(
            build_from_lua("\"all\"", PermTreeRoot::from_lua),
            Some(Permission::all())
        );
        assert_eq!(
            build_from_lua("\"none\"", PermTreeRoot::from_lua),
            Some(Permission::none())
        );
    }

    #[test]
    fn build_detailed() {
        assert_eq!(
            build_from_lua(
                "{ field1 = true, node1 = \"all\", node2 = { \"a\", \"b\" }, }",
                PermTreeRoot::from_lua
            ),
            Some(PermTreeRoot {
                field1: true,
                node1: Permission::all(),
                node2: Node2 {
                    a: true,
                    b: true,
                    c: false
                }
            })
        );
    }
}
