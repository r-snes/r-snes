use std::cmp::Ordering;

/// Combine two [`std::cmp::Ordering`]s as per product order
/// logic:
/// - only return Equal when both elements are Equal
/// - return Less when we have only Less or Less and Equal
/// - same as above Greater
/// - return None in case we have a combination of Greater and Less
///
/// This function is commutative: the order of the arguments
/// doesn't impact the return value
pub fn combine_ordering(o1: Ordering, o2: Ordering) -> Option<Ordering> {
    use Ordering::*;

    match (o1, o2) {
        (Equal, x) | (x, Equal) => Some(x),
        (Less, Less) => Some(Less),
        (Greater, Greater) => Some(Greater),
        _ => None,
    }
}

/// Reduce an iterator of [`std::cmp::Ordering`] into a single
/// `Option<Ordering>`, as per the product order rules described
/// for [`combine_ordering`].
///
/// The order of elements in the iterator don't impact the return
/// value of this function
pub fn combine_orderings<I>(iter: I) -> Option<Ordering>
where
    I: IntoIterator<Item = Ordering>,
{
    let mut acc = Ordering::Equal;

    for i in iter {
        acc = combine_ordering(acc, i)?;
    }
    Some(acc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Ordering::*;
    use duplicate::duplicate_item;

    #[test]
    fn equalities() {
        assert_eq!(combine_ordering(Equal, Equal), Some(Equal));
        assert_eq!(combine_orderings([Equal, Equal, Equal, Equal]), Some(Equal));
    }

    #[duplicate_item(
        DUP_name    DUP_order;
        [greater]   [Greater];
        [less]      [Less];
    )]
    #[test]
    fn DUP_name() {
        assert_eq!(combine_ordering(Less, Less), Some(Less));
        assert_eq!(combine_ordering(Equal, Less), Some(Less));
        assert_eq!(combine_ordering(Less, Equal), Some(Less));

        assert_eq!(combine_orderings([Less]), Some(Less));
        assert_eq!(
            combine_orderings([Less, Equal, Equal, Less]),
            Some(Less)
        );
        assert_eq!(
            combine_orderings([Less, Less, Equal, Less, Equal, Less]),
            Some(Less)
        );
    }

    #[test]
    fn not_comparable() {
        assert_eq!(combine_ordering(Less, Greater), None);
        assert_eq!(combine_ordering(Greater, Less), None);

        assert_eq!(combine_orderings([Greater, Less]), None);
        assert_eq!(combine_orderings([Less, Greater]), None);
        assert_eq!(combine_orderings([Less, Greater, Equal, Equal]), None);
        assert_eq!(combine_orderings([Less, Equal, Equal, Greater]), None);
        assert_eq!(combine_orderings([Equal, Less, Equal, Greater]), None);
    }
}
