#[macro_use]
extern crate lazy_static;

use search_grpc_rust::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

const CANDIDATES_TO_RETURN: usize = 25;
const CANDIDATES_TO_EXPAND: usize = 100;
const MAX_LINE_LENGTH: usize = 144;
const SNIPPET_LENGTH: usize = 7;

lazy_static! {
    static ref KEYWORDS_RE: regex::Regex = { regex::Regex::new(r#"("(.*?)")|([^\s]+)"#).unwrap() };
    static ref DEFINITION_RE: regex::Regex = { regex::Regex::new(r"def:(\w+)").unwrap() };
}

pub struct Searcher {
    code: Mutex<sstable::SSTableReader<File>>,
    candidates: Arc<Mutex<sstable::SSTableReader<File>>>,
    definitions: Mutex<sstable::SSTableReader<DefinitionMatches>>,
    trigrams: Mutex<sstable::SSTableReader<KeywordMatches>>,

    files: HashMap<u64, File>,

    // Configuration options
    pub candidates_to_return: usize,
    pub candidates_to_expand: usize,
}

impl Searcher {
    pub fn new(base_dir: &str) -> Self {
        let mut code =
            sstable::SSTableReader::from_filename(&format!("{}/files.sstable", base_dir)).unwrap();
        let mut definitions =
            sstable::SSTableReader::from_filename(&format!("{}/definitions.sstable", base_dir))
                .unwrap();
        let mut candidates =
            sstable::SSTableReader::from_filename(&format!("{}/candidates.sstable", base_dir))
                .unwrap();
        let mut trigrams =
            sstable::SSTableReader::from_filename(&format!("{}/trigrams.sstable", base_dir))
                .unwrap();

        let mut files = HashMap::<u64, File>::new();
        for (key, file) in &mut candidates {
            files.insert(key.parse::<u64>().unwrap(), file);
        }

        Self {
            code: Mutex::new(code),
            definitions: Mutex::new(definitions),
            candidates: Arc::new(Mutex::new(candidates)),
            trigrams: Mutex::new(trigrams),
            candidates_to_return: CANDIDATES_TO_RETURN,
            candidates_to_expand: CANDIDATES_TO_EXPAND,
            files: files,
        }
    }

    pub fn search(&self, keywords: &str) -> Vec<Candidate> {
        let query = self.parse_query(keywords);
        let start = std::time::Instant::now();
        let mut candidates = self.get_candidates(&query);
        self.initial_rank(&query, &mut candidates);
        self.cutoff(&mut candidates, self.candidates_to_expand);
        self.expand_candidates(&query, &mut candidates);
        self.final_rank(&query, &mut candidates);
        self.cutoff(&mut candidates, self.candidates_to_return);
        self.render_results(&candidates);
        println!("total search: {} ms", start.elapsed().as_millis());
        return candidates;
    }

    pub fn get_document(&self, filename: &str) -> Option<File> {
        self.code.lock().unwrap().get(filename).unwrap()
    }

    fn parse_query(&self, query: &str) -> Query {
        let mut out = Query::new();
        out.set_query(query.to_owned());
        for captures in KEYWORDS_RE.captures_iter(query) {
            let mut kw = captures.get(3);
            if kw.is_none() {
                kw = captures.get(2);
            }
            if kw.is_none() {
                kw = captures.get(1);
            }
            let mut keyword = QueryKeyword::new();
            keyword.set_keyword(kw.unwrap().as_str().to_owned());

            // Support definition search
            if keyword.get_keyword().starts_with("def:") {
                keyword.set_keyword(keyword.get_keyword()[4..].to_owned());
                keyword.set_is_definition(true);
            }

            // Support prefix search
            if keyword.get_keyword().starts_with("in:") {
                keyword.set_keyword(keyword.get_keyword()[3..].to_owned());
                keyword.set_is_prefix(true);
            }

            out.mut_keywords().push(keyword);
        }

        out
    }

    fn get_candidates(&self, query: &Query) -> Vec<Candidate> {
        let mut candidates = HashMap::new();
        self.get_candidates_by_definition(query, &mut candidates);
        self.get_candidates_by_filename(query, &mut candidates);
        self.get_possible_candidates_by_keyword(query, &mut candidates);
        self.eliminate_partially_matched_candidates(query, &mut candidates);
        self.finalize_keyword_matches(query, candidates)
    }

    fn get_candidates_by_definition(
        &self,
        query: &Query,
        candidates: &mut HashMap<u64, Candidate>,
    ) {
        for (index, keyword) in query
            .get_keywords()
            .iter()
            .enumerate()
            // Prefix requirements should not match on definitions
            .filter(|(_, k)| !k.get_is_prefix())
        {
            let mut matches = match self
                .definitions
                .lock()
                .unwrap()
                .get(&search_utils::normalize_keyword(keyword.get_keyword()))
                .unwrap()
            {
                Some(s) => s,
                None => DefinitionMatches::new(),
            };
            for mut m in matches.take_matches().into_iter() {
                let c = candidates
                    .entry(search_utils::hash_filename(m.get_filename()))
                    .or_insert(Candidate::new());

                c.mut_matched_definitions().push(m.clone());
                c.set_filename(m.take_filename());
                c.set_keyword_definite_match_mask(update_mask(
                    c.get_keyword_definite_match_mask(),
                    index,
                ));
            }
        }
    }

    fn get_candidates_by_filename(&self, query: &Query, candidates: &mut HashMap<u64, Candidate>) {
        // Construct regex matchers for each keyword
        let keyword_matchers: Vec<_> = query
            .get_keywords()
            .iter()
            .map(|k| {
                (
                    k.clone(),
                    aho_corasick::AhoCorasickBuilder::new()
                        .ascii_case_insensitive(true)
                        .build(&[k.get_keyword()]),
                )
            })
            .collect();

        let query_matcher = aho_corasick::AhoCorasickBuilder::new()
            .ascii_case_insensitive(true)
            .build(&[query.get_query()]);

        for file in self.files.values() {
            let filename = file.get_filename();
            let mut query_match = false;
            let mut exact_match = false;
            let mut match_position = 0;
            let mut match_mask = 0;
            let mut num_matches = 0;
            let mut matched_in_order = true;
            let mut filename_coverage = 0;
            for (index, (keyword, re)) in keyword_matchers.iter().enumerate() {
                if let Some(m) = re.find(filename) {
                    if keyword.get_is_definition() {
                        continue;
                    }

                    if keyword.get_is_prefix() && m.start() != 0 {
                        continue;
                    }

                    if m.end() < match_position {
                        matched_in_order = false;
                    }
                    num_matches += 1;
                    filename_coverage += m.end() - m.start();
                    match_position = std::cmp::max(match_position, m.end());
                    match_mask = update_mask(match_mask, index);
                }
            }

            if let Some(m) = query_matcher.find(filename) {
                query_match = true;
                num_matches += 1;

                // It matched all components of the query so just fill the match mask with 1s
                match_mask = std::u32::MAX;

                match_position = std::cmp::max(match_position, m.end());

                if filename == query.get_query() {
                    exact_match = true;
                }
            }

            if match_mask > 0 {
                let c = candidates
                    .entry(search_utils::hash_filename(filename))
                    .or_insert(Candidate::new());
                c.set_filename(filename.to_owned());
                c.set_keyword_matched_filename(true);
                c.set_query_in_filename(query_match);
                c.set_exactly_matched_filename(exact_match);
                c.set_filename_match_position(match_position as u32);
                c.set_filename_query_matches(num_matches);
                c.set_filename_keywords_matched_in_order(matched_in_order);
                c.set_keyword_definite_match_mask(c.get_keyword_definite_match_mask() | match_mask);
                c.set_filename_match_coverage(filename_coverage as f32 / filename.len() as f32);
            }
        }
    }

    fn get_possible_candidates_by_keyword(
        &self,
        query: &Query,
        candidates: &mut HashMap<u64, Candidate>,
    ) {
        let mut short_keyword_mask: u32 = 0;

        for (index, keyword) in query
            .get_keywords()
            .iter()
            .enumerate()
            .filter(|(_, k)| !k.get_is_prefix())
        {
            // We use a trigram index. If this keyword has fewer than 3 chars, just assume
            // any candidate might match it.
            if keyword.get_keyword().len() < 3 {
                short_keyword_mask = update_mask(short_keyword_mask, index);
            }

            let mut or_set: Option<HashSet<u64>> = None;
            for trigram in search_utils::trigrams(&keyword.get_keyword().to_lowercase()) {
                let mut matches = HashSet::new();

                let mut results = match self.trigrams.lock().unwrap().get(&trigram).unwrap() {
                    Some(s) => s,
                    None => KeywordMatches::new(),
                };
                for file_id in results.get_matches() {
                    if let Some(o) = or_set.as_ref() {
                        if o.contains(file_id) {
                            matches.insert(*file_id);
                        }
                    } else {
                        matches.insert(*file_id);
                    }
                }

                or_set = Some(matches);
            }

            if let Some(o) = or_set.as_ref() {
                for file_id in o.iter() {
                    let c = candidates.entry(*file_id).or_insert(Candidate::new());
                    c.set_keyword_possible_match_mask(update_mask(
                        c.get_keyword_possible_match_mask(),
                        index,
                    ));
                }
            }
        }

        // Make sure that all short keywords are marked as "possible matches" in all candidates.
        for (_, candidate) in candidates.iter_mut() {
            candidate.set_keyword_possible_match_mask(
                candidate.get_keyword_possible_match_mask() | short_keyword_mask,
            );
        }
    }

    fn eliminate_partially_matched_candidates(
        &self,
        query: &Query,
        candidates: &mut HashMap<u64, Candidate>,
    ) {
        let target_match_mask = (1 << (query.get_keywords().len())) - 1;

        // Eliminate all candidates that didn't possibly match all keywords
        candidates.retain(|_, c| {
            let match_mask =
                c.get_keyword_definite_match_mask() | c.get_keyword_possible_match_mask();

            match_mask >= target_match_mask
        });
    }

    fn finalize_keyword_matches(
        &self,
        query: &Query,
        candidates: HashMap<u64, Candidate>,
    ) -> Vec<Candidate> {
        let mut scanned = 0;

        // Construct regex matchers for each keyword
        let keyword_matchers: Vec<_> = query
            .get_keywords()
            .iter()
            .map(|k| {
                (
                    k.clone(),
                    aho_corasick::AhoCorasickBuilder::new()
                        .ascii_case_insensitive(true)
                        .build(&[k.get_keyword()]),
                )
            })
            .collect();

        let pool = pool::ThreadPool::new(4);

        for (file_id, candidate) in candidates.into_iter() {
            let matchers = keyword_matchers.clone();
            let reader = self.candidates.clone();
            pool.execute(move || {
                Self::finalize_keyword_matches_for_candidate(reader, file_id, candidate, matchers)
            });
            scanned += 1;
        }
        println!("fully scanned {} candidates", scanned);

        let mut candidates = pool.join();
        let target_match_mask = (1 << (query.get_keywords().len())) - 1;
        candidates.retain(|c| c.get_keyword_definite_match_mask() >= target_match_mask);
        candidates
    }

    fn finalize_keyword_matches_for_candidate(
        reader: Arc<Mutex<sstable::SSTableReader<File>>>,
        file_id: u64,
        mut candidate: Candidate,
        keyword_matchers: Vec<(QueryKeyword, aho_corasick::AhoCorasick)>,
    ) -> Candidate {
        let start = std::time::Instant::now();
        let mut file = {
            reader
                .lock()
                .unwrap()
                .get(&file_id.to_string())
                .unwrap()
                .unwrap()
        };

        candidate.set_filename(file.get_filename().to_string());
        candidate.set_is_ugly(file.get_is_ugly());
        candidate.set_file_type(file.get_file_type());
        candidate.set_child_files(file.take_child_files());
        candidate.set_child_directories(file.take_child_directories());
        candidate.set_is_directory(file.get_is_directory());

        let start = std::time::Instant::now();
        let mut matched_keywords = HashMap::new();
        for (index, (keyword, re)) in keyword_matchers.iter().enumerate() {
            for (line_number, line) in file.get_content().lines().enumerate() {
                let mut s = extract_spans(&re, &line);

                if s.len() == 0 {
                    continue;
                }

                for mut span in s.into_iter() {
                    span.set_line(line_number as u64);
                    candidate.mut_spans().push(span);
                }

                let k = matched_keywords.entry(index).or_insert_with(|| {
                    let mut e = ExtractedKeyword::new();
                    e.set_keyword(keyword.get_keyword().to_owned());
                    e
                });
                k.set_occurrences(k.get_occurrences() + 1);

                if k.get_occurrences() > 20 {
                    break;
                }
            }
        }
        if start.elapsed().as_millis() > 10 {
            println!(
                "scanned {} in {} us",
                file.get_filename(),
                start.elapsed().as_micros()
            );
        }

        for (index, k) in matched_keywords.into_iter() {
            candidate.mut_matched_keywords().push(k);
            candidate.set_keyword_definite_match_mask(update_mask(
                candidate.get_keyword_definite_match_mask(),
                index,
            ));
        }

        candidate
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
            score += 200.0 * candidate.get_filename_match_coverage();
            score += 40.0 * candidate.get_filename_query_matches() as f32;
        }
        score +=
            candidate.get_filename_match_position() as f32 / candidate.get_filename().len() as f32;

        if candidate.get_query_in_filename() {
            if candidate.get_filename_keywords_matched_in_order() {
                score += 20.0 * candidate.get_filename_query_matches() as f32;
            }
        }

        if candidate.get_exactly_matched_filename() {
            score += 100.0;
        }

        if candidate.get_filename().starts_with("third_party") {
            score /= 3.0;
        }

        // Definition scoring
        // TODO: adjust score based on symbol type
        let mut definition_score = 0;
        for def in candidate.get_matched_definitions() {
            let symbol_score = match def.get_symbol_type() {
                SymbolType::VARIABLE => 5,
                SymbolType::FUNCTION => 40,
                SymbolType::STRUCTURE => 50,
                SymbolType::TRAIT => 40,
            };
            definition_score += symbol_score;
            for keyword in query.get_keywords() {
                if def.get_symbol() == keyword.get_keyword() {
                    // Exact match, give extra points
                    definition_score += symbol_score;
                }
            }
        }

        // Sometimes definition scores can get really crazy, e.g. if there are
        // a billion instances of a variable being defined over and over. Limit at 100.
        score += std::cmp::min(definition_score, 100) as f32;

        candidate.set_score(score);
    }

    fn fullscore(&self, query: &Query, candidate: &mut Candidate) {
        if candidate.get_is_ugly() {
            candidate.set_score(candidate.get_score() / 10.0);
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
            let n = find_max_span_window(candidate.get_spans());
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

fn find_max_span_window(spans: &[Span]) -> usize {
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
            let offset = if SNIPPET_LENGTH > (max - min) {
                (SNIPPET_LENGTH - (max - min)) / 2
            } else {
                0
            };

            if min > offset {
                max_window_start = min - offset;
            } else {
                max_window_start = 0;
            }
        }
    }

    max_window_start
}

fn extract_spans(re: &aho_corasick::AhoCorasick, line: &str) -> Vec<Span> {
    let mut output = Vec::new();
    if let Some(m) = re.find(line) {
        let mut s = Span::new();
        s.set_offset(m.start() as u64);
        s.set_length((m.end() - m.start()) as u64);
        output.push(s);
    }
    output
}

fn update_mask(mask: u32, index: usize) -> u32 {
    mask | (1 << index)
}
