#[macro_use]
extern crate flags;

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

    indexer_lib::run_indexer(&code_recordio, &output_dir.path());
}
