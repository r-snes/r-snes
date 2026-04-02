use permission_derive_macro::Permission;
use perm_tree_node_derive::PermTreeNode;

use plugins::{
    permission::Permission,
    perm_tree::PermTreeNode,
};

#[derive(Debug, Eq, PartialEq, PartialOrd, Permission, PermTreeNode)]
struct PermTreeRoot {
    field1: bool,
    node1: Node1,
    node2: Node2,
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Permission, PermTreeNode)]
struct Node1 {
    subnode: SubNode
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
