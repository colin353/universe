use plume::{EmitFn, PCollection, Primitive, Stream, StreamingIterator, KV};
use search_proto_rust::*;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct ExtractKeywordsFn {}
impl plume::DoFn for ExtractKeywordsFn {
    type Input = File;
    type Output = KV<String, ExtractedKeyword>;

    fn do_it(&self, input: &File, emit: &mut dyn EmitFn<Self::Output>) {
        for keyword in language_specific::extract_keywords(input) {
            let mut normalized_keyword = search_utils::normalize_keyword(keyword.get_keyword());
            if normalized_keyword.len() > 4 {
                emit.emit(KV::new(normalized_keyword, keyword));
            }
        }
    }
}

pub struct AggregateKeywordsFn {}
impl plume::DoStreamFn for AggregateKeywordsFn {
    type Input = ExtractedKeyword;
    type Output = KV<String, ExtractedKeyword>;
    fn do_it(
        &self,
        key: &str,
        values: &mut Stream<ExtractedKeyword>,
        emit: &mut dyn EmitFn<Self::Output>,
    ) {
        let mut keywords = std::collections::HashMap::new();
        while let Some(kw) = values.next() {
            let mut count = keywords.entry(kw.get_keyword().to_string()).or_insert(0);
            *count += kw.get_occurrences();
        }

        for (key, occurrences) in keywords.into_iter() {
            let mut output = ExtractedKeyword::new();
            output.set_keyword(key.clone());
            output.set_occurrences(occurrences);
            emit.emit(KV::new(search_utils::normalize_keyword(&key), output));
        }
    }
}

pub struct ExtractDefinitionsFn {}
impl plume::DoFn for ExtractDefinitionsFn {
    type Input = File;
    type Output = KV<String, SymbolDefinition>;

    fn do_it(&self, input: &File, emit: &mut dyn EmitFn<Self::Output>) {
        for definition in language_specific::extract_definitions(input) {
            let mut normalized_symbol = search_utils::normalize_keyword(definition.get_symbol());
            emit.emit(KV::new(normalized_symbol, definition));
        }
    }
}

pub struct AggregateDefinitionsFn {}
impl plume::DoStreamFn for AggregateDefinitionsFn {
    type Input = SymbolDefinition;
    type Output = KV<String, DefinitionMatches>;
    fn do_it(
        &self,
        key: &str,
        values: &mut Stream<SymbolDefinition>,
        emit: &mut dyn EmitFn<Self::Output>,
    ) {
        let mut matches = DefinitionMatches::new();
        while let Some(m) = values.next() {
            matches.mut_matches().push(m.clone());
        }

        if matches.get_matches().len() > 0 {
            emit.emit(KV::new(key.to_owned(), matches));
        }
    }
}

pub struct ProcessFilesFn {}
impl plume::DoFn for ProcessFilesFn {
    type Input = File;
    type Output = KV<String, File>;
    fn do_it(&self, input: &File, emit: &mut dyn EmitFn<Self::Output>) {
        let mut file = input.clone();
        file.set_file_type(language_specific::get_filetype(file.get_filename()));

        // Add any other language-specific file annotations
        language_specific::annotate_file(&mut file);

        // Some machine-generated files have insanely long lines. Usually humans
        // don't want to read files like that.
        let lines = file.get_content().lines().count();
        let chars = file.get_content().len();
        if chars > 200 * lines {
            file.set_is_ugly(true);
        }

        emit.emit(KV::new(file.get_filename().to_owned(), file));
    }
}

pub struct ExtractCandidatesFn {}
impl plume::DoFn for ExtractCandidatesFn {
    type Input = File;
    type Output = KV<String, File>;
    fn do_it(&self, input: &File, emit: &mut dyn EmitFn<Self::Output>) {
        let mut file = input.clone();
        file.set_file_type(language_specific::get_filetype(file.get_filename()));

        // Add any other language-specific file annotations
        language_specific::annotate_file(&mut file);

        // Some machine-generated files have insanely long lines. Usually humans
        // don't want to read files like that.
        let lines = file.get_content().lines().count();
        let chars = file.get_content().len();
        if chars > 200 * lines {
            file.set_is_ugly(true);
        }

        emit.emit(KV::new(
            search_utils::hash_filename(file.get_filename()).to_string(),
            file,
        ));
    }
}

