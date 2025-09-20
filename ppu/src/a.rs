pub fn add(a: i32, b: i32) -> i32 {
  a + b
}

#[cfg(test)]
#[path ="./a_tests.rs"]
mod test;
