pub fn unshard(path: &str) -> Vec<String> {
    let last_component = path.rsplit("/").next().unwrap();
    let shard_spec: Vec<_> = last_component.split("@").collect();

    if shard_spec.len() != 2 {
        return vec![path.to_string()];
    }

    let shard_count: usize = match shard_spec.iter().last().unwrap().parse() {
        Ok(c) => c,
        Err(_) => return vec![path.to_string()],
    };

    let mut output = Vec::new();
    for index in 0..shard_count {
        output.push(format!(
            "{}/{}-{:05}-of-{:05}",
            &path[0..path.len() - last_component.len() - 1],
            shard_spec[0],
            index,
            shard_count
        ))
    }
    output
}

pub fn shard(path: &str, shard_count: usize) -> String {
    format!("{}@{}", path, shard_count)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_unshard() {
        let input = "/tmp/file.txt@5";
        assert_eq!(
            unshard(input),
            vec![
                String::from("/tmp/file.txt-00000-of-00005"),
                String::from("/tmp/file.txt-00001-of-00005"),
                String::from("/tmp/file.txt-00002-of-00005"),
                String::from("/tmp/file.txt-00003-of-00005"),
                String::from("/tmp/file.txt-00004-of-00005"),
            ]
        );
    }

    #[test]
    fn test_unshard_2() {
        let input = "/tmp/asdf/file.txt";
        assert_eq!(unshard(input), vec![String::from("/tmp/asdf/file.txt")]);
    }

    #[test]
    fn test_unshard_3() {
        let input = "/tmp/as@df/file.txt";
        assert_eq!(unshard(input), vec![String::from("/tmp/as@df/file.txt")]);
    }

    #[test]
    fn test_unshard_4() {
        let input = "/tmp/asdf/file.txt@text";
        assert_eq!(
            unshard(input),
            vec![String::from("/tmp/asdf/file.txt@text")]
        );
    }

    #[test]
    fn test_shard() {
        assert_eq!(
            shard("/tmp/asdf/file.txt", 2),
            String::from("/tmp/asdf/file.txt@2")
        );
    }
}
