#[cfg(test)]
mod tests {
    use crate::{Toot, Zoot};

    #[test]
    fn test_something() {
        let z = Zoot::new();
        z.set_size(5);
        assert_eq!(z.get_size(), 5);
    }
}
