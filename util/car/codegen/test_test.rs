#[cfg(test)]
mod tests {
    use crate::{Toot, Zoot};
    use car::{Deserialize, Serialize};

    #[test]
    fn test_nested_struct_encode_decode() {
        let mut t = Toot::new();
        t.set_id(5);
        assert_eq!(t.get_id(), 5);

        // Encoded version
        let mut buf = Vec::new();
        t.encode(&mut buf).unwrap();

        // Read from bytes
        let bt = Toot::from_bytes(&buf).unwrap();
        assert_eq!(bt.get_id(), 5);

        let mut z = Zoot::new();
        z.set_toot(t);

        // Encode nested struct
        let mut buf = Vec::new();
        z.encode(&mut buf);

        // Read from bytes
        let mut bz = Zoot::from_bytes(&buf).unwrap();
        assert_eq!(bz.get_toot().get_id(), 5);

        bz.set_name(String::from("Colin"));

        let mut buf2 = Vec::new();
        bz.encode(&mut buf2);

        let bzz = Zoot::from_bytes(&buf2).unwrap();
        assert_eq!(bz.get_name(), "Colin");
    }

    #[test]
    fn test_repeated_field() {
        let mut t = Toot::new();
    }
}
