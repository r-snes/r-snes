/// Trait representing any kind of permission, with varying degrees
///
/// [`Permission`] requires the implementor to define two levels of granted permissions:
/// - [`all`]: As much permissions as the implementor can represent
/// - [`none`]: Absolutely zero permission
pub trait Permission: Eq + PartialOrd + Sized {
    /// Construct a [`Permission`] requesting as much as the implementor can represent
    fn all() -> Self;

    /// Construct a [`Permission`] requesting absolutely zero permission
    fn none() -> Self;

    /// Check if the [`Permission`] object currently represents maximum permissions
    fn is_all(&self) -> bool {
        *self == Self::all()
    }

    /// Check if the [`Permission`] object currently represents empty permissions
    fn is_none(&self) -> bool {
        *self == Self::none()
    }
}

impl Permission for bool {
    fn all() -> Self {
        true
    }

    fn none() -> Self {
        false
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn trivial_checks() {
        assert!(true.is_all());
        assert!(false.is_none());
    }
}
