#[macro_use]
extern crate flags;

fn main() {
    let overwrite = define_flag!("overwrite", false, "whether to overwrite the provided file");
    let args = parse_flags!(overwrite);

    let filename = match args.iter().next() {
        Some(f) => f,
        None => {
            eprintln!("must provide a filename!");
            std::process::exit(1);
        }
    };

    let content = std::fs::read_to_string(filename).unwrap();
    let parsed = ccl::get_ast_or_panic(&content);
    println!("{}", ccl::format(parsed, &content));
}
