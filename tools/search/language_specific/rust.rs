use search_proto_rust::*;

static MIN_KEYWORD_LENGTH: usize = 3;

lazy_static! {
    static ref FUNCTION_DEFINITION: regex::Regex =
        { regex::Regex::new(r"\s*(pub)?\s*fn\s+(\w+)").unwrap() };
    static ref STRUCTURE_DEFINITION: regex::Regex =
        { regex::Regex::new(r"\s*(pub)?\s*struct\s+(\w+)").unwrap() };
    static ref LET_BINDING: regex::Regex =
        { regex::Regex::new(r"\s*let\s*(mut)?\s+(\w+)").unwrap() };
    static ref TRAIT_DEFINITION: regex::Regex =
        { regex::Regex::new(r"\s*(pub)?\s*trait\s+(\w+)").unwrap() };
    static ref STRUCT_IMPL: regex::Regex =
        { regex::Regex::new(r"\s*impl(<.*?>)?\s+(\w+)(<.*?>)?(\s+for\s+(\w+))?").unwrap() };
    static ref STOPWORDS: std::collections::HashSet<String> = {
        let mut s = std::collections::HashSet::new();
        s.insert("let".into());
        s.insert("mut".into());
        s.insert("for".into());
        s.insert("in".into());
        s.insert("while".into());
        s.insert("if".into());
        s.insert("self".into());
        s.insert("ref".into());
        s.insert("pub".into());
        s.insert("extern".into());
        s.insert("return".into());
        s
    };
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

pub fn find_closure_ending_line(content: &str, open: char, close: char) -> Option<usize> {
    let mut line = 0;
    let mut tmp = [0; 4];
    let close_str = close.encode_utf8(&mut tmp);
    let mut depth = 0;
    for m in content.matches(|ch| ch == '\n' || ch == open || ch == close) {
        if m == "\n" {
            line += 1;
        } else if m == close_str {
            depth -= 1;

            if depth == 0 {
                return Some(line);
            }
        } else {
            depth += 1;
        }
    }
    None
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

        for captures in STRUCTURE_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();

            if let Some(full_capture) = captures.get(0) {
                if let Some(idx) = line[full_capture.end()..].find('{') {
                    if let Some(end) = find_closure_ending_line(
                        &file.get_content()[line_start + full_capture.end() + 1..],
                        '{',
                        '}',
                    ) {
                        d.set_end_line_number((line_number + end) as u32);
                    }
                }
            }

            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::STRUCTURE);
            results.push(d);
        }
        for captures in FUNCTION_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();

            if let Some(full_capture) = captures.get(0) {
                // Check if the function HAS a body. Some don't (e.g. within trait definitions).
                let has_body = match file.get_content()[line_start + full_capture.end()..]
                    .matches(&[';', '}', '{'][..])
                    .next()
                {
                    Some("{") => true,
                    _ => false,
                };

                if has_body {
                    if let Some(end) = find_closure_ending_line(
                        &file.get_content()[line_start + full_capture.end() + 1..],
                        '{',
                        '}',
                    ) {
                        d.set_end_line_number((line_number + end) as u32);
                    }
                }
            }

            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::FUNCTION);
            results.push(d);
        }
        for captures in STRUCT_IMPL.captures_iter(line) {
            let mut d = SymbolDefinition::new();

            if let Some(full_capture) = captures.get(0) {
                // Check if the function HAS a body. Some don't (e.g. within trait definitions).
                let has_body = match file.get_content()[line_start + full_capture.end()..]
                    .matches(&[';', '}', '{'][..])
                    .next()
                {
                    Some("{") => true,
                    _ => false,
                };

                if has_body {
                    if let Some(end) = find_closure_ending_line(
                        &file.get_content()[line_start + full_capture.end() + 1..],
                        '{',
                        '}',
                    ) {
                        d.set_end_line_number((line_number + end) as u32);
                    }
                }
            }

            let symbol_name = captures.get(5).or(captures.get(2)).unwrap().as_str();

            d.set_symbol(symbol_name.to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::STRUCTURE);
            results.push(d);
        }
        for captures in LET_BINDING.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::VARIABLE);
            results.push(d);
        }
        for captures in TRAIT_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();

            if let Some(full_capture) = captures.get(0) {
                if let Some(end) = find_closure_ending_line(
                    &file.get_content()[line_start + full_capture.end() + 1..],
                    '{',
                    '}',
                ) {
                    d.set_end_line_number((line_number + end) as u32);
                }
            }
            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::TRAIT);
            results.push(d);
        }
    }
    results
}

