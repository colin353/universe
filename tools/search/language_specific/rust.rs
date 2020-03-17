use search_proto_rust::*;

static MIN_KEYWORD_LENGTH: usize = 3;

lazy_static! {
    static ref KEYWORDS_RE: regex::Regex = { regex::Regex::new(r"(\w+)").unwrap() };
    static ref FUNCTION_DEFINITION: regex::Regex =
        { regex::Regex::new(r"\s+(pub)?\s*fn\s+(\w+)").unwrap() };
    static ref STRUCTURE_DEFINITION: regex::Regex =
        { regex::Regex::new(r"\s+(pub)?\s*struct\s+(\w+)").unwrap() };
    static ref LET_BINDING: regex::Regex =
        { regex::Regex::new(r"\s+let\s*(mut)?\s+(\w+)").unwrap() };
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
        for captures in STRUCTURE_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::STRUCTURE);
            results.push(d);
        }
        for captures in FUNCTION_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::FUNCTION);
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
            test_fn("create_task", 1),
            test_var("initial_status", 5),
            test_var("id", 8),
            test_var("info_url", 11),
        ];
        assert_eq!(extracted, expected);
    }
}
