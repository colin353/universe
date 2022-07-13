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

pub fn compact_shards(data: Vec<String>, target_shard_count: usize) -> Vec<String> {
    let mut output = data;
    output.sort();
    output.dedup();
    if output.len() <= target_shard_count {
        return output;
    }

    let mut num_to_mark = output.len() - target_shard_count;
    let mut inverted = true;
    if output.len() > 2 * target_shard_count {
        inverted = false;
        num_to_mark = output.len() - num_to_mark;
    }

    let mut i = 0;
    let mut retained = 0;
    let num_to_jump = output.len() / num_to_mark;
    output.retain(|_| {
        i += 1;
        let should_retain = (i % (num_to_jump) == 0) ^ inverted;

        if should_retain {
            if retained == target_shard_count {
                return false;
            }
            retained += 1;
            return true;
        }

        false
    });
    output
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

    #[test]
    fn test_compact_shards() {
        let pointers = vec![
            String::from("a"),
            String::from("b"),
            String::from("c"),
            String::from("d"),
            String::from("e"),
            String::from("f"),
            String::from("g"),
            String::from("h"),
            String::from("i"),
        ];

        let expected = vec![String::from("c"), String::from("f"), String::from("i")];

        assert_eq!(compact_shards(pointers, 3), expected);
    }

    #[test]
    fn test_compact_shards_2() {
        let pointers = vec![
            String::from("a"),
            String::from("b"),
            String::from("c"),
            String::from("d"),
        ];

        let expected = vec![String::from("a"), String::from("b"), String::from("c")];

        assert_eq!(compact_shards(pointers, 3), expected);
    }
}
