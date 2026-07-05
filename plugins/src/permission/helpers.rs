use super::Permission;

/// Helper enum to make it easier to implement
/// [`Permission`] on types which don't have a fitting
/// value to represent [`Permission::all`], but already
/// have a notion of "none".
/// T's [`Default`] implementation will be used to represent "none"
///
/// For example, a Vec can use the empty vector as a representation
/// of "none", but does not have a fitting representation of "all"
#[derive(Debug)]
pub enum AllOr<T> {
    /// Artificially added "all" value
    All,

    /// The wrapped type, which already has a notion of "none"
    Inner(T),
}

impl<T: Default> Default for AllOr<T> {
    fn default() -> Self {
        Self::Inner(T::default())
    }
}

impl<T: PartialEq> PartialEq for AllOr<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::All, Self::All) => true,
            (Self::Inner(i1), Self::Inner(i2)) => i1.eq(i2),

            _ => false,
        }
    }
}
impl<T: Eq> Eq for AllOr<T> {}

impl<T: PartialOrd> PartialOrd for AllOr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::All, Self::All) => Some(std::cmp::Ordering::Equal),

            (_, Self::All) => Some(std::cmp::Ordering::Less),
            (Self::All, _) => Some(std::cmp::Ordering::Greater),

            (Self::Inner(i1), Self::Inner(i2)) => i1.partial_cmp(i2),
        }
    }
}

impl<T: Eq + PartialOrd + Default> Permission for AllOr<T> {
    fn all() -> Self {
        Self::All
    }

    fn none() -> Self {
        Self::Inner(T::default())
    }
}

#[cfg(test)]
mod tests {
    use super::Permission;
    use std::collections::HashSet;

    use crate::permission::helpers::AllOr;

    #[derive(Default, PartialEq, Eq)]
    struct ListOfAllowedThings {
        pub things: HashSet<String>,
    }

    impl PartialOrd for ListOfAllowedThings {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            // simple case for equality: compare equal
            if self == other {
                return Some(std::cmp::Ordering::Equal);
            }

            // subset / superset relations define a partial order, use them
            if self.things.is_superset(&other.things) {
                return Some(std::cmp::Ordering::Greater);
            }
            if self.things.is_subset(&other.things) {
                return Some(std::cmp::Ordering::Less);
            }

            // otherwise, no real order can be decided between the elements
            None
        }
    }

    type AllowedThingsPerm = AllOr<ListOfAllowedThings>;

    #[test]
    fn can_build_as_perm() {
        let all: AllowedThingsPerm = Permission::all();
        let none: AllowedThingsPerm = Permission::none();

        assert!(all.is_all());
        assert!(none.is_none());

        assert!(all != none);

        assert!(all > none);
        assert!(all >= none);

        assert!(none < all);
        assert!(none <= all);
    }

    #[test]
    fn comparisons_work() {
        let all: AllowedThingsPerm = Permission::all();
        let none: AllowedThingsPerm = Permission::none();

        let abc = AllowedThingsPerm::Inner(ListOfAllowedThings {
            things: HashSet::from(["a".into(), "b".into(), "c".into()]),
        });
        let ab = AllowedThingsPerm::Inner(ListOfAllowedThings {
            things: HashSet::from(["a".into(), "b".into()]),
        });

        assert!(all > abc);
        assert!(all > ab);

        assert!(none < abc);
        assert!(none < ab);

        assert!(abc > ab);
        assert!(abc >= ab);

        assert!(ab < abc);
        assert!(ab <= abc);
    }
}
