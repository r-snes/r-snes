pub mod registers;
pub mod cpu;

#[cfg(test)]
mod tests {
    use super::*;
    use registers::Registers;


    #[test]
    fn regs () {
        let _ = Registers::default();
    }
}
