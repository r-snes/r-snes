derive_aliases::define! {
    PermTree =
        ::core::default::Default,
        ::core::cmp::PartialEq,
        ::core::cmp::Eq,
        ::core::fmt::Debug,
        ::strict_partial_ord_derive::PartialOrd,
        ::permission_derive_macro::Permission,
        ::perm_tree_node_derive::PermTreeNode;
}
