use plume::{EmitFn, PTable, Stream, StreamingIterator, KV};
use search_proto_rust::*;

pub struct ExtractKeywordsFn {}
impl plume::DoFn for ExtractKeywordsFn {
    type Input = KV<String, File>;
    type Output = KV<String, KeywordMatch>;

    fn do_it(&self, input: &KV<String, File>, emit: &mut dyn EmitFn<Self::Output>) {
        for extracted_keyword in language_specific::extract_keywords(input.value()) {
            let mut doc = KeywordMatch::new();
            doc.set_filename(input.key().to_owned());
            doc.set_occurrences(extracted_keyword.get_occurrences());
            let keyword = extracted_keyword.get_keyword().to_owned();
            emit.emit(KV::new(keyword.clone(), doc.clone()));

            // Also create a normalized version, which is lowercase
            // and has _ and - chars stripped
            let mut normalized_keyword = keyword.to_lowercase();
            normalized_keyword.retain(|c| c != '_' && c != '-');
            if normalized_keyword != keyword {
                doc.set_normalized(true);
                emit.emit(KV::new(normalized_keyword, doc));
            }
        }
    }
}


pub struct AggregateKeywordsFn {}
impl plume::DoStreamFn for AggregateKeywordsFn {
    type Input = KeywordMatch;
    type Output = KV<String, KeywordMatches>;
    fn do_it(
        &self,
        key: &str,
        values: &mut Stream<KeywordMatch>,
        emit: &mut dyn EmitFn<Self::Output>,
    ) {
        let mut matches = KeywordMatches::new();
        while let Some(m) = values.next() {
            matches.mut_matches().push(m.clone());
        }

        if matches.get_matches().len() > 0 {
            emit.emit(KV::new(key.to_owned(), matches));
        }
    }
}

pub struct ExtractDefinitionsFn {}
impl plume::DoFn for ExtractDefinitionsFn {
    type Input = KV<String, File>;
    type Output = KV<String, SymbolDefinition>;

    fn do_it(&self, input: &KV<String, File>, emit: &mut dyn EmitFn<Self::Output>) {
        for definition in language_specific::extract_definitions(input.value()) {
            let symbol = definition.get_symbol().to_string();
            emit.emit(KV::new(symbol.clone(), definition.clone()));

            // Also create a normalized version, which is lowercase
            // and has _ and - chars stripped
            let mut normalized_symbol = definition.get_symbol().to_lowercase();
            normalized_symbol.retain(|c| c != '_' && c != '-');
            if normalized_symbol != symbol {
                emit.emit(KV::new(normalized_symbol, definition));
            }
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
    type Input = KV<String, File>;
    type Output = KV<String, File>;
    fn do_it(&self, input: &KV<String, File>, emit: &mut dyn EmitFn<Self::Output>) {
        let mut file = input.value().clone();
        file.set_file_type(language_specific::get_filetype(file.get_filename()));

        // Some machine-generated files have insanely long lines. Usually humans
        // don't want to read files like that.
        let lines = file.get_content().lines().count();
        let chars = file.get_content().len();
        if chars > 200 * lines {
            file.set_is_ugly(true);
        }

        emit.emit(KV::new(input.key().to_owned(), file));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kw(filename: &str, occ: u64, norm: bool) -> KeywordMatch {
        let mut m = KeywordMatch::new();
        m.set_filename(filename.to_owned());
        m.set_occurrences(occ);
        m.set_normalized(norm);
        m
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
    fn test_extract_keywords() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let mut dk = File::new();
        dk.set_filename("dk.txt".into());
        dk.set_content("let donkey_kong = 5;".into());

        let mut mario = File::new();
        mario.set_filename("mario.txt".into());
        mario.set_content("let mario = donkey_kong * 5;".into());

        let code = PTable::from_table(vec![
            KV::new(String::from("dk.txt"), dk),
            KV::new(String::from("mario.txt"), mario),
        ]);

        let keywords = code.par_do(ExtractKeywordsFn {});
        let mut index = keywords.group_by_key_and_par_do(AggregateKeywordsFn {});
        index.write_to_vec();

        plume::run();

        let output = index.into_vec();

        let mut m = KeywordMatches::new();
        m.mut_matches().push(kw("dk.txt", 1, false));
        m.mut_matches().push(kw("mario.txt", 1, false));

        assert_eq!(output.as_ref()[0], KV::new(String::from("donkey_kong"), m));

        let mut m = KeywordMatches::new();
        m.mut_matches().push(kw("dk.txt", 1, true));
        m.mut_matches().push(kw("mario.txt", 1, true));

        assert_eq!(output.as_ref()[1], KV::new(String::from("donkeykong"), m));

        let mut m = KeywordMatches::new();
        m.mut_matches().push(kw("mario.txt", 1, false));

        assert_eq!(output.as_ref()[2], KV::new(String::from("mario"), m));
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

        let code = PTable::from_table(vec![
            KV::new(String::from("dk.rs"), dk),
            KV::new(String::from("mario.rs"), mario),
        ]);

        let definitions = code.par_do(ExtractDefinitionsFn {});
        let mut index = definitions.group_by_key_and_par_do(AggregateDefinitionsFn {});
        index.write_to_vec();

        plume::run();

        let output = index.into_vec();
        assert_eq!(output.len(), 3);

        let mut m = DefinitionMatches::new();
        m.mut_matches().push(df("dk.rs", "donkey_kong", 0));

        assert_eq!(output.as_ref()[0], KV::new(String::from("donkey_kong"), m));

        let mut m = DefinitionMatches::new();
        m.mut_matches().push(df("dk.rs", "donkey_kong", 0));

        assert_eq!(output.as_ref()[1], KV::new(String::from("donkeykong"), m));

        let mut m = DefinitionMatches::new();
        m.mut_matches().push(df("mario.rs", "mario", 0));

        assert_eq!(output.as_ref()[2], KV::new(String::from("mario"), m));
    }
}
