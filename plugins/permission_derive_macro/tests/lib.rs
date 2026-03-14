use permission_derive_macro::Permission;
use plugins::permission::Permission;

#[derive(Debug, Eq, PartialEq, PartialOrd)]
enum MyPerm {
    None,
    All,
}

impl Permission for MyPerm {
    fn all() -> Self {
        Self::All
    }

    fn none() -> Self {
        Self::None
    }
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Permission)]
struct MyCombinedPerm {
    perm1: MyPerm,
    perm2: MyPerm,
    perm3: MyPerm,
}

#[test]
fn derive_perm_all() {
    assert_eq!(MyCombinedPerm::all(), MyCombinedPerm {
        perm1: MyPerm::All,
        perm2: MyPerm::All,
        perm3: MyPerm::All,
    })
}

#[test]
fn derive_perm_none() {
    assert_eq!(MyCombinedPerm::none(), MyCombinedPerm {
        perm1: MyPerm::None,
        perm2: MyPerm::None,
        perm3: MyPerm::None,
    })
}
