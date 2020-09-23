#[macro_use]
extern crate flags;

use indexer_lib::{
    AggregateDefinitionsFn, AggregateKeywordsFn, AggregateTrigramsFn, ExtractCandidatesFn,
    ExtractDefinitionsFn, ExtractImportsFn, ExtractKeywordsFn, ExtractTrigramsFn, ImportsJoinFn,
    ProcessFilesFn,
};
use plume::{EmitFn, PCollection, Stream, StreamingIterator, KV};

fn fail(message: &str) -> ! {
    eprintln!("{}", message);
    std::process::exit(1);
}

fn main() {
    let input_dir = define_flag!("input_dir", String::new(), "The directory to read from");
    let input_recordio = define_flag!(
        "input_recordio",
        String::new(),
        "The code recordio to read from. If provided, won't generate one"
    );
    let output_dir = define_flag!(
        "output_dir",
        String::new(),
        "The directory to write the index to"
    );

    parse_flags!(input_dir, output_dir, input_recordio);

    if output_dir.path().is_empty() {
        fail("You must specify an --output to write to!");
    }

    let starting_dir = if input_dir.path().is_empty() {
        std::env::current_dir().unwrap()
    } else {
        input_dir.path().into()
    };

    // Extract the codebase into a code sstable
    let code_recordio = if input_recordio.path().is_empty() {
        let code_recordio = format!("{}/code.recordio", output_dir.path());
        extract_lib::extract_code(&starting_dir, &code_recordio);
        code_recordio
    } else {
        input_recordio.path()
    };

    // Interpret filetypes and process file data
    let code = PCollection::from_recordio(&code_recordio);
    let files = code.par_do(ProcessFilesFn {});

    // Extract and join imports into files
    let imports = code.par_do_side_input(ExtractImportsFn::new(), code.clone());
    let mut annotated_files = imports.join(files, ImportsJoinFn {});

    let files_sstable = format!("{}/files.sstable", output_dir.path());
    annotated_files.write_to_sstable(&files_sstable);

    // Extract file info by file_id
    let mut candidates = annotated_files.par_do(ExtractCandidatesFn {});
    let candidates_sstable = format!("{}/candidates.sstable", output_dir.path());
    candidates.write_to_sstable(&candidates_sstable);

    // Extract trigrams
    let trigrams = code.par_do(ExtractTrigramsFn {});
    let mut trigram_matches = trigrams.group_by_key_and_par_do(AggregateTrigramsFn {});
    let trigrams_sstable = format!("{}/trigrams.sstable", output_dir.path());
    trigram_matches.write_to_sstable(&trigrams_sstable);

    // Extract definitions
    let keywords = code.par_do(ExtractDefinitionsFn {});
    let mut index = keywords.group_by_key_and_par_do(AggregateDefinitionsFn {});
    let definitions_sstable = format!("{}/definitions.sstable", output_dir.path());
    index.write_to_sstable(&definitions_sstable);

    // Extract keywords
    let keywords = code.par_do(ExtractKeywordsFn {});
    let mut extracted_keywords = keywords.group_by_key_and_par_do(AggregateKeywordsFn {});
    let keywords_sstable = format!("{}/keywords.sstable", output_dir.path());
    extracted_keywords.write_to_sstable(&keywords_sstable);

    plume::run();
}
