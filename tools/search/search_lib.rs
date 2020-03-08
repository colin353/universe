#[macro_use]
extern crate lazy_static;

use search_proto_rust::*;
use std::collections::HashSet;
use std::sync::Mutex;

const CANDIDATES_TO_RETURN: usize = 25;

lazy_static! {
    static ref KEYWORDS_RE: regex::Regex = { regex::Regex::new(r"(\w+)").unwrap() };
}

pub struct Searcher {
    keywords: Mutex<sstable::SSTableReader<KeywordMatches>>,
    code: Mutex<sstable::SSTableReader<File>>,
    filenames: Vec<String>,

    // Configuration options
    pub candidates_to_return: usize,
}

impl Searcher {
    pub fn new(base_dir: &str) -> Self {
        let keywords =
            sstable::SSTableReader::from_filename(&format!("{}/keywords.sstable", base_dir))
                .unwrap();
        let mut code =
            sstable::SSTableReader::from_filename(&format!("{}/code.sstable", base_dir)).unwrap();

        let mut filenames = Vec::new();
        for (filename, _) in &mut code {
            filenames.push(filename);
        }

        Self {
            code: Mutex::new(code),
            keywords: Mutex::new(keywords),
            candidates_to_return: CANDIDATES_TO_RETURN,
            filenames: filenames,
        }
    }

    pub fn search(&self, keywords: &str) -> Vec<Candidate> {
        let query = self.parse_query(keywords);
        let mut candidates = Vec::new();
        self.get_candidates(&query, &mut candidates);
        self.deduplicate(&mut candidates);
        self.rank(&query, &mut candidates);
        self.cutoff(&mut candidates);
        self.render_results(&candidates);
        return candidates;
    }

    pub fn get_document(&self, filename: &str) -> Option<File> {
        self.code.lock().unwrap().get(filename).unwrap()
    }

    fn parse_query(&self, query: &str) -> Query {
        let mut out = Query::new();
        out.set_query(query.to_owned());
        for captures in KEYWORDS_RE.captures_iter(query) {
            let keyword = &captures[0];
            out.mut_keywords().push(keyword.to_owned());
        }
        out
    }

    fn get_candidates(&self, query: &Query, candidates: &mut Vec<Candidate>) {
        self.get_candidates_by_keyword(query, candidates);
        self.get_candidates_by_filename(query, candidates);
    }

    fn get_candidates_by_filename(&self, query: &Query, candidates: &mut Vec<Candidate>) {
        for filename in &self.filenames {
            let mut matched = true;
            let mut query_match = false;
            let mut exact_match = false;
            let mut match_position = 0;
            for keyword in query.get_keywords() {
                if let Some(idx) = filename.rfind(keyword) {
                    match_position = std::cmp::max(match_position, idx + keyword.len());
                } else {
                    matched = false;
                }
            }

            if let Some(idx) = filename.rfind(query.get_query()) {
                matched = true;
                query_match = true;

                match_position = std::cmp::max(match_position, idx + query.get_query().len());

                if filename == query.get_query() {
                    exact_match = true;
                }
            }

            if matched {
                println!("match! query: {:?}", query);
                println!("filename: {:?}", filename);
                println!(
                    "matches: {} {} {}",
                    query_match, exact_match, match_position
                );

                let mut c = Candidate::new();
                c.set_filename(filename.to_owned());
                c.set_keyword_matched_filename(true);
                c.set_query_in_filename(query_match);
                c.set_exactly_matched_filename(exact_match);
                c.set_filename_match_position(match_position as u32);
                candidates.push(c);
            }
        }
    }

    fn get_candidates_by_keyword(&self, query: &Query, candidates: &mut Vec<Candidate>) {
        let mut or_set = None;
        let mut all_candidates = Vec::new();
        for keyword in query.get_keywords() {
            let mut these_candidates = Vec::new();
            self.get_candidates_matching_keyword(keyword, &or_set, &mut these_candidates);

            or_set = Some(HashSet::new());

            for candidate in &these_candidates {
                or_set
                    .as_mut()
                    .unwrap()
                    .insert(candidate.get_filename().to_owned());
            }
            all_candidates.append(&mut these_candidates);
        }

        if or_set.is_none() {
            return;
        }

        for candidate in all_candidates {
            if or_set.as_mut().unwrap().contains(candidate.get_filename()) {
                candidates.push(candidate);
            }
        }
    }

    fn get_candidates_matching_keyword(
        &self,
        keyword: &str,
        or_set: &Option<HashSet<String>>,
        candidates: &mut Vec<Candidate>,
    ) {
        let mut matches = match self.keywords.lock().unwrap().get(keyword).unwrap() {
            Some(s) => s,
            None => return,
        };

        for mut m in matches.take_matches().into_iter() {
            if let Some(ref set) = or_set {
                if !set.contains(m.get_filename()) {
                    continue;
                }
            }

            let mut c = Candidate::new();
            c.set_filename(m.take_filename());

            let mut k = ExtractedKeyword::new();
            k.set_keyword(keyword.to_owned());
            k.set_occurrences(m.get_occurrences());
            c.mut_matched_keywords().push(k);

            candidates.push(c);
        }
    }

    fn cutoff(&self, candidates: &mut Vec<Candidate>) {
        candidates.truncate(self.candidates_to_return);
    }

    fn rank(&self, query: &Query, candidates: &mut Vec<Candidate>) {
        for mut candidate in candidates.iter_mut() {
            self.score(query, &mut candidate);
        }

        candidates.sort_by(|a, b| {
            b.get_score()
                .partial_cmp(&a.get_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    fn score(&self, query: &Query, candidate: &mut Candidate) {
        let mut score = candidate.get_score();

        // Keyword match scoring
        for kw in candidate.get_matched_keywords() {
            score += 10.0;
            score += 0.1 * std::cmp::min(kw.get_occurrences(), 10) as f32;
        }

        // Filename match scoring
        if candidate.get_keyword_matched_filename() {
            score += 10.0;
        }

        if candidate.get_query_in_filename() {
            score += 20.0;
        }

        if candidate.get_exactly_matched_filename() {
            score += 100.0;
        }

        score +=
            candidate.get_filename_match_position() as f32 / candidate.get_filename().len() as f32;

        candidate.set_score(score);
    }

    fn deduplicate(&self, candidates: &mut Vec<Candidate>) {
        let mut observed = std::collections::HashMap::<String, usize>::new();
        let mut index = 0;
        while index < candidates.len() {
            if let Some(observed_idx) = observed.get(candidates[index].get_filename()) {
                let to = candidates.swap_remove(index);
                merge_candidates(to, &mut candidates[*observed_idx])
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
            println!(
                "{}. (score={}) {}",
                idx + 1,
                candidate.get_score(),
                candidate.get_filename()
            );
        }
    }
}

fn merge_candidates(mut from: Candidate, to: &mut Candidate) {
    for mkw in from.take_matched_keywords().into_iter() {
        to.mut_matched_keywords().push(mkw);
    }

    to.set_keyword_matched_filename(
        to.get_keyword_matched_filename() || from.get_keyword_matched_filename(),
    );
    to.set_query_in_filename(to.get_query_in_filename() || from.get_query_in_filename());
    to.set_exactly_matched_filename(
        to.get_exactly_matched_filename() || from.get_exactly_matched_filename(),
    );
    to.set_filename_match_position(std::cmp::max(
        from.get_filename_match_position(),
        to.get_filename_match_position(),
    ));
}
