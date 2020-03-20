use search_proto_rust::*;

static MIN_KEYWORD_LENGTH: usize = 3;

lazy_static! {
    static ref KEYWORDS_RE: regex::Regex = { regex::Regex::new(r"(\w+)").unwrap() };
    static ref NUMERIC: regex::Regex = { regex::Regex::new(r"\d+").unwrap() };
    static ref MESSAGE_DEFINITION: regex::Regex =
        { regex::Regex::new(r"^\s*message\s+(\w+)").unwrap() };
    static ref FIELD_DEFINITION: regex::Regex =
        { regex::Regex::new(r"^\s*(repeated\s)?\s*(\w+)\s+(\w+)\s*=").unwrap() };
    static ref STOPWORDS: std::collections::HashSet<String> = {
        let mut s = std::collections::HashSet::new();
        s.insert("message".into());
        s.insert("string".into());
        s.insert("enum".into());
        s.insert("int".into());
        s.insert("bool".into());
        s.insert("int64".into());
        s.insert("int32".into());
        s.insert("uint64".into());
        s.insert("uint32".into());
        s.insert("repeated".into());
        s.insert("syntax".into());
        s.insert("rpc".into());
        s.insert("service".into());
        s.insert("returns".into());
        s
    };
}

pub fn extract_keywords(file: &File) -> Vec<ExtractedKeyword> {
    let mut results = std::collections::BTreeMap::<String, ExtractedKeyword>::new();
    for captures in KEYWORDS_RE.captures_iter(file.get_content()) {
        let keyword = &captures[0];
        if STOPWORDS.contains(keyword) {
            continue;
        }

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

pub fn extract_definitions(file: &File) -> Vec<SymbolDefinition> {
    let mut results = Vec::new();
    for (line_number, line) in file.get_content().lines().enumerate() {
        for captures in MESSAGE_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::STRUCTURE);
            results.push(d);
        }
        for captures in FIELD_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::VARIABLE);
            results.push(d);
        }
    }
    results
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
        f.set_content(
            "message File {
  string filename = 1;

  // Whether the file was found or not.
  bool found = 2;
  bool deleted = 3;

  // Whether this file actually represents a directory.
  bool directory = 4;

  // The unix file attributes.
  uint64 mtime = 5;
  uint64 atime = 6;
  uint64 ctime = 7;
  uint64 crtime = 8;
  uint64 nlink = 9;
  uint64 rdev = 10;
  uint64 flags = 11;
  uint64 perm = 12;
    "
            .into(),
        );

        let extracted = extract_keywords(&f);

        assert_eq!(extracted.len(), 24);
        assert_eq!(&extracted[0], &kw("File", 1));
    }

    fn test_s(symbol: &str, line: u32) -> SymbolDefinition {
        let mut s = SymbolDefinition::new();
        s.set_symbol(symbol.to_string());
        s.set_symbol_type(SymbolType::STRUCTURE);
        s.set_line_number(line);
        s
    }

    fn test_var(symbol: &str, line: u32) -> SymbolDefinition {
        let mut s = SymbolDefinition::new();
        s.set_symbol(symbol.to_string());
        s.set_symbol_type(SymbolType::VARIABLE);
        s.set_line_number(line);
        s
    }

    #[test]
    fn test_extract_definitions() {
        let mut f = File::new();
        f.set_content(
            "message File {
  string filename = 1;

  // Whether the file was found or not.
  bool found = 2;
  bool deleted = 3;
    "
            .into(),
        );

        let extracted = extract_definitions(&f);
        let expected = vec![
            test_s("File", 0),
            test_var("filename", 1),
            test_var("found", 4),
            test_var("deleted", 5),
        ];
        assert_eq!(extracted, expected);
    }
}
