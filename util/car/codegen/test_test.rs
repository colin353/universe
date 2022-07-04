#[cfg(test)]
mod tests {
    use crate::{Container, Toot, TootOwned, Zoot};
    #[test]
    fn test_nested_struct_encode_decode() {
        let mut t = Toot::new();
        t.set_id(5).unwrap();
        assert_eq!(t.get_id(), 5);

        // Encoded version
        let mut buf = Vec::new();
        t.encode(&mut buf).unwrap();

        // Read from bytes
        let bt = Toot::from_bytes(&buf).unwrap();
        assert_eq!(bt.get_id(), 5);

        let mut z = Zoot::new();
        z.set_toot(t).unwrap();

        // Encode nested struct
        let mut buf = Vec::new();
        z.encode(&mut buf).unwrap();

        // Read from bytes
        let mut bz = Zoot::from_bytes(&buf).unwrap();
        assert_eq!(bz.get_toot().get_id(), 5);

        bz.set_name(String::from("Colin")).unwrap();

        let mut buf2 = Vec::new();
        bz.encode(&mut buf2).unwrap();

        let bz = Zoot::from_bytes(&buf2).unwrap();
        assert_eq!(bz.get_name(), "Colin");
    }

    #[test]
    fn test_repeated_field() {
        let mut z = Zoot::new();
        {
            let s = z.mut_size().unwrap();
            s.push(5);
            s.push(10);
            s.push(15);
            s.push(20);
        }

        z.mut_toot().unwrap().id = 77;

        let mut buf = Vec::new();
        z.encode(&mut buf).unwrap();

        let bz = Zoot::from_bytes(&buf).unwrap();
        assert_eq!(bz.get_size().get(0), Some(5));
        assert_eq!(bz.get_size().get(1), Some(10));
        assert_eq!(bz.get_size().get(2), Some(15));
        assert_eq!(bz.get_size().get(3), Some(20));
        assert_eq!(bz.get_size().get(4), None);
        assert_eq!(bz.get_toot().get_id(), 77);
    }

    #[test]
    fn test_debug_representation() {
        let mut t = Toot::new();
        t.set_id(15).unwrap();
        let out = format!("{:?}", t);
        assert_eq!(out, r#"Toot { id: 15 }"#);

        let mut z = Zoot::new();
        {
            let s = z.mut_size().unwrap();
            s.push(5);
            s.push(10);
            s.push(15);
            s.push(20);
        }
        z.mut_toot().unwrap().id = 77;

        let mut buf = Vec::new();
        z.encode(&mut buf).unwrap();
        let bz = Zoot::from_bytes(&buf).unwrap();

        let out = format!("{:?}", bz);
        assert_eq!(
            out,
            r#"Zoot { toot: Toot { id: 77 }, size: [5, 10, 15, 20], name: "" }"#
        );
    }

    #[test]
    fn test_repeated_struct() {
        let mut c = Container::new();
        c.set_values(vec![TootOwned { id: 23 }, TootOwned { id: 34 }])
            .unwrap();

        for (idx, v) in c.get_values().iter().enumerate() {
            if idx == 0 {
                assert_eq!(v.get_id(), 23);
            } else {
                assert_eq!(v.get_id(), 34);
            }
        }
        assert_eq!(c.get_values().iter().count(), 2);

        let mut buf = Vec::new();
        c.encode(&mut buf).unwrap();

        let bc = Container::from_bytes(&buf).unwrap();
        for (idx, v) in bc.get_values().iter().enumerate() {
            if idx == 0 {
                assert_eq!(v.get_id(), 23);
            } else {
                assert_eq!(v.get_id(), 34);
            }
        }
        assert_eq!(bc.get_values().iter().count(), 2);
    }

    #[test]
    fn test_repeated_string() {
        let mut c = Container::new();
        c.set_values(vec![TootOwned { id: 23 }, TootOwned { id: 34 }])
            .unwrap();
        c.set_names(vec![String::from("asdf"), String::from("fdsa")])
            .unwrap();

        assert_eq!(format!("{:?}", c), "Container { values: [TootOwned { id: 23 }, TootOwned { id: 34 }], names: [\"asdf\", \"fdsa\"] }");

        for (idx, v) in c.get_names().iter().enumerate() {
            if idx == 0 {
                assert_eq!(v, "asdf");
            } else {
                assert_eq!(v, "fdsa");
            }
        }
        assert_eq!(c.get_names().iter().count(), 2);

        let mut buf = Vec::new();
        c.encode(&mut buf).unwrap();

        let bc = Container::from_bytes(&buf).unwrap();
        for (idx, v) in bc.get_names().iter().enumerate() {
            if idx == 0 {
                assert_eq!(v, "asdf");
            } else {
                assert_eq!(v, "fdsa");
            }
        }
        assert_eq!(bc.get_names().iter().count(), 2);
    }
}