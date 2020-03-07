use search_proto_rust::*;

static MIN_KEYWORD_LENGTH: usize = 4;
static SPLIT_CHARS: &'static [char] = &[' ', ',', '.', '?'];

pub fn extract_keywords(file: &File) -> Vec<ExtractedKeyword> {
    let mut results = std::collections::BTreeMap::<String, ExtractedKeyword>::new();
    for line in file.get_content().lines() {
        for entity in line.split(SPLIT_CHARS) {
            if entity.len() < MIN_KEYWORD_LENGTH {
                continue;
            }

            if let Some(kw) = results.get_mut(entity) {
                kw.set_occurences(kw.get_occurences() + 1);
            } else {
                let mut kw = ExtractedKeyword::new();
                kw.set_keyword(entity.to_owned());
                kw.set_occurences(1);
                results.insert(entity.to_owned(), kw);
            }
        }
    }

    results.into_iter().map(|(_, x)| x).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kw(word: &str, occ: u64) -> ExtractedKeyword {
        let mut xk = ExtractedKeyword::new();
        xk.set_keyword(word.to_owned());
        xk.set_occurences(occ);
        xk
    }

    #[test]
    fn test_extract_keywords() {
        let mut f = File::new();
        f.set_content("am I? a man from a Japan... a, from".into());

        let expected = vec![kw("Japan", 1), kw("from", 2)];

        assert_eq!(extract_keywords(&f), expected);
    }
}
