use search_proto_rust::*;

static MIN_KEYWORD_LENGTH: usize = 3;

lazy_static! {
    static ref KEYWORDS_RE: regex::Regex = { regex::Regex::new(r"(\w+)").unwrap() };
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
}
