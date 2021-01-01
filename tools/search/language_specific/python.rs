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

    let prefix: Vec<usize> = vec![0];
    let suffix: Vec<usize> = vec![file.get_content().len()];

    let mut newlines: Vec<_> = prefix
        .into_iter()
        .chain(
            file.get_content()
                .match_indices("\n")
                .map(|(index, _)| index),
        )
        .chain(suffix.into_iter())
        .collect();

    for (line_number, window) in newlines.windows(2).enumerate() {
        let line_start = window[0];
        let line_end = window[1];
        let line = &file.get_content()[line_start..line_end];

        for captures in CLASS_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::STRUCTURE);

            let end_line =
                line_number + take_until_whitespace(&file.get_content()[line_start + 1..]);
            d.set_end_line_number(end_line as u32);

            results.push(d);
        }
        for captures in FUNCTION_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::FUNCTION);

            let end_line =
                line_number + take_until_whitespace(&file.get_content()[line_start + 1..]);
            d.set_end_line_number(end_line as u32);

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

fn count_whitespace(line: &str) -> usize {
    let mut count = 0;
    for ch in line.chars() {
        count += match ch {
            '\t' => 8,
            ' ' => 1,
            _ => break,
        }
    }

    count
}

fn take_until_whitespace(text: &str) -> usize {
    let mut lines = text.lines();

    let initial_indent = match lines.next() {
        Some(x) => count_whitespace(x),
        None => return 0,
    };

    let mut line_number = 0;
    let mut whitespace_lines = 0;

    while let Some(line) = lines.next() {
        // Skip if the entire line is whitespace
        if line.find(|c| !char::is_whitespace(c)).is_none() {
            whitespace_lines += 1;
            continue;
        }

        if count_whitespace(line) <= initial_indent {
            break;
        }
        line_number += whitespace_lines + 1;
        whitespace_lines = 0;
    }
    line_number
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_fn(symbol: &str, line: u32, end_line: u32) -> SymbolDefinition {
        let mut s = SymbolDefinition::new();
        s.set_symbol(symbol.to_string());
        s.set_symbol_type(SymbolType::FUNCTION);
        s.set_line_number(line);
        s.set_end_line_number(end_line);
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
    fn test_take_until_whitespace() {
        let text = "def my_function(a, b, c):
    print(a)
    if b:
        print(c)

def another_function(c, d, e):
    pass
        ";

        assert_eq!(take_until_whitespace(text), 3);
    }

    #[test]
    fn test_extract_definitions() {
        let mut f = File::new();
        f.set_content(
            "def my_function(a, b, c):
    print(a)
    if b:
        print(c)

    class Borg(object):
        def another_function(c, d, e):
            pass

def final_fn():
    return 3
        "
            .into(),
        );

        let result = extract_definitions(&f);
        assert_eq!(result[0].get_symbol(), "my_function");
        assert_eq!(result[0].get_line_number(), 0);
        assert_eq!(result[0].get_end_line_number(), 7);

        assert_eq!(result[1].get_symbol(), "Borg");
        assert_eq!(result[1].get_line_number(), 5);
        assert_eq!(result[1].get_end_line_number(), 7);

        assert_eq!(result[2].get_symbol(), "another_function");
        assert_eq!(result[2].get_line_number(), 6);
        assert_eq!(result[2].get_end_line_number(), 7);
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

    def xyz(self):
        # test
        if x:
           y

    return z
        ",
        ));

        let extracted = extract_definitions(&f);
        let expected = vec![
            test_var("num", 1),
            test_var("num", 4),
            test_var("factorial", 6),
            test_fn("fact", 8, 18),
            test_var("factorial", 11),
            test_fn("xyz", 13, 16),
        ];
        assert_eq!(extracted.len(), expected.len());
        for i in 0..extracted.len() {
            assert_eq!(extracted[i], expected[i]);
        }
    }
}
