#[macro_use]
extern crate lazy_static;

use search_grpc_rust::*;
use sstable2::{SSTableReader, SpecdSSTableReader};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const CANDIDATES_TO_RETURN: usize = 25;
const CANDIDATES_TO_EXPAND: usize = 100;
const MAX_LINE_LENGTH: usize = 144;
const SNIPPET_LENGTH: usize = 7;
const SUGGESTION_LIMIT: usize = 10;

lazy_static! {
    static ref KEYWORDS_RE: regex::Regex = { regex::Regex::new(r#"("(.*?)")|([^\s]+)"#).unwrap() };
    static ref DEFINITION_RE: regex::Regex = { regex::Regex::new(r"def:(\w+)").unwrap() };
}

pub struct Searcher {
    code: SSTableReader<File>,
    candidates: Arc<SSTableReader<File>>,
    definitions: SSTableReader<DefinitionMatches>,
    keywords: SSTableReader<ExtractedKeyword>,
    trigrams: SSTableReader<KeywordMatches>,

    files: HashMap<u64, File>,

    // Configuration options
    pub candidates_to_return: usize,
    pub candidates_to_expand: usize,
}

impl Searcher {
    pub fn new(base_dir: &str) -> Self {
        let mut code =
            SSTableReader::from_filename(&format!("{}/files.sstable", base_dir)).unwrap();
        let mut definitions =
            SSTableReader::from_filename(&format!("{}/definitions.sstable", base_dir)).unwrap();
        let mut candidates =
            SSTableReader::from_filename(&format!("{}/candidates.sstable", base_dir)).unwrap();
        let mut trigrams =
            SSTableReader::from_filename(&format!("{}/trigrams.sstable", base_dir)).unwrap();
        let mut keywords =
            SSTableReader::from_filename(&format!("{}/keywords.sstable", base_dir)).unwrap();

        let mut files = HashMap::<u64, File>::new();
        for (key, file) in &mut candidates {
            files.insert(key.parse::<u64>().unwrap(), file);
        }

        Self {
            code,
            definitions,
            candidates: Arc::new(candidates),
            trigrams,
            keywords,
            candidates_to_return: CANDIDATES_TO_RETURN,
            candidates_to_expand: CANDIDATES_TO_EXPAND,
            files: files,
        }
    }

    pub fn suggest(&self, keyword: &str) -> SuggestResponse {
        let query = self.parse_query(keyword);

        // Extract the common query prefix
        let mut prefix = query
            .get_keywords()
            .iter()
            .take(query.get_keywords().len() - 1)
            .map(|x| render_query_keyword(x))
            .collect::<Vec<_>>()
            .join(" ");

        if prefix.len() > 0 {
            prefix.push(' ');
        }

        // Only provide suggestions for the LAST keyword in the query
        let last_keyword = match query.get_keywords().iter().last() {
            Some(x) => x,
            None => return SuggestResponse::new(),
        };

        let keyword = search_utils::normalize_keyword(last_keyword.get_keyword());

        let specd_reader = SpecdSSTableReader::from_reader(&self.keywords, &keyword);
        let mut count = 0;
        let mut suggestions = Vec::new();
        for (_, keyword) in specd_reader {
            suggestions.push(keyword);
            count += 1;
            if count > SUGGESTION_LIMIT * 100 {
                break;
            }
        }

        suggestions.sort_by_key(|x| std::u64::MAX - x.get_occurrences());

        let mut output = SuggestResponse::new();
        let mut expanded_keyword = last_keyword.clone();
        for suggestion in suggestions
            .into_iter()
            .take(SUGGESTION_LIMIT)
            .map(|mut x| x.take_keyword())
        {
            expanded_keyword.set_keyword(suggestion);

            output.mut_suggestions().push(format!(
                "{}{}",
                prefix,
                render_query_keyword(&expanded_keyword)
            ));
        }
        output
    }

    pub fn search(&self, keywords: &str) -> SearchResponse {
        let mut response = SearchResponse::new();
        let query = self.parse_query(keywords);
        let start = std::time::Instant::now();
        let mut candidates = self.get_candidates(&query);
        self.initial_rank(&query, &mut candidates);
        self.extract_metadata(&mut response, &candidates);
        self.cutoff(&mut candidates, self.candidates_to_expand);
        self.expand_candidates(&query, &mut candidates);
        self.final_rank(&query, &mut candidates);
        self.cutoff(&mut candidates, self.candidates_to_return);

        #[cfg(debug_scoring)]
        self.print_debug_scoring(&query, &candidates);

        self.render_results(&candidates);
        println!("total search: {} ms", start.elapsed().as_millis());

        *response.mut_candidates() = protobuf::RepeatedField::from_vec(candidates);
        response
    }

    pub fn get_document(&self, filename: &str) -> Option<File> {
        self.code.get(filename).unwrap()
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

            // Language search
            if keyword.get_keyword().starts_with("lang:") {
                keyword.set_keyword(keyword.get_keyword()[5..].to_owned());
                keyword.set_is_language(true);
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
            .filter(|(_, k)| !k.get_is_prefix() && !k.get_is_language())
        {
            let mut matches = match self
                .definitions
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

    fn print_debug_scoring(&self, query: &Query, candidates: &[Candidate]) {
        println!("\n\nScoring report:");

        for candidate in candidates {
            debug_scoring("\n---------------------------------------------------------------------------------");
            debug_scoring(format!("candidate: {}", candidate.get_filename()));
            debug_scoring(
                "---------------------------------------------------------------------------------",
            );
            let mut c = candidate.clone();
            self.score(query, &mut c);
            self.fullscore(query, &mut c);
            debug_scoring(format!("final score: {}", c.get_score()));
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
            let mut border_match_mask = 0;
            let mut complete_match_mask = 0;
            let mut num_matches = 0;
            let mut matched_in_order = true;
            let mut filename_coverage = 0;

            let language = format!("{:?}", file.get_file_type());

            for (index, (keyword, re)) in keyword_matchers.iter().enumerate() {
                if keyword.get_is_language() && keyword.get_keyword().to_uppercase() == language {
                    match_mask = update_mask(match_mask, index);
                    continue;
                }

                if keyword.get_is_definition() || keyword.get_is_language() {
                    continue;
                }

                if let Some(m) = re.find(filename) {
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

                    // Keep track of whether the keyword matches were interior, border, or complete
                    // matches.
                    let mut keyword_matches_border = false;
                    if m.start() == 0
                        || !is_valid_variable_char(filename.chars().nth(m.start() - 1).unwrap())
                    {
                        border_match_mask = update_mask(border_match_mask, index);
                    }

                    if let Some(ch) = filename.chars().nth(m.end()) {
                        if !is_valid_variable_char(ch) {
                            if keyword_matches_border {
                                complete_match_mask = update_mask(complete_match_mask, index);
                            } else {
                                border_match_mask = update_mask(border_match_mask, index);
                            }
                        }
                    } else {
                        border_match_mask = update_mask(border_match_mask, index);
                    }
                }
            }

            if let Some(m) = query_matcher.find(&search_utils::normalize_keyword(filename)) {
                query_match = true;

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
                c.set_page_rank(file.get_page_rank());
                c.set_filename(filename.to_owned());
                c.set_keyword_matched_filename(true);
                c.set_query_in_filename(query_match);
                c.set_exactly_matched_filename(exact_match);
                c.set_filename_match_position(match_position as u32);
                c.set_filename_query_matches(num_matches);
                if num_matches > 1 {
                    c.set_filename_keywords_matched_in_order(matched_in_order);
                }
                c.set_keyword_definite_match_mask(c.get_keyword_definite_match_mask() | match_mask);
                c.set_keyword_border_match_mask(
                    c.get_keyword_border_match_mask() | border_match_mask,
                );
                c.set_keyword_complete_match_mask(
                    c.get_keyword_complete_match_mask() | complete_match_mask,
                );
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

        for (index, keyword) in
            query.get_keywords().iter().enumerate().filter(|(_, k)| {
                !k.get_is_definition() && !k.get_is_prefix() && !k.get_is_language()
            })
        {
            // We use a trigram index. If this keyword has fewer than 3 chars, just assume
            // any candidate might match it.
            if keyword.get_keyword().len() < 3 {
                short_keyword_mask = update_mask(short_keyword_mask, index);
            }

            let mut or_set: Option<HashSet<u64>> = None;
            for trigram in search_utils::trigrams(&keyword.get_keyword().to_lowercase()) {
                let mut matches = HashSet::new();

                let mut results = match self.trigrams.get(&trigram).unwrap() {
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

        let pool = pool::ThreadPool::new(8);

        for (file_id, candidate) in candidates.into_iter() {
            let matchers = keyword_matchers.clone();
            let reader = self.candidates.clone();
            pool.execute(move || {
                Self::finalize_keyword_matches_for_candidate(&reader, file_id, candidate, matchers)
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
        reader: &SSTableReader<File>,
        file_id: u64,
        mut candidate: Candidate,
        keyword_matchers: Vec<(QueryKeyword, aho_corasick::AhoCorasick)>,
    ) -> Candidate {
        let start = std::time::Instant::now();
        let mut file = { reader.get(&file_id.to_string()).unwrap().unwrap() };

        candidate.set_is_test(file.get_is_test());
        candidate.set_page_rank(file.get_page_rank());
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

                let mut border_match = false;
                let mut complete_match = false;

                for mut span in s.into_iter() {
                    span.set_line(line_number as u64);

                    let mut span_is_border_match = false;
                    if span.get_offset() >= 1
                        && !is_valid_variable_char(
                            line.chars()
                                .nth((span.get_offset() - 1) as usize)
                                .unwrap_or(' '),
                        )
                    {
                        span_is_border_match = true;
                    }
                    if let Some(c) = line
                        .chars()
                        .nth((span.get_offset() + span.get_length() + 1) as usize)
                    {
                        if !is_valid_variable_char(c) {
                            if span_is_border_match {
                                complete_match = true;
                            } else {
                                border_match = true;
                            }
                        }
                    }
                    border_match |= span_is_border_match;

                    candidate.mut_spans().push(span);
                }

                let k = matched_keywords.entry(index).or_insert_with(|| {
                    let mut e = ExtractedKeyword::new();
                    e.set_keyword(keyword.get_keyword().to_owned());
                    e
                });
                k.set_occurrences(k.get_occurrences() + 1);
                k.set_border_match(border_match);
                k.set_complete_match(complete_match);

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
            candidate.set_keyword_definite_match_mask(update_mask(
                candidate.get_keyword_definite_match_mask(),
                index,
            ));

            if (k.get_border_match()) {
                candidate.set_keyword_border_match_mask(update_mask(
                    candidate.get_keyword_border_match_mask(),
                    index,
                ))
            }
            if (k.get_complete_match()) {
                candidate.set_keyword_complete_match_mask(update_mask(
                    candidate.get_keyword_complete_match_mask(),
                    index,
                ))
            }
            candidate.mut_matched_keywords().push(k);
        }

        candidate
    }

    fn extract_metadata(&self, response: &mut SearchResponse, candidates: &Vec<Candidate>) {
        let mut languages = HashMap::new();
        let mut prefixes = HashMap::new();
        for candidate in candidates {
            if candidate.get_file_type() != FileType::UNKNOWN {
                *languages.entry(candidate.get_file_type()).or_insert(0) += 1;
            }

            if let Some(p) = candidate.get_filename().split("/").next() {
                *prefixes.entry(p).or_insert(0) += 1;
            }
        }

        let mut languages: Vec<_> = languages.iter().collect();
        languages.sort_by_key(|(k, v)| *v);
        for language in languages.iter().rev().map(|(k, v)| k).take(5) {
            response
                .mut_languages()
                .push(format!("{:?}", language).to_lowercase());
        }
        if response.mut_languages().len() == 1 {
            response.mut_languages().clear();
        }

        let mut prefixes: Vec<_> = prefixes.iter().collect();
        prefixes.sort_by_key(|(k, v)| *v);
        for prefix in prefixes.iter().rev().map(|(k, v)| k).take(5) {
            response.mut_prefixes().push(prefix.to_string());
        }
        if response.mut_prefixes().len() == 1 {
            response.mut_prefixes().clear();
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
        let mut score = 0.0;

        // Keyword match scoring
        for kw in candidate.get_matched_keywords() {
            let mut points = 10.0;
            points += std::cmp::min(kw.get_occurrences(), 10) as f32;
            score += points;
            debug_score(format!("matched `{}`", kw.get_keyword()), points);

            // Penalty for non-exact keyword match
            if kw.get_normalized() {
                score -= 3.0;
                debug_score("keyword was normalized", -3);
            }
        }

        // Filename match scoring
        if candidate.get_keyword_matched_filename() {
            score += 10.0;
            debug_score("keyword matched filename", 10);
            score += 100.0 * candidate.get_filename_match_coverage();
            if candidate.get_filename_match_coverage() > 0.0 {
                debug_score(
                    "filename match coverage",
                    100.0 * candidate.get_filename_match_coverage(),
                );
            }
            score += 10.0 * candidate.get_filename_query_matches() as f32;
            if candidate.get_filename_query_matches() > 0 {
                debug_score(
                    "filename query matches",
                    10.0 * candidate.get_filename_query_matches() as f32,
                );
            }
        }

        let points = 50.0 * candidate.get_filename_match_position() as f32
            / candidate.get_filename().len() as f32;

        if points > 0.0 {
            debug_score("filename match position", points);
        }
        score += points;

        if candidate.get_query_in_filename() {
            if candidate.get_filename_keywords_matched_in_order() {
                let points = 20.0 * candidate.get_filename_query_matches() as f32;
                debug_score("filename match is ordered", points);
                score += points;
            }
        }

        score += 10.0 * candidate.get_page_rank().log2();
        debug_score("pagerank: ", 10.0 * candidate.get_page_rank().log2());

        if candidate.get_exactly_matched_filename() {
            score += 40.0;
            debug_score("exact filename match", 40);
        }

        if candidate.get_filename().contains("/migrations/") {
            debug_score("contains migrations", -score / 2.0);
            score /= 2.0;
        }

        if candidate.get_filename().starts_with("third_party") {
            debug_score("contains third_party", -score / 3.0);
            score /= 3.0;
        }

        // Definition scoring
        let mut definition_score = 0;
        let mut variable_definitions = HashSet::new();
        for def in candidate.get_matched_definitions() {
            // Skip duplicate variable definitions, since those can be spammy
            if def.get_symbol_type() == SymbolType::VARIABLE
                && variable_definitions.contains(def.get_symbol())
            {
                continue;
            } else if def.get_symbol_type() == SymbolType::VARIABLE {
                variable_definitions.insert(def.get_symbol());
            }

            let symbol_score = score_definition_type(def.get_symbol_type());
            definition_score += symbol_score;
            debug_score(
                format!("contains definition `{}`", def.get_symbol()),
                symbol_score,
            );
            for keyword in query.get_keywords() {
                if def.get_symbol() == keyword.get_keyword() {
                    // Exact match, give extra points
                    definition_score += symbol_score;
                    debug_score("exact definition match", symbol_score);
                }
            }
        }

        // Sometimes definition scores can get really crazy, e.g. if there are
        // a billion instances of a variable being defined over and over. Limit at 100.
        if definition_score > 100 {
            debug_score("cap definition score", -(definition_score as f32 - 100.0));
        }
        score += std::cmp::min(definition_score, 100) as f32;

        let points = 10.0 * candidate.get_keyword_complete_match_mask().count_ones() as f32;
        debug_score("complete match mask", points);
        score += points;

        let points = 5.0 * candidate.get_keyword_border_match_mask().count_ones() as f32;
        debug_score("border match mask", points);
        score += points;

        let target_match_mask = (1 << (query.get_keywords().len())) - 1;
        if candidate.get_keyword_border_match_mask() != target_match_mask {
            debug_score("incomplete border match mask", score / 2.0);
            score /= 2.0;
        }

        score -= 0.25 * candidate.get_filename().len() as f32;
        debug_score(
            "long filename discount",
            -0.25 * (candidate.get_filename().len() as f32),
        );

        candidate.set_score(score);
    }

    fn fullscore(&self, query: &Query, candidate: &mut Candidate) {
        let mut score = candidate.get_score();

        if candidate.get_is_ugly() {
            debug_score("is ugly", -(9.0 * score / 10.0));
            score /= 10.0;
        }

        if candidate.get_is_test() {
            debug_score("is a test", -(score / 2.0));
            score /= 2.0;
        }

        candidate.set_score(score);
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
                candidate.get_filename(),
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
            // Need to find the MOST important definition to highlight in the snippet
            let mut line_options: Vec<_> = candidate
                .get_matched_definitions()
                .iter()
                .map(|x| (x.get_symbol_type(), x.get_line_number()))
                .collect();
            line_options.sort_by_key(|x| score_definition_type(x.0));

            let mut line_number = line_options.last().unwrap().1 as usize;
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
            started = true;
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

#[cfg(debug_scoring)]
fn debug_scoring<T: std::fmt::Display>(input: T) {
    println!("{}", input);
}

#[cfg(debug_scoring)]
fn debug_score<T: std::fmt::Display, U: std::fmt::Display>(input: T, diff: U) {
    let mut out = format!("{}", input);
    let spaces = 60 - out.len();
    for _ in (0..spaces) {
        out.push(' ');
    }
    out += &format!("{}", diff);
    println!("{}", out);
}

#[cfg(not(debug_scoring))]
fn debug_score<T: std::fmt::Display, U: std::fmt::Display>(input: T, diff: U) {}

#[cfg(not(debug_scoring))]
fn debug_scoring<T: std::fmt::Display>(_: T) {}

fn render_query_keyword(kw: &QueryKeyword) -> String {
    let mut prefix = "";
    if kw.get_is_definition() {
        prefix = "def:";
    } else if kw.get_is_prefix() {
        prefix = "in:";
    } else if kw.get_is_language() {
        prefix = "lang:";
    }

    format!("{}{}", prefix, kw.get_keyword())
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

fn is_valid_variable_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn update_mask(mask: u32, index: usize) -> u32 {
    mask | (1 << index)
}

fn score_definition_type(def: SymbolType) -> u64 {
    match def {
        SymbolType::VARIABLE => 5,
        SymbolType::FUNCTION => 40,
        SymbolType::STRUCTURE => 80,
        SymbolType::TRAIT => 40,
    }
}
