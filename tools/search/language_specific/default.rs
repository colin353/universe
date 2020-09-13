use search_proto_rust::*;

static MIN_KEYWORD_LENGTH: usize = 4;
static SPLIT_CHARS: &'static [char] = &[' ', ',', '.', '?', ':'];

lazy_static! {
    static ref KEYWORDS_RE: regex::Regex = { regex::Regex::new(r"(\w+)").unwrap() };
}

pub fn extract_keywords(file: &File) -> Vec<ExtractedKeyword> {
    let mut results = std::collections::BTreeMap::<String, ExtractedKeyword>::new();
    for captures in KEYWORDS_RE.captures_iter(file.get_content()) {
        let keyword = &captures[0];
        if keyword.len() < MIN_KEYWORD_LENGTH {
            continue;
        }

        if let Some(kw) = results.get_mut(keyword) {
            kw.set_occurrences(kw.get_occurrences() + 1);
        } else {
            let mut kw = ExtractedKeyword::new();
            kw.set_keyword(keyword.to_owned());
            kw.set_occurrences(1);
            results.insert(keyword.to_owned(), kw);
        }
    }

    results.into_iter().map(|(_, x)| x).collect()
}

pub fn annotate_file(file: &mut File) {
    if file.get_filename().contains("__tests__")
        || file.get_filename().contains("/tests/")
        || file.get_filename().contains("__snapshots__")
    {
        file.set_is_test(true);
    }
}

pub fn extract_definitions(file: &File) -> Vec<SymbolDefinition> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kw(word: &str, occ: u64) -> ExtractedKeyword {
        let mut xk = ExtractedKeyword::new();
        xk.set_keyword(word.to_owned());
        xk.set_occurrences(occ);
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
