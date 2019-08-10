pub fn merge(original: &str, a: &str, b: &str) -> (String, bool) {
    (String::from(a), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge() {
        let (joined, ok) = merge("a brown cow", "a cow", "a cow");
        assert!(ok);
        assert_eq!(&joined, "a cow");
    }
}
