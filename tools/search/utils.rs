use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn hash_filename(filename: &str) -> u64 {
    let mut s = DefaultHasher::new();
    filename.hash(&mut s);
    s.finish()
}

pub fn trigrams<'a>(src: &'a str) -> impl Iterator<Item = &'a str> {
    src.char_indices().flat_map(move |(from, _)| {
        src[from..]
            .char_indices()
            .skip(2)
            .next()
            .map(|(to, c)| &src[from..from + to + c.len_utf8()])
    })
}

pub fn normalize_keyword(keyword: &str) -> String {
    let mut normalized_keyword = keyword.to_lowercase();
    normalized_keyword.retain(|c| c != '_' && c != '-');
    normalized_keyword
}
