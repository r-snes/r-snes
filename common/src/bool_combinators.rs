//! Small combinators which AND and OR iterators of bools

/// ORs together an iterator of booleans, returning false if there are no elements
pub fn or(iter: impl IntoIterator<Item = bool>) -> bool {
    iter.into_iter().reduce(|a, b| a || b).unwrap_or(false)
}

/// ANDs together an iterator of booleans, returning true if there are no elements
pub fn and(iter: impl IntoIterator<Item = bool>) -> bool {
    iter.into_iter().reduce(|a, b| a && b).unwrap_or(true)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn or_and_empty() {
        assert!(!or([]));
        assert!(and([]));
    }

    #[test]
    fn or_cases() {
        assert!(!or([false]));
        assert!(or([true]));

        assert!(!or([false, false, false]));
        assert!(or([true, true, true]));

        assert!(or([false, true, false]));
    }

    #[test]
    fn and_cases() {
        assert!(!and([false]));
        assert!(and([true]));

        assert!(!and([false, false, false]));
        assert!(and([true, true, true]));

        assert!(!and([false, true, false]));
    }
}