pub fn annotate_file(file: &mut File) {
    if file.get_filename().ends_with("tests.rs") || file.get_filename().ends_with("test.rs") {
        file.set_is_test(true);
    }
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

    fn structure(word: &str, start: u32, end: u32) -> SymbolDefinition {
        let mut s = SymbolDefinition::new();
        s.set_symbol(word.to_string());
        s.set_line_number(start);
        s.set_end_line_number(end);
        s.set_symbol_type(SymbolType::STRUCTURE);
        s
    }

    fn function(word: &str, start: u32, end: u32) -> SymbolDefinition {
        let mut s = SymbolDefinition::new();
        s.set_symbol(word.to_string());
        s.set_line_number(start);
        s.set_end_line_number(end);
        s.set_symbol_type(SymbolType::FUNCTION);
        s
    }

    fn _trait(word: &str, start: u32, end: u32) -> SymbolDefinition {
        let mut s = SymbolDefinition::new();
        s.set_symbol(word.to_string());
        s.set_line_number(start);
        s.set_end_line_number(end);
        s.set_symbol_type(SymbolType::TRAIT);
        s
    }

    #[test]
    fn test_extract_trait_impl() {
        let mut f = File::new();
        f.set_content(
            "
impl<T: std::clone::Clone + std::str::FromStr> Flag<T> {
    pub fn parse(&self, value: &str) -> Result<T, Error> {
        match value.parse() {
            Ok(x) => Ok(x),
            Err(_) => Err(Error::new(
                ErrorKind::InvalidData,
            )),
        }
    }
}

impl Flag<String> {
    pub fn path(&self) -> String {
        if value.starts_with() {
            Err(_) => ()
        };
    }
}
            "
            .into(),
        );

        let extracted = extract_definitions(&f);

        assert_eq!(extracted.len(), 4);
        assert_eq!(&extracted[0], &structure("Flag", 1, 10));
        assert_eq!(&extracted[1], &function("parse", 2, 9));
        assert_eq!(&extracted[2], &structure("Flag", 12, 18));
        assert_eq!(&extracted[3], &function("path", 13, 17));
    }

    #[test]
    fn test_extract_struct() {
        let mut f = File::new();
        f.set_content(
            "
    pub struct ExtractKeywordsFn {
        data: u64,
    }

    #[derive(Clone)]
    pub struct Flag<T: std::str::FromStr> {
        pub name: &'static str,
        pub usage: &'static str,
        pub default: T,
    }

    pub trait ParseableFlag {
        fn validate(&self, &str) -> Result<(), Error>;
        fn get_usage_string(&self) -> &str;
        fn get_default_value(&self) -> String;
    }

    pub fn create_task(
        &self,
        mut req: tasks_grpc_rust::CreateTaskRequest,
    ) -> tasks_grpc_rust::TaskStatus {
        let mut initial_status = TaskStatus::new();
        initial_status.set_name(req.take_task_name());
        initial_status.set_arguments(req.take_arguments());
        let id = self.client.reserve_task_id();
        initial_status.set_task_id(id.clone());

        let info_url = format!(\"{}/{}\", self.config.base_url, id);
        initial_status.set_info_url(info_url);

        self.client.write(&initial_status);
        self.scheduler.unbounded_send(id);
        initial_status
    }
    "
            .into(),
        );

        let extracted = extract_definitions(&f);

        assert_eq!(extracted.len(), 10);
        assert_eq!(&extracted[0], &structure("ExtractKeywordsFn", 1, 3));
        assert_eq!(&extracted[1], &structure("Flag", 6, 10));
        assert_eq!(&extracted[2], &_trait("ParseableFlag", 12, 16));
        assert_eq!(&extracted[3], &function("validate", 13, 0));
        assert_eq!(&extracted[4], &function("get_usage_string", 14, 0));
        assert_eq!(&extracted[5], &function("get_default_value", 15, 0));
        assert_eq!(&extracted[6], &function("create_task", 18, 34));
    }

    #[test]
    fn test_extract_keywords() {
        let mut f = File::new();
        f.set_content(
            "
   pub fn create_task(
        &self,
        mut req: tasks_grpc_rust::CreateTaskRequest,
    ) -> tasks_grpc_rust::TaskStatus {
        let mut initial_status = TaskStatus::new();
        initial_status.set_name(req.take_task_name());
        initial_status.set_arguments(req.take_arguments());
        let id = self.client.reserve_task_id();
        initial_status.set_task_id(id.clone());

        let info_url = format!(\"{}/{}\", self.config.base_url, id);
        initial_status.set_info_url(info_url);

        self.client.write(&initial_status);
        self.scheduler.unbounded_send(id);
        initial_status
    }"
            .into(),
        );

        let extracted = extract_keywords(&f);

        assert_eq!(extracted.len(), 23);
        assert_eq!(&extracted[0], &kw("CreateTaskRequest", 1));
    }

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
    fn test_extract_definitions() {
        let mut f = File::new();
        f.set_content(
            "
   pub fn create_task(
        &self,
        mut req: tasks_grpc_rust::CreateTaskRequest,
    ) -> tasks_grpc_rust::TaskStatus {
        let mut initial_status = TaskStatus::new();
        initial_status.set_name(req.take_task_name());
        initial_status.set_arguments(req.take_arguments());
        let id = self.client.reserve_task_id();
        initial_status.set_task_id(id.clone());

        let info_url = format!(\"{}/{}\", self.config.base_url, id);
        initial_status.set_info_url(info_url);

        self.client.write(&initial_status);
        self.scheduler.unbounded_send(id);
        initial_status
    }"
            .into(),
        );

        let extracted = extract_definitions(&f);
        let expected = vec![
            test_fn("create_task", 1, 17),
            test_var("initial_status", 5),
            test_var("id", 8),
            test_var("info_url", 11),
        ];
        assert_eq!(extracted, expected);
    }
}
