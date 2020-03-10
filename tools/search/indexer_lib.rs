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
}
