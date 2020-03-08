#[macro_use]
extern crate flags;

use indexer_lib::{AggregateKeywordsFn, ExtractKeywordsFn};
use plume::{EmitFn, PTable, Stream, StreamingIterator, KV};

fn fail(message: &str) -> ! {
    eprintln!("{}", message);
    std::process::exit(1);
}

fn main() {
    let input_dir = define_flag!("input_dir", String::new(), "The directory to read from");
    let input_sstable = define_flag!(
        "input_sstable",
        String::new(),
        "The code sstable to read from. If provided, won't generate one"
    );
    let output_dir = define_flag!(
        "output_dir",
        String::new(),
        "The directory to write the index to"
    );

    parse_flags!(input_dir, output_dir, input_sstable);

    if output_dir.path().is_empty() {
        fail("You must specify an --output to write to!");
    }

    let starting_dir = if input_dir.path().is_empty() {
        std::env::current_dir().unwrap()
    } else {
        input_dir.path().into()
    };

    // Extract the codebase into a code sstable
    let code_sstable = if input_sstable.path().is_empty() {
        let code_sstable = format!("{}/code.sstable", output_dir.path());
        extract_lib::extract_code(&starting_dir, &code_sstable);
        code_sstable
    } else {
        input_sstable.path()
    };

    // Process the codebase into a keyword map
    let code = PTable::from_sstable(&code_sstable);
    let keywords = code.par_do(ExtractKeywordsFn {});
    let mut index = keywords.group_by_key_and_par_do(AggregateKeywordsFn {});

    let keywords_sstable = format!("{}/keywords.sstable", output_dir.path());
    index.write_to_sstable(&keywords_sstable);

    plume::run();
}
