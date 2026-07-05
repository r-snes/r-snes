derive_aliases::define! {
    PermTree =
        ::core::default::Default,
        ::core::cmp::PartialEq,
        ::core::cmp::Eq,
        ::core::fmt::Debug,
        ::product_order::PartialOrd,
        ::permission_derive_macro::Permission,
        ::perm_tree_node_derive::PermTreeNode;
}
