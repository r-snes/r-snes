fn main() {
    println!("You just ran the root main.rs");
}

#[allow(dead_code)] // Function never read
fn add(nb1: u16, nb2: u16) -> u16 {
    nb1 + nb2
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }
}
