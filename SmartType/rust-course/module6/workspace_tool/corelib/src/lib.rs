pub fn greet(name: &str) -> String {
    format!("Hello, {name}! Welcome to the workspace tool.")
}

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greets_name() {
        let msg = greet("Ferris");
        assert!(msg.contains("Ferris"));
    }

    #[test]
    fn adds_numbers() {
        assert_eq!(add(2, 3), 5);
    }
}
