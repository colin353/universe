#[macro_use]
extern crate flags;

fn fail(message: &str) -> ! {
    eprintln!("{}", message);
    std::process::exit(1);
}

fn main() {
    let root_dir = define_flag!(
        "path",
        String::new(),
        "The root dir to extract from. If empty, use CWD"
    );
    let output = define_flag!("output", String::new(), "The filename to write to");

    parse_flags!(root_dir, output);

    if output.path().is_empty() {
        fail("You must specify an --output to write to!");
    }

    let starting_dir = if root_dir.path().is_empty() {
        std::env::current_dir().unwrap()
    } else {
        root_dir.path().into()
    };

    extract_lib::extract_code(&starting_dir, &output.path());
}
