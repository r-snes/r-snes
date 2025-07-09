mod registers;

#[cfg(test)]
mod tests {
    use super::*;
    use registers::Registers;


    #[test]
    fn regs () {
        let regs = Registers::default();
    }
}