pub struct ExtractTrigramsFn {}
impl plume::DoFn for ExtractTrigramsFn {
    type Input = File;
    type Output = KV<String, Primitive<u64>>;
    fn do_it(&self, input: &File, emit: &mut dyn EmitFn<Self::Output>) {
        let file = input;
        let file_id = Primitive::from(search_utils::hash_filename(file.get_filename()));

        // Only extract trigrams for files < 1MB or else it basically matches
        // everything and is useless
        if file.get_content().len() > 1_000_000 {
            println!("skipped `{}`, too big", file.get_filename());
            return;
        }

        let mut collected_trigrams = std::collections::HashSet::new();
        for line in file.get_content().lines() {
            for trigram in search_utils::trigrams(&line.to_lowercase()) {
                collected_trigrams.insert(trigram.to_string());
            }
        }
        for trigram in collected_trigrams.into_iter() {
            emit.emit(KV::new(trigram, file_id));
        }
    }
}

pub struct AggregateTrigramsFn {}
impl plume::DoStreamFn for AggregateTrigramsFn {
    type Input = Primitive<u64>;
    type Output = KV<String, KeywordMatches>;
    fn do_it(
        &self,
        key: &str,
        values: &mut Stream<Primitive<u64>>,
        emit: &mut dyn EmitFn<Self::Output>,
    ) {
        let mut matches = KeywordMatches::new();
        while let Some(m) = values.next() {
            matches.mut_matches().push(**m);
        }

        if matches.get_matches().len() > 0 {
            emit.emit(KV::new(key.to_owned(), matches));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kw_matches(file_ids: &[u64]) -> KeywordMatches {
        let mut out = KeywordMatches::new();
        for id in file_ids {
            out.mut_matches().push(*id);
        }
        out
    }

    fn df(filename: &str, symbol: &str, line: u32) -> SymbolDefinition {
        let mut s = SymbolDefinition::new();
        s.set_filename(filename.to_owned());
        s.set_symbol(symbol.to_owned());
        s.set_line_number(line);
        s.set_symbol_type(SymbolType::VARIABLE);
        s
    }

    #[test]
    fn test_extract_trigams() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let mut dk = File::new();
        dk.set_filename("dk.rs".into());
        dk.set_content("let donkey_kong = 5;".into());

        let code = PCollection::from_vec(vec![dk]);

        let trigrams = code.par_do(ExtractTrigramsFn {});
        let mut index = trigrams.group_by_key_and_par_do(AggregateTrigramsFn {});
        index.write_to_vec();

        plume::run();

        let output = index.into_vec();
        assert_eq!(output.len(), 18);

        let m = kw_matches(&[1844087588747791261]);
        assert_eq!(output.as_ref()[0], KV::new(String::from(" 5;"), m.clone()));
        assert_eq!(output.as_ref()[1], KV::new(String::from(" = "), m.clone()));
        assert_eq!(output.as_ref()[2], KV::new(String::from(" do"), m.clone()));
        assert_eq!(output.as_ref()[3], KV::new(String::from("= 5"), m.clone()));
    }

    #[test]
    fn test_extract_definitions() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let mut dk = File::new();
        dk.set_filename("dk.rs".into());
        dk.set_content("let donkey_kong = 5;".into());

        let mut mario = File::new();
        mario.set_filename("mario.rs".into());
        mario.set_content("let mario = donkey_kong * 5;".into());

        let code = PCollection::from_vec(vec![dk, mario]);

        let definitions = code.par_do(ExtractDefinitionsFn {});
        let mut index = definitions.group_by_key_and_par_do(AggregateDefinitionsFn {});
        index.write_to_vec();

        plume::run();

        let output = index.into_vec();
        assert_eq!(output.len(), 2);

        let mut m = DefinitionMatches::new();
        m.mut_matches().push(df("dk.rs", "donkey_kong", 0));

        assert_eq!(output.as_ref()[0], KV::new(String::from("donkeykong"), m));

        let mut m = DefinitionMatches::new();
        m.mut_matches().push(df("mario.rs", "mario", 0));

        assert_eq!(output.as_ref()[1], KV::new(String::from("mario"), m));
    }
}
