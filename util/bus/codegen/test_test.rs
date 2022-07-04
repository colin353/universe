#[cfg(test)]
mod tests {
    use crate::{Container, ContainerView, Toot, TootView, Zoot, ZootView};
    use bus::Serialize;
    #[test]
    fn test_nested_struct_encode_decode() {
        let mut t = Toot::new();
        t.id = 5;

        // Encoded version
        let mut buf = Vec::new();
        t.encode(&mut buf).unwrap();

        // Read from bytes
        let bt = TootView::from_bytes(&buf).unwrap();
        assert_eq!(bt.get_id(), 5);

        let mut z = Zoot::new();
        z.toot = t;

        // Encode nested struct
        let mut buf = Vec::new();
        z.encode(&mut buf).unwrap();

        // Read from bytes
        let bz = ZootView::from_bytes(&buf).unwrap();
        assert_eq!(bz.get_toot().get_id(), 5);

        let mut bz = bz.to_owned().unwrap();
        bz.name = String::from("Colin");

        let mut buf2 = Vec::new();
        bz.encode(&mut buf2).unwrap();

        let bz = ZootView::from_bytes(&buf2).unwrap();
        assert_eq!(bz.get_name(), "Colin");
    }

    #[test]
    fn test_repeated_field() {
        let mut z = Zoot::new();
        {
            let s = &mut z.size;
            s.push(5);
            s.push(10);
            s.push(15);
            s.push(20);
        }

        z.toot.id = 77;

        let mut buf = Vec::new();
        z.encode(&mut buf).unwrap();

        let bz = ZootView::from_bytes(&buf).unwrap();
        assert_eq!(bz.get_size().get(0), Some(5));
        assert_eq!(bz.get_size().get(1), Some(10));
        assert_eq!(bz.get_size().get(2), Some(15));
        assert_eq!(bz.get_size().get(3), Some(20));
        assert_eq!(bz.get_size().get(4), None);
        assert_eq!(bz.get_toot().get_id(), 77);
    }

    #[test]
    fn test_bytes() {
        let t = Toot {
            id: 5,
            data: vec![1, 2, 3, 4],
        };
        let mut buf = Vec::new();
        t.encode(&mut buf).unwrap();

        assert_eq!(
            &buf,
            &[
                5, // f0
                1, 2, 3, 4, // f1
                1, 1, // pack<1>
                3  // footer
            ]
        );
    }

    #[test]
    fn test_debug_representation() {
        let mut t = Toot::new();
        t.id = 15;
        let out = format!("{:?}", t);
        assert_eq!(out, r#"Toot { id: 15, data: [] }"#);

        let mut z = Zoot::new();
        {
            let s = &mut z.size;
            s.push(5);
            s.push(10);
            s.push(15);
            s.push(20);
        }
        z.toot.id = 77;

        let mut buf = Vec::new();
        z.encode(&mut buf).unwrap();
        let bz = Zoot::from_bytes(&buf).unwrap();

        let out = format!("{:?}", bz);
        assert_eq!(
            out,
            r#"Zoot { toot: Toot { id: 77, data: [] }, size: [5, 10, 15, 20], name: "" }"#
        );
    }

    #[test]
    fn test_repeated_struct() {
        let mut c = Container::new();
        c.values = vec![
            Toot {
                id: 23,
                data: vec![],
            },
            Toot {
                id: 34,
                data: vec![],
            },
        ];

        let cv = c.as_view();

        for (idx, v) in cv.get_values().iter().enumerate() {
            if idx == 0 {
                assert_eq!(v.get_id(), 23);
            } else {
                assert_eq!(v.get_id(), 34);
            }
        }
        assert_eq!(cv.get_values().iter().count(), 2);

        let mut buf = Vec::new();
        cv.encode(&mut buf).unwrap();

        let bc = ContainerView::from_bytes(&buf).unwrap();
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
        c.values = vec![
            Toot {
                id: 23,
                data: vec![1, 2, 3],
            },
            Toot {
                id: 34,
                data: vec![4, 5, 6],
            },
        ];
        c.names = vec![String::from("asdf"), String::from("fdsa")];

        assert_eq!(
            format!("{:?}", c.as_view()),
            "Container { values: [Toot { id: 23, data: [1, 2, 3] }, Toot { id: 34, data: [4, 5, 6] }], names: [\"asdf\", \"fdsa\"] }"
        );

        let cv = c.as_view();

        for (idx, v) in cv.get_names().iter().enumerate() {
            if idx == 0 {
                assert_eq!(v, "asdf");
            } else {
                assert_eq!(v, "fdsa");
            }
        }
        assert_eq!(cv.get_names().iter().count(), 2);

        let mut buf = Vec::new();
        cv.encode(&mut buf).unwrap();

        let bc = ContainerView::from_bytes(&buf).unwrap();

        assert_eq!(
            format!("{:?}", bc),
            "Container { values: [Toot { id: 23, data: [1, 2, 3] }, Toot { id: 34, data: [4, 5, 6] }], names: [\"asdf\", \"fdsa\"] }"
        );

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
