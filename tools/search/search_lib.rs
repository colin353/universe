use search_proto_rust::*;

pub struct Searcher {
    keywords: sstable::SSTableReader<KeywordMatches>,
}

impl Searcher {
    pub fn new(base_dir: &str) -> Self {
        let keywords =
            sstable::SSTableReader::from_filename(&format!("{}/keywords.sstable", base_dir))
                .unwrap();

        Self { keywords: keywords }
    }

    pub fn search(&mut self, keywords: &str) -> Vec<Candidate> {
        let mut candidates = self.get_candidates(keywords);
        self.deduplicate(&mut candidates);
        self.render_results(&candidates);
        return candidates;
    }

    fn get_candidates(&mut self, keyword: &str) -> Vec<Candidate> {
        let mut matches = match self.keywords.get(keyword).unwrap() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut candidates = Vec::new();
        for mut m in matches.take_matches().into_iter() {
            let mut c = Candidate::new();
            c.set_filename(m.take_filename());

            let mut k = ExtractedKeyword::new();
            k.set_keyword(keyword.to_owned());
            k.set_occurrences(m.get_occurrences());
            c.mut_matched_keywords().push(k);

            candidates.push(c);
        }

        candidates
    }

    fn deduplicate(&self, candidates: &mut Vec<Candidate>) {
        let mut observed = std::collections::HashMap::<String, usize>::new();
        let mut index = 0;
        while index < candidates.len() {
            if let Some(observed_idx) = observed.get(candidates[index].get_filename()) {
                for mkw in candidates[index].take_matched_keywords().into_iter() {
                    candidates[*observed_idx].mut_matched_keywords().push(mkw);
                }
                candidates.swap_remove(index);
            } else {
                observed.insert(candidates[index].get_filename().to_owned(), index);
                index += 1;
            }
        }
    }

    pub fn render_results(&self, results: &[Candidate]) {
        if results.len() == 0 {
            println!("No results!");
        }

        for (idx, candidate) in results.iter().enumerate() {
            println!("{}. {}", idx + 1, candidate.get_filename());
        }
    }
}
