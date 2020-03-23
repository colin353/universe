#[macro_use]
extern crate lazy_static;

use search_grpc_rust::*;
use std::collections::HashSet;
use std::sync::Mutex;

const CANDIDATES_TO_RETURN: usize = 25;
const CANDIDATES_TO_EXPAND: usize = 100;
const MAX_LINE_LENGTH: usize = 144;
const SNIPPET_LENGTH: usize = 7;

lazy_static! {
    static ref KEYWORDS_RE: regex::Regex = { regex::Regex::new(r"(\w+)").unwrap() };
    static ref DEFINITION_RE: regex::Regex = { regex::Regex::new(r"def:(\w+)").unwrap() };
}

pub struct Searcher {
    keywords: Mutex<sstable::SSTableReader<KeywordMatches>>,
    code: Mutex<sstable::SSTableReader<File>>,
    definitions: Mutex<sstable::SSTableReader<DefinitionMatches>>,
    filenames: Vec<String>,

    // Configuration options
    pub candidates_to_return: usize,
    pub candidates_to_expand: usize,
}

impl Searcher {
    pub fn new(base_dir: &str) -> Self {
        let keywords =
            sstable::SSTableReader::from_filename(&format!("{}/keywords.sstable", base_dir))
                .unwrap();
        let mut code =
            sstable::SSTableReader::from_filename(&format!("{}/files.sstable", base_dir)).unwrap();
        let mut definitions =
            sstable::SSTableReader::from_filename(&format!("{}/definitions.sstable", base_dir))
                .unwrap();

        let mut filenames = Vec::new();
        for (filename, _) in &mut code {
            filenames.push(filename);
        }

        Self {
            code: Mutex::new(code),
            keywords: Mutex::new(keywords),
            definitions: Mutex::new(definitions),
            candidates_to_return: CANDIDATES_TO_RETURN,
            candidates_to_expand: CANDIDATES_TO_EXPAND,
            filenames: filenames,
        }
    }

    pub fn search(&self, keywords: &str) -> Vec<Candidate> {
        let query = self.parse_query(keywords);
        let mut candidates = Vec::new();
        self.get_candidates(&query, &mut candidates);
        self.deduplicate(&mut candidates);
        self.initial_rank(&query, &mut candidates);
        self.cutoff(&mut candidates, self.candidates_to_expand);
        self.expand_candidates(&query, &mut candidates);
        self.final_rank(&query, &mut candidates);
        self.cutoff(&mut candidates, self.candidates_to_return);
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
        for captures in DEFINITION_RE.captures_iter(query) {
            out.set_definition(captures[1].to_owned());
        }
        out
    }

    fn get_candidates(&self, query: &Query, candidates: &mut Vec<Candidate>) {
        // If we are doing a definition search, only extract definition candidates
        if query.get_definition().len() > 0 {
            self.get_candidates_by_definition(query, candidates);
            return;
        }

        self.get_candidates_by_keyword(query, candidates);
        self.get_candidates_by_filename(query, candidates);
    }

