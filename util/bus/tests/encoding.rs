#[cfg(test)]
mod tests {
    use bus::Serialize;

    #[test]
    fn test_encode() {
        let mut data = battery::Data::new();
        data.id = 1234;
        data.name = String::from("asdf");
        data.payload = vec![0x0a, 0x0b, 0x0c];

        let mut buf = Vec::new();
        data.encode(&mut buf).unwrap();

        let d = battery::Data::from_bytes(&buf).unwrap();
        assert_eq!(d.id, 1234);
        assert_eq!(&d.name, "asdf");
        assert_eq!(&d.payload, &[0x0a, 0x0b, 0x0c]);
    }
}
