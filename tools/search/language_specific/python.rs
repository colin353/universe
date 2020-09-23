use search_proto_rust::*;

static MIN_KEYWORD_LENGTH: usize = 3;

lazy_static! {
    static ref FUNCTION_DEFINITION: regex::Regex =
        { regex::Regex::new(r"\s*def\s*(\w+)\(").unwrap() };
    static ref CLASS_DEFINITION: regex::Regex =
        { regex::Regex::new(r"\s*class\s+(\w+)[\(:]").unwrap() };
    static ref VARIABLE_DEFINITION: regex::Regex =
        { regex::Regex::new(r"\s*(\w+)\s+=\s+").unwrap() };
    static ref STOPWORDS: std::collections::HashSet<String> = {
        let mut s = std::collections::HashSet::new();
        s.insert("class".into());
        s.insert("def".into());
        s.insert("super".into());
        s.insert("import".into());
        s.insert("in".into());
        s.insert("is".into());
        s.insert("not".into());
        s.insert("or".into());
        s.insert("None".into());
        s.insert("from".into());
        s.insert("for".into());
        s.insert("self".into());
        s.insert("return".into());
        s.insert("if".into());
        s.insert("elif".into());
        s.insert("raise".into());
        s
    };
    static ref IMPORT_DEFINITION_1: regex::Regex =
        { regex::Regex::new(r"^\s*from\s+(\S+)\s+import").unwrap() };
    static ref IMPORT_DEFINITION_2: regex::Regex =
        { regex::Regex::new(r"^\s*import\s+(\S+)").unwrap() };
}

pub fn extract_imports(file: &File) -> Vec<String> {
    let mut results = Vec::new();
    for line in file.get_content().lines() {
        for captures in IMPORT_DEFINITION_1.captures_iter(line) {
            let import_path = &captures[captures.len() - 1];
            results.push(format!("{}.py", import_path.replace(".", "/")));
        }
        for captures in IMPORT_DEFINITION_2.captures_iter(line) {
            let import_path = &captures[captures.len() - 1];
            results.push(format!("{}.py", import_path.replace(".", "/")));
        }
    }
    results
}

pub fn extract_keywords(file: &File) -> Vec<ExtractedKeyword> {
    let mut results = std::collections::BTreeMap::<String, ExtractedKeyword>::new();
    for keyword in file
        .get_content()
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
    {
        if keyword.len() < MIN_KEYWORD_LENGTH {
            continue;
        }

        if STOPWORDS.contains(keyword) {
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
        for captures in CLASS_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::STRUCTURE);
            results.push(d);
        }
        for captures in FUNCTION_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::FUNCTION);
            results.push(d);
        }
        for captures in VARIABLE_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::VARIABLE);
            results.push(d);
        }
    }
    results
}

pub fn annotate_file(file: &mut File) {
    if file.get_filename().contains("__tests__")
        || file.get_filename().contains("/tests/")
        || file.get_filename().contains("/__snapshots__")
        || file.get_filename().ends_with(".ambr")
        || file.get_filename().contains("/test_")
    {
        file.set_is_test(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_fn(symbol: &str, line: u32) -> SymbolDefinition {
        let mut s = SymbolDefinition::new();
        s.set_symbol(symbol.to_string());
        s.set_symbol_type(SymbolType::FUNCTION);
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
    fn test_extract_imports() {
        let mut f = File::new();
        f.set_content(
            "
                from abcdef.gooble.test_123 import Comment
                from constants.xyz.mycode import (
                    MY_BIG_CONST,
                    test_constant,
                )
                import re
            "
            .into(),
        );

        let result = extract_imports(&f);
        assert_eq!(result[0], "abcdef/gooble/test_123.py");
        assert_eq!(result[1], "constants/xyz/mycode.py");
        assert_eq!(result[2], "re.py");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_extract_keywords() {
        let mut f = File::new();
        f.set_content(String::from(
            "
    num = 7

# To take input from the user
num = int(input())

factorial = 1

def fact(self):
    # check if the number is negative, positive or zero
    for i in range(1,num + 1):
       factorial = factorial*i
        ",
        ));

        let extracted = extract_definitions(&f);
        let expected = vec![
            test_var("num", 1),
            test_var("num", 4),
            test_var("factorial", 6),
            test_fn("fact", 8),
            test_var("factorial", 11),
        ];
        assert_eq!(extracted, expected);
    }
}
