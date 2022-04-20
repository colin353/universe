#[macro_use]
extern crate flags;

fn main() {
    let index_dir = define_flag!(
        "index_dir",
        String::new(),
        "The directory to find the search index"
    );

    let keywords = parse_flags!(index_dir);

    let s = search_lib::Searcher::new(&index_dir.path());
    s.search(&keywords.join(" "));
}
