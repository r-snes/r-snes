//! Small combinators which AND and OR iterators of bools

/// ORs together an iterator of booleans, returning true if there are no elements
pub fn or(iter: impl IntoIterator<Item = bool>) -> bool {
    iter.into_iter().reduce(|a, b| a || b).unwrap_or(true)
}

/// ANDs together an iterator of booleans, returning false if there are no elements
pub fn and(iter: impl IntoIterator<Item = bool>) -> bool {
    iter.into_iter().reduce(|a, b| a && b).unwrap_or(false)
}