    fn get_candidates_by_definition(&self, query: &Query, candidates: &mut Vec<Candidate>) {
        let mut matches = match self
            .definitions
            .lock()
            .unwrap()
            .get(&normalize_keyword(query.get_definition()))
            .unwrap()
        {
            Some(s) => s,
            None => DefinitionMatches::new(),
        };
        for mut m in matches.take_matches().into_iter() {
            let mut c = Candidate::new();
            c.mut_matched_definitions().push(m.clone());
            c.set_filename(m.take_filename());
            candidates.push(c);
        }
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

        // Check symbol definitions as well
        let mut defs = std::collections::HashMap::new();
        for keyword in query.get_keywords() {
            let mut matches = match self
                .definitions
                .lock()
                .unwrap()
                .get(&normalize_keyword(keyword))
                .unwrap()
            {
                Some(s) => s,
                None => DefinitionMatches::new(),
            };
            for m in matches.take_matches().into_iter() {
                if or_set.as_ref().unwrap().contains(m.get_filename()) {
                    defs.insert(m.get_filename().to_owned(), m);
                }
            }
        }

        for mut candidate in all_candidates {
            if or_set.as_mut().unwrap().contains(candidate.get_filename()) {
                if let Some(symbol_def) = defs.get(candidate.get_filename()) {
                    candidate
                        .mut_matched_definitions()
                        .push(symbol_def.to_owned().to_owned());
                }

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
            None => KeywordMatches::new(),
        };
        let normalized_keyword = normalize_keyword(keyword);
        let mut normalized_matches = match self
            .keywords
            .lock()
            .unwrap()
            .get(&normalized_keyword)
            .unwrap()
        {
            Some(s) => s,
            None => KeywordMatches::new(),
        };

        for mut m in matches
            .take_matches()
            .into_iter()
            .chain(normalized_matches.take_matches().into_iter())
        {
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
            k.set_normalized(m.get_normalized());
            c.mut_matched_keywords().push(k);

            candidates.push(c);
        }
    }

    fn cutoff(&self, candidates: &mut Vec<Candidate>, num_candidates: usize) {
        candidates.truncate(num_candidates);
    }

    fn initial_rank(&self, query: &Query, candidates: &mut Vec<Candidate>) {
        for mut candidate in candidates.iter_mut() {
            self.score(query, &mut candidate);
        }

        candidates.sort_by(|a, b| {
            b.get_score()
                .partial_cmp(&a.get_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    fn final_rank(&self, query: &Query, candidates: &mut Vec<Candidate>) {
        for mut candidate in candidates.iter_mut() {
            self.fullscore(query, &mut candidate);
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

            // Penalty for non-exact keyword match
            if kw.get_normalized() {
                score -= 3.0;
            }
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

        if candidate.get_filename().starts_with("third_party") {
            score /= 2.0;
        }

        // Definition scoring
        // TODO: adjust score based on symbol type
        score += 50.0 * candidate.get_matched_definitions().len() as f32;
        for def in candidate.get_matched_definitions() {
            for keyword in query.get_keywords() {
                if def.get_symbol() == keyword {
                    // Exact match, give an extra 50 points
                    score += 50.0;
                }
            }
        }

        score +=
            candidate.get_filename_match_position() as f32 / candidate.get_filename().len() as f32;

        candidate.set_score(score);
    }

    fn fullscore(&self, query: &Query, candidate: &mut Candidate) {
        if candidate.get_is_ugly() {
            candidate.set_score(candidate.get_score() / 10.0);
        }
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

    pub fn expand_candidates(&self, query: &Query, candidates: &mut Vec<Candidate>) {
        for candidate in candidates.iter_mut() {
            self.expand_candidate(query, candidate);
        }
    }

    pub fn expand_candidate(&self, query: &Query, candidate: &mut Candidate) {
        let doc = match self.get_document(candidate.get_filename()) {
            Some(d) => d,
            None => return,
        };

        candidate.set_is_ugly(doc.get_is_ugly());
        candidate.set_file_type(doc.get_file_type());

        let mut spans = Vec::new();
        for (line_number, line) in doc.get_content().lines().enumerate() {
            let line = line.to_lowercase();

            for keyword in query.get_keywords() {
                let mut s = extract_spans(keyword, &line);
                for span in s.iter_mut() {
                    span.set_line(line_number as u64);
                }
                spans.append(&mut s);
            }
        }

        let window_start = if candidate.get_matched_definitions().len() > 0 {
            let mut line_number = candidate.get_matched_definitions()[0].get_line_number() as usize;
            candidate.set_jump_to_line(line_number as u32);

            if line_number > SNIPPET_LENGTH / 2 {
                line_number -= SNIPPET_LENGTH / 2;
            } else {
                line_number = 0;
            }
            line_number
        } else {
            let n = find_max_span_window(spans);
            candidate.set_jump_to_line(n as u32);
            n
        };

        candidate.set_snippet_starting_line(window_start as u32);

        let mut started = false;
        for line in doc
            .get_content()
            .lines()
            .skip(window_start)
            .take(SNIPPET_LENGTH)
        {
            if !started && line.trim().is_empty() {
                continue;
            }
            let mut snippet = line.to_string();
            if let Some((idx, _)) = snippet.char_indices().nth(MAX_LINE_LENGTH) {
                snippet.truncate(idx);
                candidate.mut_snippet().push(snippet);
            } else {
                candidate.mut_snippet().push(snippet);
            }
        }
    }
}

fn find_max_span_window(spans: Vec<Span>) -> usize {
    if spans.len() == 0 {
        return 0;
    }

    // Find the highest density of spans within a window
    let mut max_window_start = 0;
    let mut window_start = 0;
    let mut max_spans = 0;
    let mut included_spans = std::collections::VecDeque::new();
    let mut span_iter = spans.iter();
    for span in spans {
        if (span.get_line() as usize) < window_start + SNIPPET_LENGTH {
            included_spans.push_back(span.get_line() as usize);
        } else {
            window_start = span.get_line() as usize - SNIPPET_LENGTH;
            included_spans.push_back(span.get_line() as usize);
            while let Some(s) = included_spans.front() {
                if *s < window_start {
                    included_spans.pop_front();
                } else {
                    break;
                }
            }
        }

        if included_spans.len() > max_spans {
            max_spans = included_spans.len();

            // Adjust the window to be centered: let's find the max and min included spans and set
            // the window to center on that position.
            let max = *included_spans.iter().max().unwrap();
            let min = *included_spans.iter().min().unwrap();
            let offset = (SNIPPET_LENGTH - (max - min)) / 2;

            if min > offset {
                max_window_start = min - offset;
            } else {
                max_window_start = 0;
            }
        }
    }

    max_window_start
}

fn extract_spans(term: &str, line: &str) -> Vec<Span> {
    let mut output = Vec::new();
    for (idx, _) in line.match_indices(term) {
        let mut s = Span::new();
        s.set_offset(idx as u64);
        s.set_length(term.len() as u64);
        output.push(s);
    }
    output
}

fn normalize_keyword(keyword: &str) -> String {
    let mut normalized_keyword = keyword.to_lowercase();
    normalized_keyword.retain(|c| c != '_' && c != '-');
    normalized_keyword
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
