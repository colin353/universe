#[cfg(test)]
mod tests {
    use crate::{Toot, Zoot};
    use car::{Deserialize, Serialize};

    #[test]
    fn test_something() {
        let mut t = Toot::new();
        t.set_id(5);
        assert_eq!(t.get_id(), 5);

        // Encoded version
        let mut buf = Vec::new();
        t.encode(&mut buf).unwrap();

        // Read from bytes
        let bz = Toot::from_bytes(&buf).unwrap();
        assert_eq!(bz.get_id(), 5);

        let mut z = Zoot::new();
        z.set_toot(t);
    }
}
