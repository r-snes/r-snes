pub mod registers;
pub mod cpu;
mod instrs;

#[cfg(test)]
mod tests {
    use super::*;
    use registers::Registers;


    #[test]
    fn regs () {
        let _ = Registers::default();
    }
}
