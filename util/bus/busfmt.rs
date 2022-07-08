#[macro_use]
extern crate flags;

use std::io::Read;
fn main() {
    let overwrite = define_flag!("overwrite", false, "whether to overwrite the provided file");
    let args = parse_flags!(overwrite);
    let mut content = match args.iter().next() {
        Some(f) => std::fs::read_to_string(f).unwrap(),
        None => {
            // Read from stdin
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).unwrap();
            buf
        }
    };
    content.push('\n');

    let parsed = match parser::parse_ast(&content) {
        Ok(p) => p,
        Err(parser::BusError::ParseError(g)) => {
            eprintln!("{}", g.render(&content));
            std::process::exit(1);
        }
    };
    fmt::format(parsed, &content, &mut std::io::stdout()).unwrap();
}
